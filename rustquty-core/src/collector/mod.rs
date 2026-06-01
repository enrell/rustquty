//! Collector framework and trait definition.

pub mod audit;
pub mod clippy;
pub mod complexity;
pub mod coverage;
pub mod deny;
pub mod duplicates;
pub mod fmt;
pub mod hack;
pub mod loc;
pub mod mutants;
pub mod size;
pub mod tests;

use crate::context::Context;
use crate::schema::{
    AuditResult, ClippyLint, ClippyResult, CollectorStatus, ComplexityResult, CoverageResult,
    DenyResult, DuplicatesResult, FmtResult, HackResult, LocResult, MetricsSummary, MutantsResult,
    ProjectInfo, SizeResult, TestResult,
};

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

/// Execute collectors and return raw results.
///
/// Respects `ctx.disabled_collectors`. Skips unavailable collectors.
/// Runs in parallel if `parallel` is true.
pub fn execute_collectors<'a>(
    collectors: &'a [Box<dyn Collector>],
    ctx: &Context,
    parallel: bool,
) -> Vec<(&'a str, CollectorOutput)> {
    if parallel {
        use rayon::prelude::*;
        collectors
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
            .collect()
    } else {
        let mut results = Vec::new();
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
        results
    }
}

/// Assemble collector results into a MetricsSummary.
///
/// Parses the JSON stdout of each collector to populate detailed metrics.
pub fn assemble_results(
    results: &[(&str, CollectorOutput)],
    project_name: &str,
    rust_edition: &str,
    workspace_root: &str,
) -> MetricsSummary {
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
    let mut duplicates_result = DuplicatesResult {
        status: CollectorStatus::Skipped,
        total_lines: 0,
        duplicate_lines: 0,
        files_with_duplicates: 0,
        duplicate_files: vec![],
    };
    let mut loc_result = LocResult {
        status: CollectorStatus::Skipped,
        total_lines: 0,
        code_lines: 0,
        comment_lines: 0,
        blank_lines: 0,
        long_lines: 0,
        max_line_length_found: 0,
        max_line_length_allowed: 0,
        files: 0,
        files_with_long_lines: 0,
        long_line_files: vec![],
    };
    let mut size_result = SizeResult {
        status: CollectorStatus::Skipped,
        files: 0,
        max_lines_per_file: 0,
        max_code_lines_per_file: 0,
        max_lines_per_function: 0,
        max_parameters_per_function: 0,
        violations: vec![],
    };
    let mut complexity_result = ComplexityResult {
        status: CollectorStatus::Skipped,
        functions: 0,
        max_cyclomatic_complexity: 0,
        max_nesting_depth: 0,
        complex_functions: 0,
        violations: vec![],
    };

    for (name, output) in results {
        let details = serde_json::from_str::<serde_json::Value>(&output.stdout).ok();
        match *name {
            "fmt" => fmt_result.status.clone_from(&output.status),
            "clippy" => {
                if let Some(ref d) = details {
                    clippy_result.warning_count = d["warningCount"].as_u64().unwrap_or(0) as u32;
                    if let Some(arr) = d["details"].as_array() {
                        clippy_result.details = arr
                            .iter()
                            .map(|v| ClippyLint {
                                code: v["code"].as_str().unwrap_or("").to_string(),
                                message: v["message"].as_str().unwrap_or("").to_string(),
                                file: v["file"].as_str().map(String::from),
                                line: v["line"].as_u64().map(|v| v as u32),
                            })
                            .collect();
                    }
                }
                clippy_result.status.clone_from(&output.status);
            }
            "tests" => {
                if let Some(ref d) = details {
                    test_result.passed = d["passed"].as_u64().unwrap_or(0) as u32;
                    test_result.failed = d["failed"].as_u64().unwrap_or(0) as u32;
                    test_result.ignored = d["ignored"].as_u64().unwrap_or(0) as u32;
                    test_result.runner = d["runner"].as_str().map(String::from);
                }
                test_result.status.clone_from(&output.status);
            }
            "coverage" => {
                if let Some(ref d) = details {
                    coverage_result.line_percent = d["linePercent"].as_f64().unwrap_or(0.0);
                }
                coverage_result.status.clone_from(&output.status);
            }
            "deny" => {
                if let Some(ref d) = details {
                    deny_result.banned_count = d["bannedCount"].as_u64().unwrap_or(0) as u32;
                    deny_result.license_violations =
                        d["licenseViolations"].as_u64().unwrap_or(0) as u32;
                }
                deny_result.status.clone_from(&output.status);
            }
            "audit" => {
                if let Some(ref d) = details {
                    audit_result.vulnerability_count =
                        d["vulnerabilityCount"].as_u64().unwrap_or(0) as u32;
                    audit_result.critical_count =
                        d["criticalCount"].as_u64().unwrap_or(0) as u32;
                }
                audit_result.status.clone_from(&output.status);
            }
            "hack" => {
                if let Some(ref d) = details {
                    hack_result.feature_combinations_tested =
                        d["featureCombinationsTested"].as_u64().unwrap_or(0) as u32;
                }
                hack_result.status.clone_from(&output.status);
            }
            "mutants" => mutants_result.status.clone_from(&output.status),
            "duplicates" => {
                if let Some(ref d) = details {
                    duplicates_result.total_lines = d["totalLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.duplicate_lines =
                        d["duplicateLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.files_with_duplicates =
                        d["filesWithDuplicates"].as_u64().unwrap_or(0) as u32;
                }
                duplicates_result.status.clone_from(&output.status);
            }
            "loc" => {
                if let Some(ref d) = details {
                    loc_result.total_lines = d["totalLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.code_lines = d["codeLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.comment_lines = d["commentLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.blank_lines = d["blankLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.long_lines = d["longLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.max_line_length_found =
                        d["maxLineLengthFound"].as_u64().unwrap_or(0) as usize;
                    loc_result.files = d["files"].as_u64().unwrap_or(0) as u32;
                }
                loc_result.status.clone_from(&output.status);
            }
            "size" => {
                if let Some(ref d) = details {
                    size_result.files = d["files"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_file =
                        d["maxLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_code_lines_per_file =
                        d["maxCodeLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_function =
                        d["maxLinesPerFunction"].as_u64().unwrap_or(0) as u32;
                    size_result.max_parameters_per_function =
                        d["maxParametersPerFunction"].as_u64().unwrap_or(0) as u32;
                    if let Some(arr) = d["violations"].as_array() {
                        size_result.violations = arr
                            .iter()
                            .map(|v| crate::schema::SizeViolation {
                                rule_id: v["ruleId"].as_str().unwrap_or("").to_string(),
                                file: v["file"].as_str().unwrap_or("").to_string(),
                                line: v["line"].as_u64().unwrap_or(0) as u32,
                                function: v["function"].as_str().map(String::from),
                                message: v["message"].as_str().unwrap_or("").to_string(),
                                actual: v["actual"].as_u64().unwrap_or(0) as u32,
                                threshold: v["threshold"].as_u64().unwrap_or(0) as u32,
                                severity: v["severity"].as_str().unwrap_or("").to_string(),
                            })
                            .collect();
                    }
                }
                size_result.status.clone_from(&output.status);
            }
            "complexity" => {
                if let Some(ref d) = details {
                    complexity_result.functions = d["functions"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_cyclomatic_complexity =
                        d["maxCyclomaticComplexity"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_nesting_depth =
                        d["maxNestingDepth"].as_u64().unwrap_or(0) as u32;
                    complexity_result.complex_functions =
                        d["complexFunctions"].as_u64().unwrap_or(0) as u32;
                    if let Some(arr) = d["violations"].as_array() {
                        complexity_result.violations = arr
                            .iter()
                            .map(|v| crate::schema::ComplexityViolation {
                                rule_id: v["ruleId"].as_str().unwrap_or("").to_string(),
                                file: v["file"].as_str().unwrap_or("").to_string(),
                                line: v["line"].as_u64().unwrap_or(0) as u32,
                                function: v["function"].as_str().map(String::from),
                                message: v["message"].as_str().unwrap_or("").to_string(),
                                actual: v["actual"].as_u64().unwrap_or(0) as u32,
                                threshold: v["threshold"].as_u64().unwrap_or(0) as u32,
                                severity: v["severity"].as_str().unwrap_or("").to_string(),
                            })
                            .collect();
                    }
                }
                complexity_result.status.clone_from(&output.status);
            }
            _ => {}
        }
    }

    MetricsSummary {
        schema_version: "1".to_string(),
        generated_at: crate::util::chrono_now(),
        rustquty_version: env!("CARGO_PKG_VERSION").to_string(),
        project: ProjectInfo {
            name: project_name.to_string(),
            rust_edition: rust_edition.to_string(),
            workspace_root: workspace_root.to_string(),
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
            duplicates: duplicates_result,
            loc: loc_result,
            size: size_result,
            complexity: complexity_result,
        },
    }
}

/// Execute collectors and assemble results into a MetricsSummary.
///
/// Convenience function that calls [`execute_collectors`] then [`assemble_results`].
pub fn run_collectors(
    collectors: &[Box<dyn Collector>],
    ctx: &Context,
    parallel: bool,
) -> MetricsSummary {
    let results = execute_collectors(collectors, ctx, parallel);
    let project_name = ctx
        .workspace_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    assemble_results(
        &results,
        &project_name,
        "2021",
        &ctx.workspace_root.to_string_lossy(),
    )
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
