//! Test collector — runs `cargo test` or `cargo nextest`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct TestCollector;

impl TestCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn runner_name(&self) -> &'static str {
        if Command::new("cargo-nextest")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "nextest"
        } else {
            "test"
        }
    }

    fn parse_test_output(&self, stdout: &str, stderr: &str) -> (u32, u32, u32) {
        // Simple heuristic: parse "test result: ok. X passed; Y failed; Z ignored"
        let combined = format!("{}\n{}", stdout, stderr);
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut ignored = 0u32;

        for line in combined.lines() {
            if line.contains("test result:") {
                let parts: Vec<&str> = line.split(';').collect();
                for part in parts {
                    let part = part.trim();
                    if part.contains("passed")
                        && let Some(n) = part.split_whitespace().find(|w| w.parse::<u32>().is_ok())
                    {
                        passed = n.parse().unwrap_or(0);
                    } else if part.contains("failed")
                        && let Some(n) = part.split_whitespace().find(|w| w.parse::<u32>().is_ok())
                    {
                        failed = n.parse().unwrap_or(0);
                    } else if part.contains("ignored")
                        && let Some(n) = part.split_whitespace().find(|w| w.parse::<u32>().is_ok())
                    {
                        ignored = n.parse().unwrap_or(0);
                    }
                }
            }
        }

        (passed, failed, ignored)
    }
}

impl Collector for TestCollector {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo")
            .args(["test", "--no-run", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let runner = self.runner_name();

        let output = if runner == "nextest" {
            Command::new("cargo")
                .args(["nextest", "run", "--message-format=json"])
                .current_dir(&ctx.workspace_root)
                .output()
        } else {
            Command::new("cargo")
                .args(["test", "--message-format=json"])
                .current_dir(&ctx.workspace_root)
                .output()
        };

        let output = output.map_err(|e| CollectorError::IoError(e.to_string()))?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (_passed, failed, _ignored) = self.parse_test_output(&stdout, &stderr);
        let status = if failed > 0 {
            crate::schema::CollectorStatus::Fail
        } else {
            crate::schema::CollectorStatus::Pass
        };

        let mut result = CollectorOutput {
            status,
            duration_ms,
            stdout,
            stderr,
        };
        // Attach runner info in stderr as a convention for now
        if runner == "nextest" {
            result.stderr = format!("[runner: nextest] {}", result.stderr);
        }
        Ok(result)
    }
}

impl Default for TestCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_output() {
        let collector = TestCollector::new();
        let output = "running 10 tests\ntest result: ok. 8 passed; 1 failed; 1 ignored";
        let (passed, failed, ignored) = collector.parse_test_output(output, "");
        assert_eq!(passed, 8);
        assert_eq!(failed, 1);
        assert_eq!(ignored, 1);
    }

    #[test]
    fn test_parse_test_output_with_rustc_output() {
        let collector = TestCollector::new();
        // Simulate rustc output format
        let output = "test result: ok. 42 passed; 0 failed; 0 ignored";
        let (passed, failed, ignored) = collector.parse_test_output(output, "");
        assert_eq!(passed, 42);
        assert_eq!(failed, 0);
        assert_eq!(ignored, 0);
    }
}
