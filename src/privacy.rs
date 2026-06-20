//! Output redaction via the [privacy-filter](https://github.com/packyme/privacy-filter)
//! service. Implements the engine's `OutputFilter` seam (conduit v0.2.0): every
//! `exec` / poll-chunk result is sent to privacy-filter's HTTP `/redact/batch`
//! and the caller-facing stdout/stderr are replaced with the redacted text.
//!
//! The audit trail keeps the RAW output (engine design) — this only shapes what
//! the MCP caller receives.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use conduit_core::{CapturedOutput, OutputContext, OutputFilter};

#[derive(Serialize)]
struct BatchRequest {
    texts: Vec<String>,
}

/// One element of privacy-filter's `/redact/batch` response array. We only need
/// `redacted`; the other fields (hit/count/entities/elapsed_ms) are ignored.
#[derive(Deserialize)]
struct RedactResponse {
    redacted: String,
}

pub struct PrivacyFilter {
    endpoint: String,
    client: reqwest::Client,
    /// On privacy-filter error: return raw output (true) or withhold it (false).
    fail_open: bool,
}

impl PrivacyFilter {
    pub fn new(base_url: String, fail_open: bool) -> Self {
        let endpoint = format!("{}/redact/batch", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("build reqwest client");
        Self { endpoint, client, fail_open }
    }

    async fn redact(&self, texts: Vec<String>) -> reqwest::Result<Vec<String>> {
        let resp = self
            .client
            .post(&self.endpoint)
            .json(&BatchRequest { texts })
            .send()
            .await?
            .error_for_status()?;
        let arr: Vec<RedactResponse> = resp.json().await?;
        Ok(arr.into_iter().map(|r| r.redacted).collect())
    }
}

#[async_trait]
impl OutputFilter for PrivacyFilter {
    async fn filter(&self, ctx: &OutputContext<'_>, out: &mut CapturedOutput) {
        if out.stdout.is_empty() && out.stderr.is_empty() {
            return;
        }
        let stdout = std::mem::take(&mut out.stdout);
        let stderr = std::mem::take(&mut out.stderr);
        match self.redact(vec![stdout.clone(), stderr.clone()]).await {
            Ok(mut r) if r.len() == 2 => {
                out.stderr = r.pop().unwrap();
                out.stdout = r.pop().unwrap();
            }
            other => {
                match &other {
                    Err(e) => tracing::warn!(
                        error = %e, server = ctx.server_alias,
                        "privacy-filter call failed"
                    ),
                    Ok(_) => tracing::warn!(
                        server = ctx.server_alias,
                        "privacy-filter returned unexpected response shape"
                    ),
                }
                if self.fail_open {
                    out.stdout = stdout;
                    out.stderr = stderr;
                } else {
                    out.stdout = String::new();
                    out.stderr =
                        "[privacy-filter unavailable; output withheld]".to_string();
                }
            }
        }
    }
}
