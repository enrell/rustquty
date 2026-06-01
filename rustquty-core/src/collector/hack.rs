//! Hack collector — runs `cargo hack check --feature-powerset`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct HackCollector;

impl HackCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_feature_count(&self, stderr: &str) -> u32 {
        // Look for "checking N combinations" or "testing X feature combinations"
        for line in stderr.lines() {
            if let Some(n) = line.split_whitespace().find_map(|w| w.parse::<u32>().ok()) {
                // Heuristic: look for lines with "combinations" nearby
                if stderr.contains("combination") {
                    return n;
                }
            }
        }
        0
    }
}

impl Collector for HackCollector {
    fn name(&self) -> &'static str {
        "hack"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo-hack")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo-hack")
            .args(["check", "--feature-powerset", "--no-dev-deps"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let feature_combinations = self.parse_feature_count(&stderr);
        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let details = serde_json::json!({
            "featureCombinationsTested": feature_combinations,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr,
        })
    }
}

impl Default for HackCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature_count() {
        let collector = HackCollector::new();
        let stderr = "Checking feature combinations...\nChecked 16 combinations";
        assert_eq!(collector.parse_feature_count(stderr), 16);
    }
}
