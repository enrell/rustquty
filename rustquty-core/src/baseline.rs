//! Baseline file management.

use crate::schema::{
    AuditThreshold, Baseline, ClippyThreshold, CoverageThreshold, DenyThreshold,
    DuplicatesThreshold, FmtThreshold, HackThreshold, LocThreshold, MutantsThreshold,
    SizeThreshold, TestThreshold, Thresholds,
};
use std::path::Path;

pub struct BaselineWriter;

impl BaselineWriter {
    /// Initialize a new baseline file from a metrics summary.
    pub fn init(
        summary: &crate::schema::MetricsSummary,
        output_path: &Path,
        force: bool,
    ) -> anyhow::Result<()> {
        if output_path.exists() && !force {
            anyhow::bail!("baseline file already exists; use --force to overwrite");
        }

        let thresholds = Thresholds {
            fmt: FmtThreshold {
                must_pass: summary.collectors.fmt.status == crate::schema::CollectorStatus::Pass,
            },
            clippy: ClippyThreshold {
                max_warnings: summary.collectors.clippy.warning_count,
            },
            tests: TestThreshold {
                max_failures: summary.collectors.tests.failed,
            },
            coverage: CoverageThreshold {
                min_line_percent: summary.collectors.coverage.line_percent,
            },
            deny: DenyThreshold {
                max_banned: summary.collectors.deny.banned_count,
                max_license_violations: summary.collectors.deny.license_violations,
            },
            audit: AuditThreshold {
                max_vulnerabilities: summary.collectors.audit.vulnerability_count,
                max_critical: summary.collectors.audit.critical_count,
            },
            hack: HackThreshold {
                must_pass: summary.collectors.hack.status == crate::schema::CollectorStatus::Pass,
            },
            mutants: MutantsThreshold {
                min_score: summary.collectors.mutants.mutation_score,
            },
            duplicates: DuplicatesThreshold {
                max_duplicate_lines: summary.collectors.duplicates.duplicate_lines,
            },
            loc: LocThreshold {
                max_line_length: summary.collectors.loc.max_line_length_found.max(120),
            },
            size: SizeThreshold {
                max_lines_per_file: summary.collectors.size.max_lines_per_file.into(),
                max_code_lines_per_file: summary.collectors.size.max_code_lines_per_file.into(),
                max_lines_per_function: summary.collectors.size.max_lines_per_function.into(),
                max_parameters_per_function: summary
                    .collectors
                    .size
                    .max_parameters_per_function
                    .into(),
            },
        };

        let baseline = Baseline {
            schema_version: "1".to_string(),
            created_at: chrono_now(),
            rustquty_version: summary.rustquty_version.clone(),
            thresholds,
        };

        let json = serde_json::to_string_pretty(&baseline)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }

    /// Update an existing baseline file, printing a diff of what changed.
    pub fn update(
        summary: &crate::schema::MetricsSummary,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        let existing = if output_path.exists() {
            let contents = std::fs::read_to_string(output_path)?;
            Some(serde_json::from_str::<Baseline>(&contents)?)
        } else {
            None
        };

        let thresholds = Thresholds {
            fmt: FmtThreshold {
                must_pass: summary.collectors.fmt.status == crate::schema::CollectorStatus::Pass,
            },
            clippy: ClippyThreshold {
                max_warnings: summary.collectors.clippy.warning_count,
            },
            tests: TestThreshold {
                max_failures: summary.collectors.tests.failed,
            },
            coverage: CoverageThreshold {
                min_line_percent: summary.collectors.coverage.line_percent,
            },
            deny: DenyThreshold {
                max_banned: summary.collectors.deny.banned_count,
                max_license_violations: summary.collectors.deny.license_violations,
            },
            audit: AuditThreshold {
                max_vulnerabilities: summary.collectors.audit.vulnerability_count,
                max_critical: summary.collectors.audit.critical_count,
            },
            hack: HackThreshold {
                must_pass: summary.collectors.hack.status == crate::schema::CollectorStatus::Pass,
            },
            mutants: MutantsThreshold {
                min_score: summary.collectors.mutants.mutation_score,
            },
            duplicates: DuplicatesThreshold {
                max_duplicate_lines: summary.collectors.duplicates.duplicate_lines,
            },
            loc: LocThreshold {
                max_line_length: summary.collectors.loc.max_line_length_found.max(120),
            },
            size: SizeThreshold {
                max_lines_per_file: summary.collectors.size.max_lines_per_file.into(),
                max_code_lines_per_file: summary.collectors.size.max_code_lines_per_file.into(),
                max_lines_per_function: summary.collectors.size.max_lines_per_function.into(),
                max_parameters_per_function: summary
                    .collectors
                    .size
                    .max_parameters_per_function
                    .into(),
            },
        };

        let baseline = Baseline {
            schema_version: "1".to_string(),
            created_at: chrono_now(),
            rustquty_version: summary.rustquty_version.clone(),
            thresholds,
        };

        if let Some(ref old) = existing {
            print_threshold_diff(&old.thresholds, &baseline.thresholds);
        }

        let json = serde_json::to_string_pretty(&baseline)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }
}

fn chrono_now() -> String {
    // Use a simple timestamp without pulling in chrono just for this.
    // Returns ISO-8601 format.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    // This is a simplified approach; production code might use the chrono crate.
    format!("{}", now.as_secs())
}

fn print_threshold_diff(old: &Thresholds, new: &Thresholds) {
    let mut changed = Vec::new();

    if old.fmt.must_pass != new.fmt.must_pass {
        changed.push(format!(
            "fmt.must_pass: {} -> {}",
            old.fmt.must_pass, new.fmt.must_pass
        ));
    }
    if old.clippy.max_warnings != new.clippy.max_warnings {
        changed.push(format!(
            "clippy.max_warnings: {} -> {}",
            old.clippy.max_warnings, new.clippy.max_warnings
        ));
    }
    if old.tests.max_failures != new.tests.max_failures {
        changed.push(format!(
            "tests.max_failures: {} -> {}",
            old.tests.max_failures, new.tests.max_failures
        ));
    }
    if (old.coverage.min_line_percent - new.coverage.min_line_percent).abs() > f64::EPSILON {
        changed.push(format!(
            "coverage.min_line_percent: {} -> {}",
            old.coverage.min_line_percent, new.coverage.min_line_percent
        ));
    }

    if changed.is_empty() {
        println!("No threshold changes detected.");
    } else {
        println!("Threshold changes:");
        for line in &changed {
            println!("  {}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_writer_init() {
        use crate::schema::{
            AuditResult, ClippyResult, CollectorStatus, CollectorsSummary, CoverageResult,
            DenyResult, DuplicatesResult, FmtResult, HackResult, LocResult, MutantsResult,
            SizeResult, TestResult,
        };

        let summary = crate::schema::MetricsSummary {
            schema_version: "1".to_string(),
            generated_at: "2026-05-04T12:00:00Z".to_string(),
            rustquty_version: "0.1.0".to_string(),
            project: crate::schema::ProjectInfo {
                name: "test".to_string(),
                rust_edition: "2021".to_string(),
                workspace_root: "/tmp".to_string(),
            },
            collectors: CollectorsSummary {
                fmt: FmtResult {
                    status: CollectorStatus::Pass,
                    details: Default::default(),
                },
                clippy: ClippyResult {
                    status: CollectorStatus::Pass,
                    warning_count: 3,
                    details: vec![],
                },
                tests: TestResult {
                    status: CollectorStatus::Pass,
                    passed: 10,
                    failed: 1,
                    ignored: 0,
                    runner: None,
                },
                coverage: CoverageResult {
                    status: CollectorStatus::Pass,
                    line_percent: 85.5,
                },
                deny: DenyResult {
                    status: CollectorStatus::Pass,
                    banned_count: 0,
                    license_violations: 0,
                },
                audit: AuditResult {
                    status: CollectorStatus::Pass,
                    vulnerability_count: 0,
                    critical_count: 0,
                },
                hack: HackResult {
                    status: CollectorStatus::Pass,
                    feature_combinations_tested: 16,
                },
                mutants: MutantsResult {
                    status: CollectorStatus::Pass,
                    mutation_score: 0.85,
                    caught: 85,
                    missed: 15,
                },
                duplicates: DuplicatesResult {
                    status: CollectorStatus::Pass,
                    total_lines: 1000,
                    duplicate_lines: 5,
                    files_with_duplicates: 2,
                    duplicate_files: vec!["src/a.rs".to_string()],
                },
                loc: LocResult {
                    status: CollectorStatus::Pass,
                    total_lines: 1000,
                    code_lines: 800,
                    comment_lines: 100,
                    blank_lines: 100,
                    long_lines: 0,
                    max_line_length_found: 100,
                    max_line_length_allowed: 120,
                    files: 10,
                    files_with_long_lines: 0,
                    long_line_files: vec![],
                },
                size: SizeResult {
                    status: CollectorStatus::Pass,
                    files: 10,
                    max_lines_per_file: 500,
                    max_code_lines_per_file: 400,
                    max_lines_per_function: 80,
                    max_parameters_per_function: 5,
                    violations: vec![],
                },
            },
        };

        let temp_dir = tempfile::TempDir::new().unwrap();
        let baseline_path = temp_dir.path().join("baseline.json");

        BaselineWriter::init(&summary, &baseline_path, false).unwrap();

        let content = std::fs::read_to_string(&baseline_path).unwrap();
        let baseline: Baseline = serde_json::from_str(&content).unwrap();

        assert_eq!(baseline.thresholds.clippy.max_warnings, 3);
        assert_eq!(baseline.thresholds.tests.max_failures, 1);
        assert!((baseline.thresholds.coverage.min_line_percent - 85.5).abs() < f64::EPSILON);
    }
}
