//! Deny collector — runs `cargo deny check`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct DenyCollector;

impl DenyCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_json_output(&self, stdout: &str) -> (u32, u32) {
        let mut banned_count = 0u32;
        let mut license_violations = 0u32;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
            if let Some(ban) = json.get("ban").and_then(|v| v.get("list"))
                && let Some(arr) = ban.as_array()
            {
                banned_count = arr.len() as u32;
            }
            if let Some(license) = json.get("license").and_then(|v| v.get("violations"))
                && let Some(arr) = license.as_array()
            {
                license_violations = arr.len() as u32;
            }
        }

        (banned_count, license_violations)
    }
}

impl Collector for DenyCollector {
    fn name(&self) -> &'static str {
        "deny"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo-deny")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo-deny")
            .args(["check", "--format=json"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (_banned_count, _license_violations) = self.parse_json_output(&stdout);
        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout,
            stderr,
        })
    }
}

impl Default for DenyCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_output_empty() {
        let collector = DenyCollector::new();
        let (banned, license) = collector.parse_json_output("{}");
        assert_eq!(banned, 0);
        assert_eq!(license, 0);
    }
}
