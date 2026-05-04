//! Mutants collector — runs `cargo mutants` and parses outcomes.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::process::Command;

pub struct MutantsCollector;

impl MutantsCollector {
    pub fn new() -> Self {
        Self
    }

    fn parse_outcomes(&self, path: &std::path::Path) -> Result<(f64, u32, u32), CollectorError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| CollectorError::IoError(e.to_string()))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| CollectorError::ParseError(e.to_string()))?;

        let caught = json.get("caught").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let missed = json.get("missed").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let total = caught + missed;
        let score = if total > 0 {
            caught as f64 / total as f64
        } else {
            0.0
        };

        Ok((score, caught, missed))
    }
}

impl Collector for MutantsCollector {
    fn name(&self) -> &'static str {
        "mutants"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo-mutants")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();
        let output_path = ctx.output_dir.join("mutants.out");

        let output = Command::new("cargo-mutants")
            .args(["--output", output_path.to_string_lossy().as_ref()])
            .current_dir(&ctx.workspace_root)
            .output()
            .map_err(|e| CollectorError::IoError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let outcomes_path = output_path.join("outcomes.json");
        let (mutation_score, caught, missed) = if outcomes_path.exists() {
            self.parse_outcomes(&outcomes_path)?
        } else {
            (0.0, 0, 0)
        };

        let status = if output.status.success() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let mut result = CollectorOutput {
            status,
            duration_ms,
            stdout,
            stderr,
        };
        result.stdout = format!(
            "mutation_score={:.3} caught={} missed={}",
            mutation_score, caught, missed
        );
        Ok(result)
    }
}

impl Default for MutantsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_outcomes() {
        let collector = MutantsCollector::new();
        let td = tempfile::TempDir::new().unwrap();
        let path = td.path().join("outcomes.json");
        std::fs::write(&path, r#"{"caught":80,"missed":20}"#).unwrap();
        let (score, caught, missed) = collector.parse_outcomes(&path).unwrap();
        assert!((score - 0.8).abs() < f64::EPSILON);
        assert_eq!(caught, 80);
        assert_eq!(missed, 20);
    }
}
