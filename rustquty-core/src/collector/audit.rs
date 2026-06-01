//! Audit collector — runs `cargo audit`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct AuditCollector;

impl AuditCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_json_output(&self, stdout: &str) -> (u32, u32) {
        let mut vulnerability_count = 0u32;
        let mut critical_count = 0u32;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
            if let Some(found) = json.get("vulnerabilities").and_then(|v| v.get("found")) {
                vulnerability_count = found.as_u64().unwrap_or(0) as u32;
            }
            if let Some(list) = json.get("vulnerabilities").and_then(|v| v.get("list"))
                && let Some(arr) = list.as_array()
            {
                for item in arr {
                    if let Some(severity) = item.get("severity").and_then(|s| s.as_str())
                        && severity.eq_ignore_ascii_case("critical")
                    {
                        critical_count += 1;
                    }
                }
            }
        }

        (vulnerability_count, critical_count)
    }
}

impl Collector for AuditCollector {
    fn name(&self) -> &'static str {
        "audit"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo-audit")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo-audit")
            .args(["--json"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (vulnerability_count, critical_count) = self.parse_json_output(&raw_stdout);
        let status = if vulnerability_count == 0 {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let details = serde_json::json!({
            "vulnerabilityCount": vulnerability_count,
            "criticalCount": critical_count,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr,
        })
    }
}

impl Default for AuditCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_output_no_vulns() {
        let collector = AuditCollector::new();
        let json = r#"{"vulnerabilities":{"found":0,"list":[]}}"#;
        let (vuln, critical) = collector.parse_json_output(json);
        assert_eq!(vuln, 0);
        assert_eq!(critical, 0);
    }

    #[test]
    fn test_parse_json_output_with_vulns() {
        let collector = AuditCollector::new();
        let json = r#"{"vulnerabilities":{"found":2,"list":[{"id":"RUSTSEC-0001","severity":"High"},{"id":"RUSTSEC-0002","severity":"critical"}]}}"#;
        let (vuln, critical) = collector.parse_json_output(json);
        assert_eq!(vuln, 2);
        assert_eq!(critical, 1);
    }
}
