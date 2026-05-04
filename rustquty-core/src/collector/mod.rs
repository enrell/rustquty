//! Collector framework and trait definition.

pub mod audit;
pub mod clippy;
pub mod coverage;
pub mod deny;
pub mod fmt;
pub mod hack;
pub mod mutants;
pub mod tests;

use crate::context::Context;
use crate::schema::{
    AuditResult, ClippyResult, CollectorStatus, CoverageResult, DenyResult, FmtResult, HackResult,
    MetricsSummary, MutantsResult, ProjectInfo, TestResult,
};
use std::time::Instant;

pub trait Collector: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError>;
}

#[derive(Debug, Clone)]
pub struct CollectorOutput {
    pub status: CollectorStatus,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("collector not available: {0}")]
    NotAvailable(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("I/O error: {0}")]
    IoError(String),
}

pub struct MockCollector {
    pub name_val: &'static str,
    pub available: bool,
    pub output: CollectorOutput,
}

impl Collector for MockCollector {
    fn name(&self) -> &'static str {
        self.name_val
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn collect(&self, _ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        Ok(self.output.clone())
    }
}

pub fn run_collectors(
    collectors: &[Box<dyn Collector>],
    ctx: &Context,
    parallel: bool,
) -> MetricsSummary {
    let start = Instant::now();
    let project_name = ctx
        .workspace_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut results: Vec<(&str, CollectorOutput)> = Vec::new();

    if parallel {
        use rayon::prelude::*;
        results = collectors
            .par_iter()
            .filter(|col| {
                let name_lower = col.name().to_lowercase();
                !ctx.disabled_collectors
                    .iter()
                    .any(|c| c.to_string() == name_lower)
            })
            .filter(|col| col.is_available())
            .flat_map(|col| match col.collect(ctx) {
                Ok(o) => vec![(col.name(), o)],
                Err(e) => {
                    let output = CollectorOutput {
                        status: CollectorStatus::Error,
                        duration_ms: 0,
                        stdout: String::new(),
                        stderr: format!("{:?}", e),
                    };
                    vec![(col.name(), output)]
                }
            })
            .collect();
    } else {
        for col in collectors {
            let name_lower = col.name().to_lowercase();
            if ctx
                .disabled_collectors
                .iter()
                .any(|c| c.to_string() == name_lower)
            {
                continue;
            }
            if !col.is_available() {
                continue;
            }
            match col.collect(ctx) {
                Ok(o) => results.push((col.name(), o)),
                Err(e) => {
                    let output = CollectorOutput {
                        status: CollectorStatus::Error,
                        duration_ms: 0,
                        stdout: String::new(),
                        stderr: format!("{:?}", e),
                    };
                    results.push((col.name(), output));
                }
            }
        }
    }

    let mut fmt_result = FmtResult {
        status: CollectorStatus::Skipped,
        details: Default::default(),
    };
    let mut clippy_result = ClippyResult {
        status: CollectorStatus::Skipped,
        warning_count: 0,
        details: vec![],
    };
    let mut test_result = TestResult {
        status: CollectorStatus::Skipped,
        passed: 0,
        failed: 0,
        ignored: 0,
        runner: None,
    };
    let mut coverage_result = CoverageResult {
        status: CollectorStatus::Skipped,
        line_percent: 0.0,
    };
    let mut deny_result = DenyResult {
        status: CollectorStatus::Skipped,
        banned_count: 0,
        license_violations: 0,
    };
    let mut audit_result = AuditResult {
        status: CollectorStatus::Skipped,
        vulnerability_count: 0,
        critical_count: 0,
    };
    let mut hack_result = HackResult {
        status: CollectorStatus::Skipped,
        feature_combinations_tested: 0,
    };
    let mut mutants_result = MutantsResult {
        status: CollectorStatus::Skipped,
        mutation_score: 0.0,
        caught: 0,
        missed: 0,
    };

    for (name, output) in &results {
        match *name {
            "fmt" => fmt_result.status.clone_from(&output.status),
            "clippy" => clippy_result.status.clone_from(&output.status),
            "tests" => test_result.status.clone_from(&output.status),
            "coverage" => coverage_result.status.clone_from(&output.status),
            "deny" => deny_result.status.clone_from(&output.status),
            "audit" => audit_result.status.clone_from(&output.status),
            "hack" => hack_result.status.clone_from(&output.status),
            "mutants" => mutants_result.status.clone_from(&output.status),
            _ => {}
        }
    }

    MetricsSummary {
        schema_version: "1".to_string(),
        generated_at: format!("{}", start.elapsed().as_secs()),
        rustquty_version: env!("CARGO_PKG_VERSION").to_string(),
        project: ProjectInfo {
            name: project_name,
            rust_edition: "2021".to_string(),
            workspace_root: ctx.workspace_root.to_string_lossy().to_string(),
        },
        collectors: crate::schema::CollectorsSummary {
            fmt: fmt_result,
            clippy: clippy_result,
            tests: test_result,
            coverage: coverage_result,
            deny: deny_result,
            audit: audit_result,
            hack: hack_result,
            mutants: mutants_result,
        },
    }
}

#[cfg(test)]
mod collector_tests {
    use super::*;

    #[test]
    fn test_mock_collector() {
        let mock = MockCollector {
            name_val: "test",
            available: true,
            output: CollectorOutput {
                status: CollectorStatus::Pass,
                duration_ms: 10,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert_eq!(mock.name(), "test");
        assert!(mock.is_available());
    }
}
