//! conduit-web-demo: a single-binary demo that pairs a web management UI
//! (servers + tokens) with a live conduit MCP endpoint, sharing one SQLite DB.
//! Manage servers/tokens in the browser, then point an MCP client at `/mcp`.

mod api;
mod store;
mod ui;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    response::{Html, IntoResponse, Json},
    routing::{delete, get},
    Router,
};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use conduit_core::crypto::MasterKey;
use conduit_core::policy::CommandPolicy;
use conduit_core::{AuditSink, Authorizer, ServerCatalog, TokenValidator};
use conduit_engine::{mcp_router, spawn_session_cleaner, AppState, RateLimiter};
use conduit_store_dibs::DibsStore;

use crate::store::Store;

#[derive(Parser, Debug)]
#[command(name = "conduit-web-demo", version, about = "Conduit single-binary web demo")]
struct Cli {
    /// Web + MCP bind address. Localhost by default; expose explicitly.
    #[arg(long, default_value = "127.0.0.1:8088", env = "CONDUIT_WEB_BIND")]
    bind: String,
    #[arg(long, default_value = "./conduit-demo.db", env = "CONDUIT_DB")]
    db: String,
    /// 64-hex master key. If unset, a key file `<db>.key` is generated/reused.
    #[arg(long, env = "CONDUIT_MASTER_KEY")]
    master_key: Option<String>,
    /// Admin password for the management UI. If unset, a random one is printed.
    #[arg(long, env = "CONDUIT_ADMIN_PASSWORD")]
    admin_password: Option<String>,
    #[arg(long, default_value_t = 30, env = "CONDUIT_RATE_PER_MIN")]
    rate_per_minute: u32,
    #[arg(long, default_value_t = 1800, env = "CONDUIT_IDLE_TIMEOUT_SECS")]
    idle_timeout_secs: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    let cli = Cli::parse();

    let key = resolve_master_key(&cli)?;
    let admin_password = cli.admin_password.clone().unwrap_or_else(|| {
        let p = MasterKey::generate_hex()[..16].to_string();
        tracing::warn!(password = %p, "no --admin-password set; generated one for this run");
        p
    });

    // Demo write-layer (creates schema + seeds the implicit user).
    let store = Store::open(&cli.db, key.clone()).await.context("open store")?;
    // Engine read-layer over the SAME db, decrypting with the same key.
    let dibs = Arc::new(DibsStore::open(&cli.db, key).await.context("open engine store")?);

    let authz: Arc<dyn Authorizer> = Arc::new(CommandPolicy::new(vec![]).context("policy")?);
    let validator: Arc<dyn TokenValidator> = dibs.clone();
    let catalog: Arc<dyn ServerCatalog> = dibs.clone();
    let audit: Arc<dyn AuditSink> = dibs.clone();

    let limiter = RateLimiter::new(cli.rate_per_minute);
    let engine_state = Arc::new(AppState::new(catalog, authz, audit, limiter, cli.idle_timeout_secs));
    spawn_session_cleaner(engine_state.clone());

    // Management API, gated by the admin password.
    let api = Router::new()
        .route("/api/me", get(api::me))
        .route("/api/servers", get(api::list_servers).post(api::create_server))
        .route("/api/servers/:alias", delete(api::delete_server))
        .route("/api/tokens", get(api::list_tokens).post(api::create_token))
        .route("/api/tokens/:id", delete(api::revoke_token))
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(admin_password),
            api::admin_auth,
        ))
        .with_state(store);

    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .merge(api)
        .merge(mcp_router(engine_state, validator));

    let listener = tokio::net::TcpListener::bind(&cli.bind).await.context("bind")?;
    tracing::info!(addr = %cli.bind, "conduit-web-demo listening — UI at http://{}/  MCP at http://{}/mcp", cli.bind, cli.bind);
    axum::serve(listener, app).await.context("serve")?;
    Ok(())
}

/// Resolve the master key: explicit flag/env wins; otherwise reuse or create a
/// `<db>.key` file so the demo runs with zero config.
fn resolve_master_key(cli: &Cli) -> Result<MasterKey> {
    if let Some(hex) = &cli.master_key {
        return MasterKey::from_hex(hex).context("invalid CONDUIT_MASTER_KEY");
    }
    let key_path = format!("{}.key", cli.db);
    if let Ok(hex) = std::fs::read_to_string(&key_path) {
        return MasterKey::from_hex(hex.trim()).context("invalid key file");
    }
    let hex = MasterKey::generate_hex();
    std::fs::write(&key_path, &hex).with_context(|| format!("write key file {key_path}"))?;
    tracing::warn!(path = %key_path, "generated a new master key (keep this file safe; losing it makes stored secrets unrecoverable)");
    MasterKey::from_hex(&hex).context("invalid generated key")
}

async fn index() -> impl IntoResponse {
    Html(ui::INDEX_HTML)
}

async fn healthz() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
