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
    AuditResult, ClippyResult, CollectorStatus, ComplexityResult, CoverageResult, DenyResult,
    DuplicatesResult, FmtResult, HackResult, LocResult, MetricsSummary, MutantsResult, ProjectInfo,
    SizeResult, TestResult,
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

pub fn run_collectors(
    collectors: &[Box<dyn Collector>],
    ctx: &Context,
    parallel: bool,
) -> MetricsSummary {
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
            "duplicates" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    duplicates_result.total_lines =
                        details["totalLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.duplicate_lines =
                        details["duplicateLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.files_with_duplicates =
                        details["filesWithDuplicates"].as_u64().unwrap_or(0) as u32;
                }
                duplicates_result.status.clone_from(&output.status);
            }
            "loc" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    loc_result.total_lines = details["totalLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.code_lines = details["codeLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.comment_lines = details["commentLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.blank_lines = details["blankLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.long_lines = details["longLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.max_line_length_found =
                        details["maxLineLengthFound"].as_u64().unwrap_or(0) as usize;
                    loc_result.files = details["files"].as_u64().unwrap_or(0) as u32;
                }
                loc_result.status.clone_from(&output.status);
            }
            "size" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    size_result.files = details["files"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_file =
                        details["maxLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_code_lines_per_file =
                        details["maxCodeLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_function =
                        details["maxLinesPerFunction"].as_u64().unwrap_or(0) as u32;
                    size_result.max_parameters_per_function =
                        details["maxParametersPerFunction"].as_u64().unwrap_or(0) as u32;
                }
                size_result.status.clone_from(&output.status);
            }
            "complexity" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    complexity_result.functions =
                        details["functions"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_cyclomatic_complexity =
                        details["maxCyclomaticComplexity"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_nesting_depth =
                        details["maxNestingDepth"].as_u64().unwrap_or(0) as u32;
                    complexity_result.complex_functions =
                        details["complexFunctions"].as_u64().unwrap_or(0) as u32;
                }
                complexity_result.status.clone_from(&output.status);
            }
            _ => {}
        }
    }

    MetricsSummary {
        schema_version: "1".to_string(),
        generated_at: chrono_now(),
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
            duplicates: duplicates_result,
            loc: loc_result,
            size: size_result,
            complexity: complexity_result,
        },
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let (year, month, day, hour, min, sec) = unix_to_datetime(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn unix_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let min = (time_of_day % 3600) / 60;
    let sec = time_of_day % 60;

    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u64;
    for &d in &month_days {
        if remaining < d {
            break;
        }
        remaining -= d;
        m += 1;
    }
    let d = remaining + 1;
    (y, m, d, hour, min, sec)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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
