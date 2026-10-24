//! Update-check wiring — UpdateKit
//! (https://github.com/SeanShaffer88/UpdateKit).
//!
//! Exact API used: UpdateKit's native Rust function
//! `updatekit::check_update(current_version, manifest_url)`, backed
//! by its `updatekit` rlib from the pinned git dependency. That call
//! fetches and evaluates a version manifest; it does not download or
//! install a package.
//!
//! The skeleton never invents a manifest URL and never phones home
//! by default: an empty `updates.manifest_url` setting means the
//! check is disabled and returns before UpdateKit is called.

use crate::VERSION;

/// Outcome of an update check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheck {
    Disabled,
    Checked(String),
    Unsupported(String),
    Failed(String),
}

impl UpdateCheck {
    pub fn summary(&self) -> String {
        match self {
            Self::Disabled => "update check disabled — no manifest URL configured".to_string(),
            Self::Checked(json) => format!("update check completed: {json}"),
            Self::Unsupported(message) => format!("update check unsupported: {message}"),
            Self::Failed(message) => format!("update check failed: {message}"),
        }
    }
}

/// Check the configured manifest URL, if one is configured.
pub fn check_for_updates(manifest_url: &str) -> UpdateCheck {
    let url = manifest_url.trim();
    if url.is_empty() {
        return UpdateCheck::Disabled;
    }
    match updatekit::check_update(VERSION, url) {
        Ok(json) => UpdateCheck::Checked(json),
        Err((code, message)) if code == updatekit::ERR_NOT_SUPPORTED => {
            UpdateCheck::Unsupported(message)
        }
        Err((code, message)) => UpdateCheck::Failed(format!("({code}) {message}")),
    }
}
