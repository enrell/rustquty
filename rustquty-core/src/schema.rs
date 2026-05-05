//! JSON schemas for rustquty data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ------------------------------------------------------------------------------------------------
// MetricsSummary
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSummary {
    pub schema_version: String,
    pub generated_at: String,
    pub rustquty_version: String,
    pub project: ProjectInfo,
    pub collectors: CollectorsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub rust_edition: String,
    pub workspace_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorsSummary {
    pub fmt: FmtResult,
    pub clippy: ClippyResult,
    pub tests: TestResult,
    pub coverage: CoverageResult,
    pub deny: DenyResult,
    pub audit: AuditResult,
    pub hack: HackResult,
    pub mutants: MutantsResult,
    pub duplicates: DuplicatesResult,
    pub loc: LocResult,
    pub size: SizeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FmtResult {
    pub status: CollectorStatus,
    #[serde(default)]
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClippyResult {
    pub status: CollectorStatus,
    pub warning_count: u32,
    #[serde(default)]
    pub details: Vec<ClippyLint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClippyLint {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub status: CollectorStatus,
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
    #[serde(default)]
    pub runner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageResult {
    pub status: CollectorStatus,
    pub line_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DenyResult {
    pub status: CollectorStatus,
    pub banned_count: u32,
    pub license_violations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditResult {
    pub status: CollectorStatus,
    pub vulnerability_count: u32,
    pub critical_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HackResult {
    pub status: CollectorStatus,
    pub feature_combinations_tested: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MutantsResult {
    pub status: CollectorStatus,
    pub mutation_score: f64,
    pub caught: u32,
    pub missed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesResult {
    pub status: CollectorStatus,
    pub total_lines: u32,
    pub duplicate_lines: u32,
    #[serde(default)]
    pub files_with_duplicates: u32,
    #[serde(default)]
    pub duplicate_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocResult {
    pub status: CollectorStatus,
    pub total_lines: u32,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub blank_lines: u32,
    pub long_lines: u32,
    pub max_line_length_found: usize,
    pub max_line_length_allowed: usize,
    pub files: u32,
    #[serde(default)]
    pub files_with_long_lines: u32,
    #[serde(default)]
    pub long_line_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SizeResult {
    pub status: CollectorStatus,
    pub files: u32,
    #[serde(default)]
    pub max_lines_per_file: u32,
    #[serde(default)]
    pub max_code_lines_per_file: u32,
    #[serde(default)]
    pub max_lines_per_function: u32,
    #[serde(default)]
    pub max_parameters_per_function: u32,
    #[serde(default)]
    pub violations: Vec<SizeViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SizeViolation {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    #[serde(default)]
    pub function: Option<String>,
    pub message: String,
    pub actual: u32,
    pub threshold: u32,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CollectorStatus {
    Pass,
    Fail,
    Skipped,
    Error,
}

// ------------------------------------------------------------------------------------------------
// Baseline
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Baseline {
    pub schema_version: String,
    pub created_at: String,
    pub rustquty_version: String,
    pub thresholds: Thresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    pub fmt: FmtThreshold,
    pub clippy: ClippyThreshold,
    pub tests: TestThreshold,
    pub coverage: CoverageThreshold,
    pub deny: DenyThreshold,
    pub audit: AuditThreshold,
    pub hack: HackThreshold,
    pub mutants: MutantsThreshold,
    pub duplicates: DuplicatesThreshold,
    pub loc: LocThreshold,
    pub size: SizeThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FmtThreshold {
    pub must_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClippyThreshold {
    pub max_warnings: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestThreshold {
    pub max_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageThreshold {
    pub min_line_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DenyThreshold {
    pub max_banned: u32,
    pub max_license_violations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditThreshold {
    pub max_vulnerabilities: u32,
    pub max_critical: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HackThreshold {
    pub must_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MutantsThreshold {
    pub min_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesThreshold {
    pub max_duplicate_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocThreshold {
    pub max_line_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SizeThreshold {
    #[serde(default)]
    pub max_lines_per_file: Option<u32>,
    #[serde(default)]
    pub max_code_lines_per_file: Option<u32>,
    #[serde(default)]
    pub max_lines_per_function: Option<u32>,
    #[serde(default)]
    pub max_parameters_per_function: Option<u32>,
}

// ------------------------------------------------------------------------------------------------
// QualityReport
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QualityReport {
    pub schema_version: String,
    pub generated_at: String,
    pub gate_result: GateResult,
    #[serde(default)]
    pub violations: Vec<Violation>,
    pub summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GateResult {
    Pass,
    Fail,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub collector: String,
    pub metric: String,
    pub baseline_value: serde_json::Value,
    pub current_value: serde_json::Value,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummary {
    pub collectors_run: u32,
    pub collectors_passed: u32,
    pub collectors_failed: u32,
    pub collectors_skipped: u32,
}

// ------------------------------------------------------------------------------------------------
// Schema version errors
// ------------------------------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SchemaVersionError {
    #[error("unknown schema version: {0}")]
    UnknownVersion(String),
}

impl MetricsSummary {
    pub fn check_version(&self) -> Result<(), SchemaVersionError> {
        if &self.schema_version != "1" {
            return Err(SchemaVersionError::UnknownVersion(
                self.schema_version.clone(),
            ));
        }
        Ok(())
    }
}

impl Baseline {
    pub fn check_version(&self) -> Result<(), SchemaVersionError> {
        if &self.schema_version != "1" {
            return Err(SchemaVersionError::UnknownVersion(
                self.schema_version.clone(),
            ));
        }
        Ok(())
    }
}

impl QualityReport {
    pub fn check_version(&self) -> Result<(), SchemaVersionError> {
        if &self.schema_version != "1" {
            return Err(SchemaVersionError::UnknownVersion(
                self.schema_version.clone(),
            ));
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// Round-trip tests
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_summary_roundtrip() {
        let json = r#"{
          "schemaVersion": "1",
          "generatedAt": "2026-05-04T12:00:00Z",
          "rustqutyVersion": "0.1.0",
          "project": {
            "name": "test-project",
            "rustEdition": "2021",
            "workspaceRoot": "/path/to/project"
          },
          "collectors": {
            "fmt": { "status": "pass", "details": {} },
            "clippy": { "status": "pass", "warningCount": 0, "details": [] },
            "tests": { "status": "pass", "passed": 10, "failed": 0, "ignored": 0 },
            "coverage": { "status": "skipped", "linePercent": 0.0 },
            "deny": { "status": "skipped", "bannedCount": 0, "licenseViolations": 0 },
            "audit": { "status": "skipped", "vulnerabilityCount": 0, "criticalCount": 0 },
            "hack": { "status": "skipped", "featureCombinationsTested": 0 },
            "mutants": { "status": "skipped", "mutationScore": 0.0, "caught": 0, "missed": 0 },
            "duplicates": { "status": "skipped", "totalLines": 0, "duplicateLines": 0, "filesWithDuplicates": 0, "duplicateFiles": [] },
            "loc": { "status": "skipped", "totalLines": 0, "codeLines": 0, "commentLines": 0, "blankLines": 0, "longLines": 0, "maxLineLengthFound": 0, "maxLineLengthAllowed": 120, "files": 0, "filesWithLongLines": 0, "longLineFiles": [] },
            "size": { "status": "skipped", "files": 0, "maxLinesPerFile": 0, "maxCodeLinesPerFile": 0, "maxLinesPerFunction": 0, "maxParametersPerFunction": 0, "violations": [] }
          }
        }"#;
        let summary: MetricsSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.schema_version, "1");
        let output = serde_json::to_string(&summary).unwrap();
        assert!(output.contains("\"schemaVersion\":\"1\""));
    }

    #[test]
    fn test_baseline_roundtrip() {
        let json = r#"{
          "schemaVersion": "1",
          "createdAt": "2026-05-04T00:00:00Z",
          "rustqutyVersion": "0.1.0",
          "thresholds": {
            "fmt": { "mustPass": true },
            "clippy": { "maxWarnings": 0 },
            "tests": { "maxFailures": 0 },
            "coverage": { "minLinePercent": 80.0 },
            "deny": { "maxBanned": 0, "maxLicenseViolations": 0 },
            "audit": { "maxVulnerabilities": 0, "maxCritical": 0 },
            "hack": { "mustPass": true },
            "mutants": { "minScore": 0.8 },
            "duplicates": { "maxDuplicateLines": 100 },
            "loc": { "maxLineLength": 120 },
            "size": { "maxLinesPerFile": 1000, "maxCodeLinesPerFile": 700, "maxLinesPerFunction": 80, "maxParametersPerFunction": 6 }
          }
        }"#;
        let baseline: Baseline = serde_json::from_str(json).unwrap();
        assert_eq!(baseline.thresholds.coverage.min_line_percent, 80.0);
        let output = serde_json::to_string(&baseline).unwrap();
        assert!(output.contains("\"minLinePercent\":80.0"));
    }

    #[test]
    fn test_quality_report_roundtrip() {
        let json = r#"{
          "schemaVersion": "1",
          "generatedAt": "2026-05-04T12:00:00Z",
          "gateResult": "fail",
          "violations": [
            {
              "collector": "clippy",
              "metric": "warningCount",
              "baselineValue": "0",
              "currentValue": "5",
              "message": "clippy warning count exceeded baseline"
            }
          ],
          "summary": {
            "collectorsRun": 8,
            "collectorsPassed": 7,
            "collectorsFailed": 1,
            "collectorsSkipped": 0
          }
        }"#;
        let report: QualityReport = serde_json::from_str(json).unwrap();
        assert_eq!(report.violations.len(), 1);
        let output = serde_json::to_string(&report).unwrap();
        assert!(output.contains("\"gateResult\":\"fail\""));
    }

    #[test]
    fn test_unknown_schema_version_error() {
        let json = r#"{
          "schemaVersion": "99",
          "generatedAt": "2026-05-04T12:00:00Z",
          "rustqutyVersion": "0.1.0",
          "project": {
            "name": "test",
            "rustEdition": "2021",
            "workspaceRoot": "/path"
          },
          "collectors": {
            "fmt": { "status": "pass", "details": {} },
            "clippy": { "status": "pass", "warningCount": 0, "details": [] },
            "tests": { "status": "pass", "passed": 0, "failed": 0, "ignored": 0 },
            "coverage": { "status": "pass", "linePercent": 0.0 },
            "deny": { "status": "pass", "bannedCount": 0, "licenseViolations": 0 },
            "audit": { "status": "pass", "vulnerabilityCount": 0, "criticalCount": 0 },
            "hack": { "status": "pass", "featureCombinationsTested": 0 },
            "mutants": { "status": "pass", "mutationScore": 0.0, "caught": 0, "missed": 0 },
            "duplicates": { "status": "pass", "totalLines": 1000, "duplicateLines": 0, "filesWithDuplicates": 0, "duplicateFiles": [] },
            "loc": { "status": "pass", "totalLines": 1000, "codeLines": 800, "commentLines": 100, "blankLines": 100, "longLines": 0, "maxLineLengthFound": 100, "maxLineLengthAllowed": 120, "files": 10, "filesWithLongLines": 0, "longLineFiles": [] },
            "size": { "status": "pass", "files": 10, "maxLinesPerFile": 500, "maxCodeLinesPerFile": 400, "maxLinesPerFunction": 80, "maxParametersPerFunction": 5, "violations": [] }
          }
        }"#;
        let summary: MetricsSummary = serde_json::from_str(json).unwrap();
        let err = summary.check_version().unwrap_err();
        assert!(matches!(err, SchemaVersionError::UnknownVersion(v) if v == "99"));
    }
}
