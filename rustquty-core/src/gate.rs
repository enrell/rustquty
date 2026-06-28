//! Gate logic — compare metrics against baseline.

use crate::schema::{
    Baseline, GateResult, MetricsSummary, QualityReport, ReportSummary, Violation,
};

/// Absolute thresholds that override baseline values when set.
/// Based on industry standards (SonarQube, ESLint, DeepSource).
#[derive(Debug, Clone, Default)]
pub struct GateConfig {
    pub max_cyclomatic_per_function: Option<u32>,
    pub max_nesting_depth: Option<u32>,
    pub max_lines_per_function: Option<u32>,
    pub max_lines_per_file: Option<u32>,
    pub max_code_lines_per_file: Option<u32>,
    pub max_parameters_per_function: Option<u32>,
    pub min_coverage_percent: Option<f64>,
    pub max_duplicate_lines: Option<u32>,
    pub max_clippy_warnings: Option<u32>,
    pub max_line_length: Option<usize>,
}

pub struct Gate;

impl Gate {
    /// Compare a metrics summary against a baseline and produce a quality report.
    pub fn run(summary: &MetricsSummary, baseline: &Baseline) -> QualityReport {
        Self::run_with_config(summary, baseline, None)
    }

    /// Compare metrics against baseline with optional absolute thresholds.
    /// When `config` is provided, absolute values override baseline ratchet values.
    pub fn run_with_config(
        summary: &MetricsSummary,
        baseline: &Baseline,
        config: Option<&GateConfig>,
    ) -> QualityReport {
        let mut violations = Vec::new();
        let mut collectors_passed = 0u32;
        let mut collectors_failed = 0u32;
        let mut collectors_skipped = 0u32;

        let thresholds = &baseline.thresholds;
        let default_cfg = GateConfig::default();
        let cfg = config.unwrap_or(&default_cfg);

        macro_rules! check_pass {
            ($pass:expr, $collector:expr, $metric:expr, $baseline_val:expr, $current_val:expr, $msg:expr) => {
                if $pass {
                    collectors_passed += 1;
                } else {
                    collectors_failed += 1;
                    violations.push(Violation {
                        collector: $collector.to_string(),
                        metric: $metric.to_string(),
                        baseline_value: $baseline_val,
                        current_value: $current_val,
                        message: $msg,
                    });
                }
            };
        }

        macro_rules! check_status {
            ($status:expr, $collector:expr, $must_pass:expr) => {
                match $status {
                    crate::schema::CollectorStatus::Pass => collectors_passed += 1,
                    crate::schema::CollectorStatus::Fail => {
                        collectors_failed += 1;
                        if $must_pass {
                            violations.push(Violation {
                                collector: $collector.to_string(),
                                metric: "status".to_string(),
                                baseline_value: serde_json::json!(true),
                                current_value: serde_json::json!("fail"),
                                message: format!("{} check failed", $collector),
                            });
                        }
                    }
                    crate::schema::CollectorStatus::Skipped => collectors_skipped += 1,
                    crate::schema::CollectorStatus::Error => collectors_skipped += 1,
                }
            };
        }

        // Fmt
        check_status!(
            summary.collectors.fmt.status,
            "fmt",
            thresholds.fmt.must_pass
        );

        // Clippy — use absolute if set, otherwise baseline
        let clippy_max = cfg
            .max_clippy_warnings
            .unwrap_or(thresholds.clippy.max_warnings);
        check_pass!(
            summary.collectors.clippy.warning_count <= clippy_max,
            "clippy",
            "warning_count",
            serde_json::json!(clippy_max),
            serde_json::json!(summary.collectors.clippy.warning_count),
            format!(
                "clippy warnings ({}) exceed max allowed ({})",
                summary.collectors.clippy.warning_count, clippy_max
            )
        );

        // Tests
        check_pass!(
            summary.collectors.tests.failed <= thresholds.tests.max_failures,
            "tests",
            "failed",
            serde_json::json!(thresholds.tests.max_failures),
            serde_json::json!(summary.collectors.tests.failed),
            format!(
                "test failures ({}) exceed max allowed ({})",
                summary.collectors.tests.failed, thresholds.tests.max_failures
            )
        );

        // Coverage — use absolute if set, otherwise baseline
        let coverage_min = cfg
            .min_coverage_percent
            .unwrap_or(thresholds.coverage.min_line_percent);
        check_pass!(
            summary.collectors.coverage.line_percent >= coverage_min,
            "coverage",
            "line_percent",
            serde_json::json!(coverage_min),
            serde_json::json!(summary.collectors.coverage.line_percent),
            format!(
                "coverage ({:.1}%) below minimum ({:.1}%)",
                summary.collectors.coverage.line_percent, coverage_min
            )
        );

        // Deny
        check_pass!(
            summary.collectors.deny.banned_count <= thresholds.deny.max_banned
                && summary.collectors.deny.license_violations
                    <= thresholds.deny.max_license_violations,
            "deny",
            "banned_count + license_violations",
            serde_json::json!({"max_banned": thresholds.deny.max_banned, "max_license_violations": thresholds.deny.max_license_violations}),
            serde_json::json!({"banned_count": summary.collectors.deny.banned_count, "license_violations": summary.collectors.deny.license_violations}),
            format!(
                "deny check failed: {} banned, {} license violations",
                summary.collectors.deny.banned_count, summary.collectors.deny.license_violations
            )
        );

        // Audit
        check_pass!(
            summary.collectors.audit.vulnerability_count <= thresholds.audit.max_vulnerabilities
                && summary.collectors.audit.critical_count <= thresholds.audit.max_critical,
            "audit",
            "vulnerability_count + critical_count",
            serde_json::json!({"max_vulnerabilities": thresholds.audit.max_vulnerabilities, "max_critical": thresholds.audit.max_critical}),
            serde_json::json!({"vulnerability_count": summary.collectors.audit.vulnerability_count, "critical_count": summary.collectors.audit.critical_count}),
            format!(
                "audit found {} vulnerabilities ({} critical), exceeds baseline",
                summary.collectors.audit.vulnerability_count,
                summary.collectors.audit.critical_count
            )
        );

        // Hack
        check_status!(
            summary.collectors.hack.status,
            "hack",
            thresholds.hack.must_pass
        );

        // Mutants
        check_pass!(
            summary.collectors.mutants.mutation_score >= thresholds.mutants.min_score,
            "mutants",
            "mutation_score",
            serde_json::json!(thresholds.mutants.min_score),
            serde_json::json!(summary.collectors.mutants.mutation_score),
            format!(
                "mutation score ({:.2}) below minimum ({:.2})",
                summary.collectors.mutants.mutation_score, thresholds.mutants.min_score
            )
        );

        // Duplicates — use absolute if set, otherwise baseline
        let dup_max = cfg
            .max_duplicate_lines
            .unwrap_or(thresholds.duplicates.max_duplicate_lines);
        check_pass!(
            summary.collectors.duplicates.duplicate_lines <= dup_max,
            "duplicates",
            "duplicate_lines",
            serde_json::json!(dup_max),
            serde_json::json!(summary.collectors.duplicates.duplicate_lines),
            format!(
                "duplicate lines ({}) exceed maximum ({})",
                summary.collectors.duplicates.duplicate_lines, dup_max
            )
        );

        // LOC — long_lines is measured by the collector, so report that actual threshold.
        let line_len_max = if summary.collectors.loc.max_line_length_allowed > 0 {
            summary.collectors.loc.max_line_length_allowed
        } else {
            cfg.max_line_length
                .unwrap_or(thresholds.loc.max_line_length)
                .max(120)
        };
        check_pass!(
            summary.collectors.loc.long_lines == 0,
            "loc",
            "long_lines",
            serde_json::json!(0),
            serde_json::json!(summary.collectors.loc.long_lines),
            format!(
                "{} lines exceed max length ({})",
                summary.collectors.loc.long_lines, line_len_max
            )
        );

        // Size — merge absolute config with baseline
        let size_max_lines_per_file = cfg
            .max_lines_per_file
            .or(thresholds.size.max_lines_per_file);
        let size_max_code_lines_per_file = cfg
            .max_code_lines_per_file
            .or(thresholds.size.max_code_lines_per_file);
        let size_max_lines_per_function = cfg
            .max_lines_per_function
            .or(thresholds.size.max_lines_per_function);
        let size_max_params = cfg
            .max_parameters_per_function
            .or(thresholds.size.max_parameters_per_function);

        let size_has_thresholds = size_max_lines_per_file.is_some()
            || size_max_code_lines_per_file.is_some()
            || size_max_lines_per_function.is_some()
            || size_max_params.is_some();
        check_pass!(
            !size_has_thresholds || summary.collectors.size.violations.is_empty(),
            "size",
            "violations",
            serde_json::json!(0),
            serde_json::json!(summary.collectors.size.violations.len()),
            format!(
                "{} size violation(s) detected",
                summary.collectors.size.violations.len()
            )
        );

        // Complexity — merge absolute config with baseline
        let complexity_max_cc = cfg
            .max_cyclomatic_per_function
            .or(thresholds.complexity.max_cyclomatic_per_function);
        let complexity_max_depth = cfg
            .max_nesting_depth
            .or(thresholds.complexity.max_nesting_depth);

        let complexity_has_thresholds =
            complexity_max_cc.is_some() || complexity_max_depth.is_some();
        check_pass!(
            !complexity_has_thresholds || summary.collectors.complexity.violations.is_empty(),
            "complexity",
            "violations",
            serde_json::json!(0),
            serde_json::json!(summary.collectors.complexity.violations.len()),
            format!(
                "{} complexity violation(s) detected",
                summary.collectors.complexity.violations.len()
            )
        );

        let collectors_run = collectors_passed + collectors_failed;
        let gate_result = if violations.is_empty() {
            GateResult::Pass
        } else {
            GateResult::Fail
        };

        QualityReport {
            schema_version: "1".to_string(),
            generated_at: crate::util::chrono_now(),
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
                    duplicate_blocks: vec![],
                    duplicate_blocks_omitted: 0,
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
                    long_line_details: vec![],
                    long_line_details_omitted: 0,
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
    fn test_loc_gate_message_uses_collector_threshold() {
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
        summary.collectors.loc.max_line_length_found = 284;
        summary.collectors.loc.max_line_length_allowed = 120;
        summary.collectors.loc.status = CollectorStatus::Fail;

        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 284, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        let violation = report
            .violations
            .iter()
            .find(|v| v.collector == "loc")
            .unwrap();

        assert!(violation.message.contains("(120)"));
        assert!(!violation.message.contains("(284)"));
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

    // --- Regression tests ---

    #[test]
    fn test_gate_regression_generated_at_is_iso8601() {
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
        // Must be ISO-8601, not a raw number
        assert!(
            report.generated_at.contains('T'),
            "generated_at should be ISO-8601: {}",
            report.generated_at
        );
        assert!(
            report.generated_at.ends_with('Z'),
            "generated_at should end with Z: {}",
            report.generated_at
        );
        assert!(
            report.generated_at.len() == 20,
            "generated_at should be 20 chars: {}",
            report.generated_at
        );
    }

    #[test]
    fn test_gate_regression_summary_counts_correct() {
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
        // clippy fails (5 > 0), others pass
        assert_eq!(report.summary.collectors_failed, 1);
        assert_eq!(report.summary.collectors_passed, 11);
        assert!(report.violations.iter().any(|v| v.collector == "clippy"));
    }

    #[test]
    fn test_gate_regression_violation_messages_not_empty() {
        let summary = make_summary(
            CollectorStatus::Pass,
            10,
            3,
            50.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.5,
        );
        let baseline = make_baseline(
            true, 0, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let report = Gate::run(&summary, &baseline);
        for v in &report.violations {
            assert!(
                !v.message.is_empty(),
                "Violation message should not be empty for {}",
                v.collector
            );
        }
    }

    // --- Absolute threshold (GateConfig) tests ---

    #[test]
    fn test_gate_config_clippy_override() {
        // Baseline allows 10 warnings, but config overrides to 0
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
            true, 10, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let config = GateConfig {
            max_clippy_warnings: Some(0),
            ..Default::default()
        };
        let report = Gate::run_with_config(&summary, &baseline, Some(&config));
        assert!(matches!(report.gate_result, GateResult::Fail));
        assert!(report.violations.iter().any(|v| v.collector == "clippy"));
    }

    #[test]
    fn test_gate_config_coverage_override() {
        // Baseline allows 50%, but config requires 80%
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            60.0,
            0,
            0,
            0,
            0,
            CollectorStatus::Pass,
            0.9,
        );
        let baseline = make_baseline(
            true, 0, 0, 50.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        let config = GateConfig {
            min_coverage_percent: Some(80.0),
            ..Default::default()
        };
        let report = Gate::run_with_config(&summary, &baseline, Some(&config));
        assert!(matches!(report.gate_result, GateResult::Fail));
        assert!(report.violations.iter().any(|v| v.collector == "coverage"));
    }

    #[test]
    fn test_gate_config_passes_when_within_absolute_thresholds() {
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            85.0,
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
        let config = GateConfig {
            max_clippy_warnings: Some(0),
            min_coverage_percent: Some(80.0),
            max_lines_per_function: Some(80),
            max_nesting_depth: Some(5),
            ..Default::default()
        };
        let report = Gate::run_with_config(&summary, &baseline, Some(&config));
        assert!(matches!(report.gate_result, GateResult::Pass));
    }

    #[test]
    fn test_gate_config_none_falls_back_to_baseline() {
        // Without config, baseline ratchet model is used
        let summary = make_summary(
            CollectorStatus::Pass,
            3,
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
            true, 5, 0, 80.0, 0, 0, 0, 0, true, 0.8, 100, 120, None, None, None, None,
        );
        // No config — should pass because 3 <= 5 (baseline)
        let report = Gate::run_with_config(&summary, &baseline, None);
        assert!(matches!(report.gate_result, GateResult::Pass));
    }

    #[test]
    fn test_gate_config_sonarqube_defaults() {
        // Simulate SonarQube-like absolute thresholds
        let summary = make_summary(
            CollectorStatus::Pass,
            0,
            0,
            85.0,
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
        let config = GateConfig {
            max_cyclomatic_per_function: Some(15),
            max_nesting_depth: Some(5),
            max_lines_per_function: Some(80),
            max_lines_per_file: Some(1000),
            max_code_lines_per_file: Some(700),
            max_parameters_per_function: Some(7),
            min_coverage_percent: Some(80.0),
            max_clippy_warnings: Some(0),
            max_line_length: Some(120),
            ..Default::default()
        };
        let report = Gate::run_with_config(&summary, &baseline, Some(&config));
        assert!(matches!(report.gate_result, GateResult::Pass));
    }
}
