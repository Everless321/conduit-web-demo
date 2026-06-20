//! Write/read layer over the Dibs-compatible SQLite schema. `conduit-store-dibs`
//! only reads these tables; the demo owns their creation and all writes here,
//! using the same `MasterKey` ChaCha20 format and `sha256_hex` token hashing so
//! the live engine (`DibsStore`) can consume what the UI writes.
//!
//! Access model: a single implicit user (id = 1). Every created server is
//! granted to that user, so every issued token can reach every server.

use std::str::FromStr;

use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use conduit_core::crypto::{sha256_hex, MasterKey};

pub const DEFAULT_USER_ID: i64 = 1;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    active INTEGER NOT NULL DEFAULT 1,
    mcp_token_hash TEXT
);
CREATE TABLE IF NOT EXISTS servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    alias TEXT NOT NULL UNIQUE,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_kind TEXT NOT NULL,
    secret_enc BLOB NOT NULL,
    key_passphrase_enc BLOB,
    certificate_enc BLOB,
    jump_host_alias TEXT,
    description TEXT,
    tags TEXT
);
CREATE TABLE IF NOT EXISTS server_user_permissions (
    server_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    PRIMARY KEY (server_id, user_id)
);
CREATE TABLE IF NOT EXISTS mcp_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);
"#;

#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
    pub key: MasterKey,
}

impl Store {
    /// Open (creating if absent) the SQLite DB, build the schema, and seed the
    /// single implicit user. Returns a store usable by the API handlers.
    pub async fn open(path: &str, key: MasterKey) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
            .context("parse sqlite path")?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .context("open sqlite")?;
        sqlx::query(SCHEMA).execute(&pool).await.context("create schema")?;
        sqlx::query(
            "INSERT OR IGNORE INTO users(id, username, role, active) VALUES (?, 'demo', 'admin', 1)",
        )
        .bind(DEFAULT_USER_ID)
        .execute(&pool)
        .await
        .context("seed user")?;
        Ok(Self { pool, key })
    }

    // ---- servers ---------------------------------------------------------

    pub async fn list_servers(&self) -> Result<Vec<ServerView>> {
        let rows: Vec<ServerView> = sqlx::query_as(
            "SELECT alias, host, port, username, auth_kind,
                    (key_passphrase_enc IS NOT NULL) AS has_passphrase,
                    (certificate_enc IS NOT NULL) AS has_certificate,
                    jump_host_alias, description, tags
             FROM servers ORDER BY alias",
        )
        .fetch_all(&self.pool)
        .await
        .context("list servers")?;
        Ok(rows)
    }

    pub async fn create_server(&self, input: &ServerInput) -> Result<()> {
        let auth_kind = match input.auth_kind.as_str() {
            "password" | "key" | "cert" => input.auth_kind.as_str(),
            other => anyhow::bail!("invalid auth_kind '{other}' (password|key|cert)"),
        };
        if input.alias.trim().is_empty() || input.host.trim().is_empty() {
            anyhow::bail!("alias and host are required");
        }
        let secret_enc = self
            .key
            .encrypt(input.secret.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt secret: {e}"))?;
        let passphrase_enc = match input.key_passphrase.as_deref().filter(|s| !s.is_empty()) {
            Some(p) => Some(
                self.key
                    .encrypt(p.as_bytes())
                    .map_err(|e| anyhow::anyhow!("encrypt passphrase: {e}"))?,
            ),
            None => None,
        };
        let cert_enc = match input.certificate.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(c) => Some(
                self.key
                    .encrypt(c.as_bytes())
                    .map_err(|e| anyhow::anyhow!("encrypt certificate: {e}"))?,
            ),
            None => None,
        };
        if auth_kind == "cert" && cert_enc.is_none() {
            anyhow::bail!("auth_kind=cert requires a certificate");
        }

        let mut tx = self.pool.begin().await.context("begin")?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO servers
               (alias, host, port, username, auth_kind, secret_enc,
                key_passphrase_enc, certificate_enc, jump_host_alias, description, tags)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)
             RETURNING id",
        )
        .bind(input.alias.trim())
        .bind(input.host.trim())
        .bind(input.port as i64)
        .bind(input.username.trim())
        .bind(auth_kind)
        .bind(secret_enc)
        .bind(passphrase_enc)
        .bind(cert_enc)
        .bind(input.jump_host_alias.as_deref().filter(|s| !s.is_empty()))
        .bind(input.description.as_deref().filter(|s| !s.is_empty()))
        .bind(input.tags.as_deref().filter(|s| !s.is_empty()))
        .fetch_one(&mut *tx)
        .await
        .context("insert server (alias may already exist)")?;

        sqlx::query("INSERT OR IGNORE INTO server_user_permissions(server_id, user_id) VALUES (?, ?)")
            .bind(id)
            .bind(DEFAULT_USER_ID)
            .execute(&mut *tx)
            .await
            .context("grant permission")?;
        tx.commit().await.context("commit")?;
        Ok(())
    }

    pub async fn delete_server(&self, alias: &str) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin")?;
        sqlx::query(
            "DELETE FROM server_user_permissions
             WHERE server_id IN (SELECT id FROM servers WHERE alias = ?)",
        )
        .bind(alias)
        .execute(&mut *tx)
        .await
        .context("delete permissions")?;
        sqlx::query("DELETE FROM servers WHERE alias = ?")
            .bind(alias)
            .execute(&mut *tx)
            .await
            .context("delete server")?;
        tx.commit().await.context("commit")?;
        Ok(())
    }

    // ---- tokens ----------------------------------------------------------

    pub async fn list_tokens(&self) -> Result<Vec<TokenView>> {
        let rows: Vec<TokenView> = sqlx::query_as(
            "SELECT id, label, created_at, last_used_at, revoked_at
             FROM mcp_tokens WHERE user_id = ? ORDER BY id DESC",
        )
        .bind(DEFAULT_USER_ID)
        .fetch_all(&self.pool)
        .await
        .context("list tokens")?;
        Ok(rows)
    }

    /// Create a token, returning the plaintext exactly once. Only its hash is
    /// stored — it cannot be recovered later.
    pub async fn create_token(&self, label: &str) -> Result<String> {
        let label = label.trim();
        if label.is_empty() {
            anyhow::bail!("label is required");
        }
        let mut raw = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut raw);
        let token = format!("cdt_{}", hex::encode(raw));
        let hash = sha256_hex(&token);
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO mcp_tokens(user_id, label, token_hash, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(DEFAULT_USER_ID)
        .bind(label)
        .bind(&hash)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("insert token")?;
        Ok(token)
    }

    pub async fn revoke_token(&self, id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE mcp_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("revoke token")?;
        Ok(())
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerView {
    pub alias: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_kind: String,
    pub has_passphrase: bool,
    pub has_certificate: bool,
    pub jump_host_alias: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,
}

#[derive(Deserialize)]
pub struct ServerInput {
    pub alias: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth_kind: String,
    /// Password (password) or OpenSSH private key PEM (key/cert).
    pub secret: String,
    pub key_passphrase: Option<String>,
    pub certificate: Option<String>,
    pub jump_host_alias: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,
}

fn default_port() -> u16 {
    22
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TokenView {
    pub id: i64,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}
