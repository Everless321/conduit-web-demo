//! Demo-side command authorizer. Wraps the engine's destructive-command
//! blacklist (`conduit_core::policy::CommandPolicy`) with an optional allowlist:
//! when allow patterns are configured, a command must BOTH survive the blacklist
//! AND match one allow pattern, otherwise it is `Forbidden`.
//!
//! The allowlist is hot-reloadable at runtime (`reload`) so the management UI can
//! edit it without a restart. An empty allowlist means "not enforced" — identical
//! to the blacklist-only default.

use std::sync::RwLock;

use async_trait::async_trait;
use regex::RegexSet;

use conduit_core::models::AuthContext;
use conduit_core::policy::CommandPolicy;
use conduit_core::{Authorizer, Error, Result};

struct Compiled {
    patterns: Vec<String>,
    allow: Option<RegexSet>,
}

pub struct WhitelistPolicy {
    /// Always-on destructive-command blacklist (defense in depth).
    blacklist: CommandPolicy,
    /// Current allowlist; swapped atomically on reload.
    inner: RwLock<Compiled>,
}

impl WhitelistPolicy {
    /// Build from allow patterns (regex). Empty = blacklist-only.
    pub fn new(allow_patterns: Vec<String>) -> Result<Self> {
        let blacklist = CommandPolicy::new(vec![])?;
        Ok(Self {
            blacklist,
            inner: RwLock::new(Self::compile(allow_patterns)?),
        })
    }

    fn compile(patterns: Vec<String>) -> Result<Compiled> {
        let allow = if patterns.is_empty() {
            None
        } else {
            Some(
                RegexSet::new(&patterns)
                    .map_err(|e| Error::Invalid(format!("allowlist regex: {e}")))?,
            )
        };
        Ok(Compiled { patterns, allow })
    }

    /// Replace the live allowlist. Validates regex before swapping; on error the
    /// current set is left untouched.
    pub fn reload(&self, patterns: Vec<String>) -> Result<()> {
        let compiled = Self::compile(patterns)?;
        *self.inner.write().expect("policy lock") = compiled;
        Ok(())
    }

    /// Current patterns plus whether the allowlist is actively enforced.
    pub fn snapshot(&self) -> (Vec<String>, bool) {
        let g = self.inner.read().expect("policy lock");
        (g.patterns.clone(), g.allow.is_some())
    }

    pub fn enforced(&self) -> bool {
        self.inner.read().expect("policy lock").allow.is_some()
    }
}

#[async_trait]
impl Authorizer for WhitelistPolicy {
    async fn authorize_exec(
        &self,
        _auth: &AuthContext,
        _server_alias: &str,
        command: &str,
    ) -> Result<()> {
        // Destructive blacklist always applies, even with an allowlist set.
        self.blacklist.check(command)?;
        let g = self.inner.read().expect("policy lock");
        if let Some(set) = &g.allow {
            if !set.is_match(command) {
                return Err(Error::Forbidden(format!(
                    "command not permitted by allowlist ({} patterns)",
                    g.patterns.len()
                )));
            }
        }
        Ok(())
    }
}
