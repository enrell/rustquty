//! Clippy collector — runs `cargo clippy` and parses JSON output.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use crate::schema::ClippyLint;
use std::process::Command;

pub struct ClippyCollector;

impl ClippyCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_json_output(&self, stdout: &str) -> (u32, Vec<ClippyLint>) {
        let mut warning_count = 0u32;
        let mut lints = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                let level = msg.get("level").and_then(|v| v.as_str()).unwrap_or("");
                let code = msg
                    .get("code")
                    .and_then(|v| v.get("code"))
                    .and_then(|v| v.as_str());

                if (level == "warning" || level == "error") && code.is_some() {
                    warning_count += 1;
                    let lint = ClippyLint {
                        code: code.unwrap_or("").to_string(),
                        message: msg
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        file: msg.get("file").and_then(|v| v.as_str()).map(String::from),
                        line: msg.get("line").and_then(|v| v.as_u64()).map(|v| v as u32),
                    };
                    lints.push(lint);
                }
            }
        }

        (warning_count, lints)
    }
}

impl Collector for ClippyCollector {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo")
            .args(["clippy", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo")
            .args(["clippy", "--message-format=json", "--quiet"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Even if exit code is non-zero due to warnings, we parse the output
        let (warning_count, _lints) = self.parse_json_output(&stdout);
        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else if warning_count > 0 {
            crate::schema::CollectorStatus::Fail
        } else {
            // Non-zero but no warnings parsed — could be an error
            crate::schema::CollectorStatus::Error
        };

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout,
            stderr,
        })
    }
}

impl Default for ClippyCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_output_with_warnings() {
        let collector = ClippyCollector::new();
        let json_output = r#"{"message":"warning: unused variable: `x`","level":"warning","code":{"code":"unused_variables","explanation":"..."},"file":"src/main.rs","line":5}
{"message":"error: expected `,`, found `{`","level":"error","code":{"code":"parse_error","explanation":"..."},"file":"src/main.rs","line":10}
"#;
        let (count, lints) = collector.parse_json_output(json_output);
        assert_eq!(count, 2);
        assert_eq!(lints[0].code, "unused_variables");
        assert_eq!(lints[1].code, "parse_error");
    }

    #[test]
    fn test_parse_json_output_empty() {
        let collector = ClippyCollector::new();
        let (count, lints) = collector.parse_json_output("");
        assert_eq!(count, 0);
        assert!(lints.is_empty());
    }
}
