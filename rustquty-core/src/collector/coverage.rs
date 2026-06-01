//! Coverage collector — runs `cargo llvm-cov`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct CoverageCollector;

impl CoverageCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_json_output(&self, stdout: &str) -> Option<f64> {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
            json.get("lines")
                .and_then(|v| v.get("percent"))
                .and_then(|v| v.as_f64())
                .or_else(|| {
                    // Try alternative format
                    json.get("totals")
                        .and_then(|v| v.get("lines"))
                        .and_then(|v| v.get("percent"))
                        .and_then(|v| v.as_f64())
                })
        } else {
            None
        }
    }
}

impl Collector for CoverageCollector {
    fn name(&self) -> &'static str {
        "coverage"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo")
            .args(["llvm-cov", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo")
            .args(["llvm-cov", "--json", "--quiet"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let line_percent = self.parse_json_output(&raw_stdout).unwrap_or(0.0);
        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Error
        };

        let details = serde_json::json!({
            "linePercent": line_percent,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr,
        })
    }
}

impl Default for CoverageCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_output() {
        let collector = CoverageCollector::new();
        let json = r#"{"lines":{"percent":87.5}}"#;
        assert!((collector.parse_json_output(json).unwrap() - 87.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_json_output_alternative_format() {
        let collector = CoverageCollector::new();
        let json = r#"{"totals":{"lines":{"percent":92.3}}}"#;
        assert!((collector.parse_json_output(json).unwrap() - 92.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_json_output_empty() {
        let collector = CoverageCollector::new();
        assert!(collector.parse_json_output("{}").is_none());
    }

    #[test]
    fn test_parse_json_output_invalid() {
        let collector = CoverageCollector::new();
        assert!(collector.parse_json_output("not json").is_none());
    }
}
