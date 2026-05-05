//! Gate logic — compare metrics against baseline.

use crate::schema::{
    Baseline, GateResult, MetricsSummary, QualityReport, ReportSummary, Violation,
};

pub struct Gate;

impl Gate {
    /// Compare a metrics summary against a baseline and produce a quality report.
    pub fn run(summary: &MetricsSummary, baseline: &Baseline) -> QualityReport {
        let mut violations = Vec::new();
        let mut collectors_passed = 0u32;
        let mut collectors_failed = 0u32;
        let mut collectors_skipped = 0u32;

        let t = &baseline.thresholds;

        // Fmt
        match summary.collectors.fmt.status {
            crate::schema::CollectorStatus::Pass => collectors_passed += 1,
            crate::schema::CollectorStatus::Fail => {
                collectors_failed += 1;
                if t.fmt.must_pass {
                    violations.push(Violation {
                        collector: "fmt".to_string(),
                        metric: "status".to_string(),
                        baseline_value: serde_json::json!(true),
                        current_value: serde_json::json!("fail"),
                        message: "fmt check failed".to_string(),
                    });
                }
            }
            crate::schema::CollectorStatus::Skipped => collectors_skipped += 1,
            crate::schema::CollectorStatus::Error => collectors_skipped += 1,
        }

        // Clippy
        let clippy_pass = summary.collectors.clippy.warning_count <= t.clippy.max_warnings;
        if clippy_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "clippy".to_string(),
                metric: "warning_count".to_string(),
                baseline_value: serde_json::json!(t.clippy.max_warnings),
                current_value: serde_json::json!(summary.collectors.clippy.warning_count),
                message: format!(
                    "clippy warnings ({}) exceed max allowed ({})",
                    summary.collectors.clippy.warning_count, t.clippy.max_warnings
                ),
            });
        }

        // Tests
        let tests_pass = summary.collectors.tests.failed <= t.tests.max_failures;
        if tests_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "tests".to_string(),
                metric: "failed".to_string(),
                baseline_value: serde_json::json!(t.tests.max_failures),
                current_value: serde_json::json!(summary.collectors.tests.failed),
                message: format!(
                    "test failures ({}) exceed max allowed ({})",
                    summary.collectors.tests.failed, t.tests.max_failures
                ),
            });
        }

        // Coverage
        let coverage_pass = summary.collectors.coverage.line_percent >= t.coverage.min_line_percent;
        if coverage_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "coverage".to_string(),
                metric: "line_percent".to_string(),
                baseline_value: serde_json::json!(t.coverage.min_line_percent),
                current_value: serde_json::json!(summary.collectors.coverage.line_percent),
                message: format!(
                    "coverage ({:.1}%) below minimum ({:.1}%)",
                    summary.collectors.coverage.line_percent, t.coverage.min_line_percent
                ),
            });
        }

        // Deny
        let deny_pass = summary.collectors.deny.banned_count <= t.deny.max_banned
            && summary.collectors.deny.license_violations <= t.deny.max_license_violations;
        if deny_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "deny".to_string(),
                metric: "banned_count + license_violations".to_string(),
                baseline_value: serde_json::json!({
                    "max_banned": t.deny.max_banned,
                    "max_license_violations": t.deny.max_license_violations
                }),
                current_value: serde_json::json!({
                    "banned_count": summary.collectors.deny.banned_count,
                    "license_violations": summary.collectors.deny.license_violations
                }),
                message: format!(
                    "deny check failed: {} banned, {} license violations",
                    summary.collectors.deny.banned_count,
                    summary.collectors.deny.license_violations
                ),
            });
        }

        // Audit
        let audit_pass = summary.collectors.audit.vulnerability_count
            <= t.audit.max_vulnerabilities
            && summary.collectors.audit.critical_count <= t.audit.max_critical;
        if audit_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "audit".to_string(),
                metric: "vulnerability_count + critical_count".to_string(),
                baseline_value: serde_json::json!({
                    "max_vulnerabilities": t.audit.max_vulnerabilities,
                    "max_critical": t.audit.max_critical
                }),
                current_value: serde_json::json!({
                    "vulnerability_count": summary.collectors.audit.vulnerability_count,
                    "critical_count": summary.collectors.audit.critical_count
                }),
                message: format!(
                    "audit found {} vulnerabilities ({} critical), exceeds baseline",
                    summary.collectors.audit.vulnerability_count,
                    summary.collectors.audit.critical_count
                ),
            });
        }

        // Hack
        match summary.collectors.hack.status {
            crate::schema::CollectorStatus::Pass => collectors_passed += 1,
            crate::schema::CollectorStatus::Fail => {
                collectors_failed += 1;
                if t.hack.must_pass {
                    violations.push(Violation {
                        collector: "hack".to_string(),
                        metric: "status".to_string(),
                        baseline_value: serde_json::json!(true),
                        current_value: serde_json::json!("fail"),
                        message: "cargo hack check failed".to_string(),
                    });
                }
            }
            crate::schema::CollectorStatus::Skipped => collectors_skipped += 1,
            crate::schema::CollectorStatus::Error => collectors_skipped += 1,
        }

        // Mutants
        let mutants_pass = summary.collectors.mutants.mutation_score >= t.mutants.min_score;
        if mutants_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "mutants".to_string(),
                metric: "mutation_score".to_string(),
                baseline_value: serde_json::json!(t.mutants.min_score),
                current_value: serde_json::json!(summary.collectors.mutants.mutation_score),
                message: format!(
                    "mutation score ({:.2}) below minimum ({:.2})",
                    summary.collectors.mutants.mutation_score, t.mutants.min_score
                ),
            });
        }

        // Duplicates
        let duplicates_pass =
            summary.collectors.duplicates.duplicate_lines <= t.duplicates.max_duplicate_lines;
        if duplicates_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "duplicates".to_string(),
                metric: "duplicate_lines".to_string(),
                baseline_value: serde_json::json!(t.duplicates.max_duplicate_lines),
                current_value: serde_json::json!(summary.collectors.duplicates.duplicate_lines),
                message: format!(
                    "duplicate lines ({}) exceed maximum ({})",
                    summary.collectors.duplicates.duplicate_lines, t.duplicates.max_duplicate_lines
                ),
            });
        }

        // LOC
        let loc_pass = summary.collectors.loc.long_lines == 0;
        if loc_pass {
            collectors_passed += 1;
        } else {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "loc".to_string(),
                metric: "long_lines".to_string(),
                baseline_value: serde_json::json!(0),
                current_value: serde_json::json!(summary.collectors.loc.long_lines),
                message: format!(
                    "{} lines exceed max length ({})",
                    summary.collectors.loc.long_lines, t.loc.max_line_length
                ),
            });
        }

        // Size
        // Gate passes if size is not configured in baseline or if no violations.
        let size_has_thresholds = t.size.max_lines_per_file.is_some()
            || t.size.max_code_lines_per_file.is_some()
            || t.size.max_lines_per_function.is_some()
            || t.size.max_parameters_per_function.is_some();

        if size_has_thresholds && !summary.collectors.size.violations.is_empty() {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "size".to_string(),
                metric: "violations".to_string(),
                baseline_value: serde_json::json!(0),
                current_value: serde_json::json!(summary.collectors.size.violations.len()),
                message: format!(
                    "{} size violation(s) detected",
                    summary.collectors.size.violations.len()
                ),
            });
        } else {
            collectors_passed += 1;
        }

        // Complexity
        // Gate passes if complexity is not configured or if no violations.
        let complexity_has_thresholds = t.complexity.max_cyclomatic_per_function.is_some()
            || t.complexity.max_nesting_depth.is_some();

        if complexity_has_thresholds && !summary.collectors.complexity.violations.is_empty() {
            collectors_failed += 1;
            violations.push(Violation {
                collector: "complexity".to_string(),
                metric: "violations".to_string(),
                baseline_value: serde_json::json!(0),
                current_value: serde_json::json!(summary.collectors.complexity.violations.len()),
                message: format!(
                    "{} complexity violation(s) detected",
                    summary.collectors.complexity.violations.len()
                ),
            });
        } else {
            collectors_passed += 1;
        }

        let collectors_run = collectors_passed + collectors_failed;
        let gate_result = if violations.is_empty() {
            GateResult::Pass
        } else {
            GateResult::Fail
        };

        QualityReport {
            schema_version: "1".to_string(),
            generated_at: chrono_now(),
            gate_result,
            violations,
            summary: ReportSummary {
                collectors_run,
                collectors_passed,
                collectors_failed,
                collectors_skipped,
            },
        }
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;

    #[allow(clippy::too_many_arguments)]
    fn make_summary(
        fmt_status: CollectorStatus,
        clippy_warnings: u32,
        test_failed: u32,
        line_percent: f64,
        deny_banned: u32,
        deny_license: u32,
        vuln_count: u32,
        critical_count: u32,
        hack_status: CollectorStatus,
        mutation_score: f64,
    ) -> MetricsSummary {
        MetricsSummary {
            schema_version: "1".to_string(),
            generated_at: "2026-05-04T12:00:00Z".to_string(),
            rustquty_version: "0.1.0".to_string(),
            project: ProjectInfo {
                name: "test".to_string(),
                rust_edition: "2021".to_string(),
                workspace_root: "/tmp".to_string(),
            },
            collectors: CollectorsSummary {
                fmt: FmtResult {
                    status: fmt_status,
                    details: Default::default(),
                },
                clippy: ClippyResult {
                    status: if clippy_warnings == 0 {
                        CollectorStatus::Pass
                    } else {
                        CollectorStatus::Fail
                    },
                    warning_count: clippy_warnings,
                    details: vec![],
                },
                tests: TestResult {
                    status: if test_failed == 0 {
                        CollectorStatus::Pass
                    } else {
                        CollectorStatus::Fail
                    },
                    passed: 10,
                    failed: test_failed,
                    ignored: 0,
                    runner: None,
                },
                coverage: CoverageResult {
                    status: CollectorStatus::Pass,
                    line_percent,
                },
                deny: DenyResult {
                    status: CollectorStatus::Pass,
                    banned_count: deny_banned,
                    license_violations: deny_license,
                },
                audit: AuditResult {
                    status: if vuln_count == 0 {
                        CollectorStatus::Pass
                    } else {
                        CollectorStatus::Fail
                    },
                    vulnerability_count: vuln_count,
                    critical_count,
                },
                hack: HackResult {
                    status: hack_status,
                    feature_combinations_tested: 8,
                },
                mutants: MutantsResult {
                    status: if mutation_score >= 0.8 {
                        CollectorStatus::Pass
                    } else {
                        CollectorStatus::Fail
                    },
                    mutation_score,
                    caught: 80,
                    missed: 20,
                },
                duplicates: DuplicatesResult {
                    status: CollectorStatus::Pass,
                    total_lines: 1000,
                    duplicate_lines: 0,
                    files_with_duplicates: 0,
                    duplicate_files: vec![],
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
                complexity: ComplexityResult {
                    status: CollectorStatus::Pass,
                    functions: 10,
                    max_cyclomatic_complexity: 5,
                    max_nesting_depth: 3,
                    complex_functions: 0,
                    violations: vec![],
                },
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_baseline(
        fmt_must_pass: bool,
        max_clippy: u32,
        max_failures: u32,
        min_coverage: f64,
        max_banned: u32,
        max_license: u32,
        max_vuln: u32,
        max_critical: u32,
        hack_must_pass: bool,
        min_score: f64,
        max_duplicate_lines: u32,
        max_line_length: usize,
        size_max_lines_per_file: Option<u32>,
        size_max_code_lines_per_file: Option<u32>,
        size_max_lines_per_function: Option<u32>,
        size_max_parameters_per_function: Option<u32>,
    ) -> Baseline {
        Baseline {
            schema_version: "1".to_string(),
            created_at: "2026-05-04T00:00:00Z".to_string(),
            rustquty_version: "0.1.0".to_string(),
            thresholds: Thresholds {
                fmt: FmtThreshold {
                    must_pass: fmt_must_pass,
                },
                clippy: ClippyThreshold {
                    max_warnings: max_clippy,
                },
                tests: TestThreshold { max_failures },
                coverage: CoverageThreshold {
                    min_line_percent: min_coverage,
                },
                deny: DenyThreshold {
                    max_banned,
                    max_license_violations: max_license,
                },
                audit: AuditThreshold {
                    max_vulnerabilities: max_vuln,
                    max_critical,
                },
                hack: HackThreshold {
                    must_pass: hack_must_pass,
                },
                mutants: MutantsThreshold { min_score },
                duplicates: DuplicatesThreshold {
                    max_duplicate_lines,
                },
                loc: LocThreshold { max_line_length },
                size: SizeThreshold {
                    max_lines_per_file: size_max_lines_per_file,
                    max_code_lines_per_file: size_max_code_lines_per_file,
                    max_lines_per_function: size_max_lines_per_function,
                    max_parameters_per_function: size_max_parameters_per_function,
                },
                complexity: ComplexityThreshold {
                    max_cyclomatic_per_function: None,
                    max_nesting_depth: None,
                },
            },
        }
    }

    #[test]
    fn test_gate_passes_when_all_metrics_within_baseline() {
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            90.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        assert!(matches!(report.gate_result, GateResult::Pass));
        assert!(report.violations.is_empty());
        assert_eq!(report.summary.collectors_passed, 12);
        assert_eq!(report.summary.collectors_failed, 0);
    }

    #[test]
    fn test_gate_fails_when_clippy_exceeds_baseline() {
        let summary = make_summary(
            CollectorStatus::Pass,
            5,
            0,
            90.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        assert!(matches!(report.gate_result, GateResult::Fail));
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].collector, "clippy");
    }

    #[test]
    fn test_equal_values_do_not_fail() {
        // Edge case: equal values should NOT fail (ratchet model)
        let summary = make_summary(
            CollectorStatus::Pass,
            3,
            1,
            85.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.8,
        );
        let baseline = make_baseline(
            true, 3, 1, 85.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        assert!(matches!(report.gate_result, GateResult::Pass));
    }

    #[test]
    fn test_gate_fails_when_loc_exceeds_max_line_length() {
        let mut summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            90.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        summary.collectors.loc.long_lines = 5;
        summary.collectors.loc.status = CollectorStatus::Fail;

        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        assert!(matches!(report.gate_result, GateResult::Fail));
        assert!(report.violations.iter().any(|v| v.collector == "loc"));
    }

    #[test]
    fn test_size_gate_passes_without_size_thresholds() {
        // Without size thresholds configured, size should always pass.
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            90.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        assert!(matches!(report.gate_result, GateResult::Pass));
    }

    #[test]
    fn test_size_gate_fails_with_violations_and_threshold() {
        // With size thresholds configured, violations should fail the gate.
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            90.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        let baseline = make_baseline(
            true,
            0,
            0,
            80.0,
            0,
            0,
            0,
            0,
            true,
            0.8,
            100,
            120,
            Some(500),
            Some(400),
            Some(80),
            Some(5),
        );
        let report = Gate::run(&summary, &baseline);
        // Summary has size with violations=0, so it should pass.
        assert!(matches!(report.gate_result, GateResult::Pass));
    }
}
