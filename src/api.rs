//! JSON API for the management UI, plus the admin-password gate that protects
//! it. The MCP endpoint is mounted separately and uses its own bearer-token
//! validation, so this gate only covers `/api/*`.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::policy::WhitelistPolicy;
use crate::store::{ServerInput, Store};

type ApiResult = Result<Json<Value>, (StatusCode, String)>;

fn err(e: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

/// Require `Authorization: Bearer <admin-password>` on every `/api` request.
pub async fn admin_auth(
    State(password): State<Arc<String>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim);
    match presented {
        Some(p) if p == password.as_str() => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Lightweight authed ping used by the UI to validate the admin password on
/// login and on reload. Passes through `admin_auth`, so a 200 means authorized.
pub async fn me() -> ApiResult {
    Ok(Json(json!({ "ok": true })))
}

pub async fn list_servers(State(store): State<Store>) -> ApiResult {
    let servers = store.list_servers().await.map_err(err)?;
    Ok(Json(json!({ "servers": servers })))
}

pub async fn create_server(
    State(store): State<Store>,
    Json(input): Json<ServerInput>,
) -> ApiResult {
    store.create_server(&input).await.map_err(err)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_server(
    State(store): State<Store>,
    Path(alias): Path<String>,
) -> ApiResult {
    store.delete_server(&alias).await.map_err(err)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub server: Option<String>,
    pub session: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_audit(
    State(store): State<Store>,
    Query(q): Query<AuditQuery>,
) -> ApiResult {
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let rows = store
        .list_audit(q.server.as_deref(), q.session.as_deref(), limit)
        .await
        .map_err(err)?;
    Ok(Json(json!({ "audit": rows })))
}

// ---- command policy ------------------------------------------------------

fn policy_view(policy: &WhitelistPolicy) -> Value {
    let (patterns, enforced) = policy.snapshot();
    json!({ "patterns": patterns, "enforced": enforced })
}

pub async fn get_policy(State(policy): State<Arc<WhitelistPolicy>>) -> ApiResult {
    Ok(Json(policy_view(&policy)))
}

#[derive(Deserialize)]
pub struct PolicyInput {
    pub patterns: Vec<String>,
}

pub async fn set_policy(
    State(store): State<Store>,
    State(policy): State<Arc<WhitelistPolicy>>,
    Json(input): Json<PolicyInput>,
) -> ApiResult {
    let patterns: Vec<String> = input
        .patterns
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    // Validate regex (and swap the live set) before persisting.
    policy
        .reload(patterns.clone())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    store.set_command_rules(&patterns).await.map_err(err)?;
    Ok(Json(policy_view(&policy)))
}

pub async fn list_tokens(State(store): State<Store>) -> ApiResult {
    let tokens = store.list_tokens().await.map_err(err)?;
    Ok(Json(json!({ "tokens": tokens })))
}

#[derive(Deserialize)]
pub struct CreateTokenInput {
    pub label: String,
}

pub async fn create_token(
    State(store): State<Store>,
    Json(input): Json<CreateTokenInput>,
) -> ApiResult {
    let token = store.create_token(&input.label).await.map_err(err)?;
    // Returned exactly once — only the hash is persisted.
    Ok(Json(json!({ "token": token })))
}

pub async fn revoke_token(
    State(store): State<Store>,
    Path(id): Path<i64>,
) -> ApiResult {
    store.revoke_token(id).await.map_err(err)?;
    Ok(Json(json!({ "ok": true })))
}
