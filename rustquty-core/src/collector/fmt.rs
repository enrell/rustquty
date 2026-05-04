//! Fmt collector — runs `cargo fmt --check`.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct FmtCollector;

impl FmtCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Collector for FmtCollector {
    fn name(&self) -> &'static str {
        "fmt"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo")
            .args(["fmt", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output = Command::new("cargo")
            .args(["fmt", "--check"])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

impl Default for FmtCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_collector_name() {
        let collector = FmtCollector::new();
        assert_eq!(collector.name(), "fmt");
    }
}
