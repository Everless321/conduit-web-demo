//! conduit-web-demo: a single-binary demo that pairs a web management UI
//! (servers + tokens) with a live conduit MCP endpoint, sharing one SQLite DB.
//! Manage servers/tokens in the browser, then point an MCP client at `/mcp`.

mod api;
mod policy;
mod privacy;
mod store;
mod ui;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    extract::FromRef,
    response::{Html, IntoResponse, Json},
    routing::{delete, get},
    Router,
};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use conduit_core::crypto::MasterKey;
use conduit_core::{AuditSink, Authorizer, ServerCatalog, TokenValidator};
use conduit_engine::{mcp_router, spawn_session_cleaner, AppState, RateLimiter};
use conduit_store_dibs::DibsStore;

use crate::policy::WhitelistPolicy;
use crate::privacy::PrivacyFilter;
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
    /// Allowlist regex for `exec` (repeatable; comma-separated via env). When
    /// set, a command must match one pattern AND survive the destructive
    /// blacklist. Unset = blacklist-only (every non-destructive command allowed).
    #[arg(long = "allow-command", env = "CONDUIT_ALLOW_COMMAND", value_delimiter = ',')]
    allow_command: Vec<String>,
    /// privacy-filter base URL (e.g. http://privacy-filter:8088). When set, SSH
    /// output is redacted through it before being returned. Unset = raw output.
    #[arg(long, env = "CONDUIT_PRIVACY_FILTER_URL")]
    privacy_filter_url: Option<String>,
    /// If privacy-filter is unreachable, return raw output instead of withholding
    /// it. Default (false) fails closed to avoid leaking unfiltered output.
    #[arg(long, env = "CONDUIT_PRIVACY_FAIL_OPEN", default_value_t = false)]
    privacy_fail_open: bool,
}

/// Shared state for the `/api` router. `FromRef` lets each handler extract just
/// the piece it needs (`State<Store>` or `State<Arc<WhitelistPolicy>>`).
#[derive(Clone)]
struct ApiState {
    store: Store,
    policy: Arc<WhitelistPolicy>,
}

impl FromRef<ApiState> for Store {
    fn from_ref(s: &ApiState) -> Store {
        s.store.clone()
    }
}

impl FromRef<ApiState> for Arc<WhitelistPolicy> {
    fn from_ref(s: &ApiState) -> Arc<WhitelistPolicy> {
        s.policy.clone()
    }
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

    // Allowlist lives in the DB (editable from the UI). Seed it from
    // --allow-command only on first run when the table is still empty.
    let mut rules = store.list_command_rules().await.context("load command rules")?;
    if rules.is_empty() && !cli.allow_command.is_empty() {
        store.set_command_rules(&cli.allow_command).await.context("seed command rules")?;
        rules = cli.allow_command.clone();
    }
    let policy = Arc::new(WhitelistPolicy::new(rules).context("policy")?);
    if policy.enforced() {
        let (p, _) = policy.snapshot();
        tracing::info!(patterns = p.len(), "command allowlist enforced");
    } else {
        tracing::warn!("allowlist empty; blacklist-only (every non-destructive command allowed)");
    }
    let authz: Arc<dyn Authorizer> = policy.clone();
    let validator: Arc<dyn TokenValidator> = dibs.clone();
    let catalog: Arc<dyn ServerCatalog> = dibs.clone();
    let audit: Arc<dyn AuditSink> = dibs.clone();

    let limiter = RateLimiter::new(cli.rate_per_minute);
    let mut app_state = AppState::new(catalog, authz, audit, limiter, cli.idle_timeout_secs);
    if let Some(url) = &cli.privacy_filter_url {
        tracing::info!(url = %url, fail_open = cli.privacy_fail_open, "output filter: privacy-filter enabled");
        app_state = app_state.with_output_filter(Arc::new(PrivacyFilter::new(
            url.clone(),
            cli.privacy_fail_open,
        )));
    }
    let engine_state = Arc::new(app_state);
    spawn_session_cleaner(engine_state.clone());

    // Management API, gated by the admin password. State carries both the
    // store and the live policy so handlers can extract either via FromRef.
    let api_state = ApiState { store, policy: policy.clone() };
    let api = Router::new()
        .route("/api/me", get(api::me))
        .route("/api/servers", get(api::list_servers).post(api::create_server))
        .route("/api/servers/:alias", delete(api::delete_server))
        .route("/api/audit", get(api::list_audit))
        .route("/api/policy", get(api::get_policy).put(api::set_policy))
        .route("/api/tokens", get(api::list_tokens).post(api::create_token))
        .route("/api/tokens/:id", delete(api::revoke_token))
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(admin_password),
            api::admin_auth,
        ))
        .with_state(api_state);

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
