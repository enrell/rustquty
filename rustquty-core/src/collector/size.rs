//! Size collector — measures file and function size metrics.

use super::{Collector, CollectorError, CollectorOutput};
use crate::config::SizeConfig;
use crate::context::Context;
use std::fs;
use std::path::Path;

/// A detected function in a Rust source file.
#[derive(Debug, Clone)]
struct FunctionInfo {
    /// Name of the function.
    name: String,
    /// Source file path.
    file: String,
    /// Starting line number (1-indexed).
    start_line: u32,
    /// Ending line number (1-indexed).
    #[allow(dead_code)]
    end_line: u32,
    /// Total lines in the function (end_line - start_line + 1).
    total_lines: u32,
    /// Number of explicit parameters.
    param_count: usize,
}

/// Count total lines in a file.
fn count_total_lines(content: &str) -> u32 {
    content.lines().count() as u32
}

/// Count lines that are code (not blank, not a pure comment line).
fn count_code_lines(content: &str) -> u32 {
    let mut code_lines = 0u32;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Simple comment line detection:
        // - // single line comment (including /// and //! variants)
        // - /* block comment start
        // - * block comment continuation
        // - */ block comment end
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.ends_with("*/") {
            continue;
        }
        code_lines += 1;
    }
    code_lines
}

/// Parse a Rust source file and extract function information using syn.
fn extract_functions(content: &str, file_path: &Path) -> Vec<FunctionInfo> {
    use syn::spanned::Spanned;
    use syn::{Item, parse_file};

    let mut functions = Vec::new();
    let file_stem = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Parse the file using syn.
    let Ok(syn_file) = parse_file(content) else {
        return functions;
    };

    // Process standalone functions (ItemFn).
    for item in &syn_file.items {
        if let Item::Fn(item_fn) = item {
            let name = item_fn.sig.ident.to_string();
            let span = item_fn.span();
            let start_loc = span.start();
            let end_loc = span.end();

            let start_line = start_loc.line as u32;
            let end_line = end_loc.line as u32;
            let total_lines = end_line.saturating_sub(start_line) + 1;

            // Count parameters (including self/&self/&mut self).
            let param_count = item_fn.sig.inputs.len();

            functions.push(FunctionInfo {
                name,
                file: file_stem.clone(),
                start_line,
                end_line,
                total_lines,
                param_count,
            });
        }
    }

    // Process impl blocks for methods.
    for item in &syn_file.items {
        if let Item::Impl(item_impl) = item {
            for item_impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(item_fn) = item_impl_item {
                    let name = item_fn.sig.ident.to_string();
                    let span = item_fn.span();
                    let start_loc = span.start();
                    let end_loc = span.end();

                    let start_line = start_loc.line as u32;
                    let end_line = end_loc.line as u32;
                    let total_lines = end_line.saturating_sub(start_line) + 1;

                    // Count parameters (including self/&self/&mut self).
                    let param_count = item_fn.sig.inputs.len();

                    functions.push(FunctionInfo {
                        name,
                        file: file_stem.clone(),
                        start_line,
                        end_line,
                        total_lines,
                        param_count,
                    });
                }
            }
        }
    }

    functions
}

/// A violation detected by the size collector.
#[derive(Debug, Clone, serde::Serialize)]
struct SizeViolation {
    #[serde(rename = "ruleId")]
    rule_id: String,
    file: String,
    line: u32,
    function: Option<String>,
    message: String,
    actual: u32,
    threshold: u32,
    severity: String,
}

/// Configuration for the size collector (from [gate.size] in TOML).
#[derive(Debug, Clone, Default)]
pub struct SizeCollectorConfig {
    pub max_lines_per_file: Option<u32>,
    pub max_code_lines_per_file: Option<u32>,
    pub max_lines_per_function: Option<u32>,
    pub max_parameters_per_function: Option<u32>,
}

/// The size collector.
pub struct SizeCollector {
    config: SizeCollectorConfig,
}

impl SizeCollector {
    pub fn new() -> Self {
        Self {
            config: SizeCollectorConfig::default(),
        }
    }

    /// Configure the collector from a SizeConfig (TOML [gate.size] section).
    pub fn with_config(config: SizeConfig) -> Self {
        Self {
            config: SizeCollectorConfig {
                max_lines_per_file: config.max_lines_per_file,
                max_code_lines_per_file: config.max_code_lines_per_file,
                max_lines_per_function: config.max_lines_per_function,
                max_parameters_per_function: config.max_parameters_per_function,
            },
        }
    }

    fn collect_impl(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();

        let mut total_files: u32 = 0;
        let mut max_lines_per_file: u32 = 0;
        let mut max_code_lines_per_file: u32 = 0;
        let mut max_lines_per_function: u32 = 0;
        let mut max_parameters_per_function: u32 = 0;

        let mut all_functions: Vec<FunctionInfo> = Vec::new();
        let mut violations: Vec<SizeViolation> = Vec::new();

        // Walk all Rust files.
        if let Ok(entries) = fs::read_dir(&ctx.workspace_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                    total_files += 1;

                    if let Ok(content) = fs::read_to_string(&path) {
                        let lines = count_total_lines(&content);
                        let code_lines = count_code_lines(&content);

                        if lines > max_lines_per_file {
                            max_lines_per_file = lines;
                        }
                        if code_lines > max_code_lines_per_file {
                            max_code_lines_per_file = code_lines;
                        }

                        let file_name = path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        // Check file-level violations.
                        if let Some(max) = self.config.max_lines_per_file
                            && lines > max
                        {
                            violations.push(SizeViolation {
                                rule_id: "size:max-lines-per-file".to_string(),
                                file: file_name.clone(),
                                line: 1,
                                function: None,
                                message: format!(
                                    "File has {} lines; maximum allowed is {}",
                                    lines, max
                                ),
                                actual: lines,
                                threshold: max,
                                severity: "major".to_string(),
                            });
                        }
                        if let Some(max) = self.config.max_code_lines_per_file
                            && code_lines > max
                        {
                            violations.push(SizeViolation {
                                rule_id: "size:max-code-lines-per-file".to_string(),
                                file: file_name.clone(),
                                line: 1,
                                function: None,
                                message: format!(
                                    "File has {} code lines; maximum allowed is {}",
                                    code_lines, max
                                ),
                                actual: code_lines,
                                threshold: max,
                                severity: "major".to_string(),
                            });
                        }

                        // Extract and process functions.
                        let functions = extract_functions(&content, &path);
                        for func in functions {
                            if func.total_lines > max_lines_per_function {
                                max_lines_per_function = func.total_lines;
                            }
                            if func.param_count as u32 > max_parameters_per_function {
                                max_parameters_per_function = func.param_count as u32;
                            }

                            // Check function-level violations.
                            if let Some(max) = self.config.max_lines_per_function
                                && func.total_lines > max
                            {
                                violations.push(SizeViolation {
                                    rule_id: "size:max-lines-per-function".to_string(),
                                    file: func.file.clone(),
                                    line: func.start_line,
                                    function: Some(func.name.clone()),
                                    message: format!(
                                        "Function `{}` has {} lines; maximum allowed is {}",
                                        func.name, func.total_lines, max
                                    ),
                                    actual: func.total_lines,
                                    threshold: max,
                                    severity: "major".to_string(),
                                });
                            }
                            if let Some(max) = self.config.max_parameters_per_function
                                && func.param_count as u32 > max
                            {
                                violations.push(SizeViolation {
                                    rule_id: "size:max-parameters-per-function".to_string(),
                                    file: func.file.clone(),
                                    line: func.start_line,
                                    function: Some(func.name.clone()),
                                    message: format!(
                                        "Function `{}` has {} parameters; maximum allowed is {}",
                                        func.name, func.param_count, max
                                    ),
                                    actual: func.param_count as u32,
                                    threshold: max,
                                    severity: "major".to_string(),
                                });
                            }

                            all_functions.push(func);
                        }
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Determine overall status.
        let status = if violations.is_empty() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let details = serde_json::json!({
            "files": total_files,
            "maxLinesPerFile": max_lines_per_file,
            "maxCodeLinesPerFile": max_code_lines_per_file,
            "maxLinesPerFunction": max_lines_per_function,
            "maxParametersPerFunction": max_parameters_per_function,
            "violations": violations,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr: String::new(),
        })
    }
}

impl Collector for SizeCollector {
    fn name(&self) -> &'static str {
        "size"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        self.collect_impl(ctx)
    }
}

impl Default for SizeCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a temp file with given content.
    fn temp_file(content: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, content).unwrap();
        dir
    }

    // Helper: run collector on a single file.
    fn run_on_content(content: &str) -> CollectorOutput {
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let collector = SizeCollector::new();
        collector.collect(&ctx).unwrap()
    }

    // Helper: parse details from stdout.
    fn parse_details(output: &CollectorOutput) -> serde_json::Value {
        serde_json::from_str(&output.stdout).unwrap()
    }

    // --- Line counting tests ---

    #[test]
    fn test_count_total_lines_empty() {
        assert_eq!(count_total_lines(""), 0);
    }

    #[test]
    fn test_count_total_lines_single_line() {
        assert_eq!(count_total_lines("fn main() {}"), 1);
    }

    #[test]
    fn test_count_total_lines_multiple_lines() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        assert_eq!(count_total_lines(content), 3);
    }

    #[test]
    fn test_count_code_lines_simple() {
        // Blank lines should not be counted as code.
        let content = "fn main() {\n\n\n}";
        assert_eq!(count_code_lines(content), 2); // "fn main() {" and "}"
    }

    #[test]
    fn test_count_code_lines_with_comments() {
        // Comments should not be counted as code lines.
        // "// comment" is skipped (// comment)
        // "fn main() {}" is code
        // "/* block */" is skipped (/* block */)
        let content = "// comment\nfn main() {}\n/* block */";
        assert_eq!(count_code_lines(content), 1); // "fn main() {}"
    }

    #[test]
    fn test_count_code_lines_with_doc_comments() {
        // All comment variants should be skipped.
        let content = "/// doc comment\n//! inner doc\nfn main() {}\n/* multi\n   line */";
        assert_eq!(count_code_lines(content), 1); // "fn main() {}"
    }

    #[test]
    fn test_collector_reports_total_lines() {
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["files"], 1);
        assert_eq!(details["maxLinesPerFile"], 4);
    }

    #[test]
    fn test_collector_reports_code_lines() {
        let content = "// comment\n\nfn main() {\n    let x = 1;\n}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        // 1 line for fn main, 1 for let x = 1, 1 for }
        // The empty line and comment line are not code lines.
        assert!(details["maxCodeLinesPerFile"].as_u64().unwrap() >= 3);
    }

    // --- Function detection tests ---

    #[test]
    fn test_detect_free_function() {
        let content = "fn free_function(a: i32, b: String) -> bool {\n    true\n}";
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "free_function");
        assert_eq!(functions[0].param_count, 2);
        assert!(functions[0].total_lines >= 3);
    }

    #[test]
    fn test_detect_method_in_impl() {
        let content = r#"
impl Foo {
    fn method(&self, x: i32) -> i32 {
        x
    }
}
"#;
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "method");
        assert_eq!(functions[0].param_count, 2); // &self, x
    }

    #[test]
    fn test_detect_async_fn() {
        let content = "async fn async_main() {\n    println!(\"hello\");\n}";
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "async_main");
    }

    #[test]
    fn test_detect_unsafe_fn() {
        let content = "unsafe fn unsafe_read(ptr: *const i32) -> i32 {\n    *ptr\n}";
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "unsafe_read");
        assert_eq!(functions[0].param_count, 1);
    }

    #[test]
    fn test_parameter_count_free_function() {
        let content = "fn three_params(a: i32, b: i32, c: i32) -> i32 {\n    a + b + c\n}";
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions[0].param_count, 3);
    }

    #[test]
    fn test_parameter_count_with_self() {
        let content =
            "impl Bar {\n    fn with_self(&self, x: usize) -> usize {\n        x\n    }\n}";
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions[0].param_count, 2); // &self, x
    }

    // --- Violation tests ---

    #[test]
    fn test_violation_max_lines_per_file() {
        let mut collector = SizeCollector::new();
        collector.config.max_lines_per_file = Some(5);

        // File with exactly 5 lines (threshold = 5, so no violation).
        let dir = temp_file("fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}");
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        // 5 lines: "fn main() {", "let a", "let b", "let c", "}"
        assert_eq!(details["violations"].as_array().unwrap().len(), 0);
        assert_eq!(details["maxLinesPerFile"], 5);

        // File with 6 lines against threshold 5 should produce violation.
        let dir2 = temp_file(
            "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n}",
        );
        let ctx2 = Context::new(dir2.path().to_path_buf());
        let output2 = collector.collect(&ctx2).unwrap();
        let details2: serde_json::Value = serde_json::from_str(&output2.stdout).unwrap();
        assert_eq!(details2["violations"].as_array().unwrap().len(), 1);
        assert_eq!(
            details2["violations"][0]["ruleId"],
            "size:max-lines-per-file"
        );
    }

    #[test]
    fn test_violation_max_code_lines_per_file() {
        let mut collector = SizeCollector::new();
        collector.config.max_code_lines_per_file = Some(3);

        let content = "// comment line 1\n// comment line 2\nfn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}";
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        // 4 code lines (fn main, let a, let b, let c) against threshold 3.
        assert_eq!(details["violations"].as_array().unwrap().len(), 1);
        assert_eq!(
            details["violations"][0]["ruleId"],
            "size:max-code-lines-per-file"
        );
    }

    #[test]
    fn test_violation_max_lines_per_function() {
        let mut collector = SizeCollector::new();
        collector.config.max_lines_per_function = Some(3);

        let content = "fn short() {\n    1;\n}\n\nfn longer() {\n    1;\n    2;\n    3;\n    4;\n}";
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        let violations = details["violations"].as_array().unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0]["ruleId"], "size:max-lines-per-function");
        assert_eq!(violations[0]["function"], "longer");
    }

    #[test]
    fn test_violation_max_parameters_per_function() {
        let mut collector = SizeCollector::new();
        collector.config.max_parameters_per_function = Some(2);

        let content =
            "fn two_params(a: i32, b: i32) {}\n\nfn three_params(a: i32, b: i32, c: i32) {}";
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        let violations = details["violations"].as_array().unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0]["ruleId"], "size:max-parameters-per-function");
        assert_eq!(violations[0]["function"], "three_params");
    }

    #[test]
    fn test_no_config_no_violations() {
        // Without any configuration, the collector should report metrics but not fail.
        let content = "fn huge_function(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) {\n    1;\n    2;\n    3;\n    4;\n    5;\n    6;\n    7;\n    8;\n    9;\n    10;\n}";
        let output = run_on_content(content);
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        // No violations when not configured.
        assert_eq!(details["violations"].as_array().unwrap().len(), 0);
        assert_eq!(output.status, crate::schema::CollectorStatus::Pass);
    }

    #[test]
    fn test_collector_name() {
        let collector = SizeCollector::new();
        assert_eq!(collector.name(), "size");
    }

    #[test]
    fn test_collector_available() {
        let collector = SizeCollector::new();
        assert!(collector.is_available());
    }

    #[test]
    fn test_multiple_functions_same_name() {
        // Multiple functions with the same name in different impl blocks.
        let content = r#"
impl Foo {
    fn bar(x: i32) -> i32 { x }
}

impl Baz {
    fn bar(y: i32) -> i32 { y }
}
"#;
        let dir = temp_file(content);
        let functions = extract_functions(content, &dir.path().join("test.rs"));
        assert_eq!(functions.len(), 2);
    }
}
