//! Complexity collector — measures cyclomatic complexity and nesting depth per function.

use super::{Collector, CollectorError, CollectorOutput};
use crate::config::ComplexityConfig;
use crate::context::Context;
use std::fs;
use std::path::Path;
use syn::visit::Visit;

/// A detected function with complexity metrics.
#[derive(Debug, Clone)]
struct FunctionComplexity {
    name: String,
    file: String,
    start_line: u32,
    cyclomatic_complexity: u32,
    nesting_depth: u32,
}

/// A violation detected by the complexity collector.
#[derive(Debug, Clone, serde::Serialize)]
struct ComplexityViolation {
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

/// Configuration for the complexity collector.
#[derive(Debug, Clone, Default)]
pub struct ComplexityCollectorConfig {
    pub max_cyclomatic_per_function: Option<u32>,
    pub max_nesting_depth: Option<u32>,
}

/// Extracts complexity metrics from a Rust function body using syn's Visit trait.
struct ComplexityWalker {
    cyclomatic_complexity: u32,
    nesting_depth: u32,
    max_nesting_depth: u32,
}

impl ComplexityWalker {
    fn new() -> Self {
        Self {
            cyclomatic_complexity: 1,
            nesting_depth: 0,
            max_nesting_depth: 0,
        }
    }

    fn enter_nesting(&mut self) {
        self.nesting_depth += 1;
        if self.nesting_depth > self.max_nesting_depth {
            self.max_nesting_depth = self.nesting_depth;
        }
    }

    fn exit_nesting(&mut self) {
        self.nesting_depth = self.nesting_depth.saturating_sub(1);
    }
}

impl<'ast> Visit<'ast> for ComplexityWalker {
    fn visit_expr_if(&mut self, e: &'ast syn::ExprIf) {
        self.cyclomatic_complexity += 1;
        self.enter_nesting();
        // Visit the condition
        self.visit_expr(&e.cond);
        // Visit the then branch
        self.visit_block(&e.then_branch);
        // Visit else branch
        if let Some((_, else_expr)) = &e.else_branch {
            self.visit_expr(else_expr);
        }
        self.exit_nesting();
    }

    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        self.cyclomatic_complexity += 1; // 1 for the match itself
        // Each arm adds +1 complexity
        for arm in &e.arms {
            self.cyclomatic_complexity += 1;
            // Visit arm body
            self.visit_expr(&arm.body);
        }
    }

    fn visit_expr_for_loop(&mut self, e: &'ast syn::ExprForLoop) {
        self.cyclomatic_complexity += 1;
        self.enter_nesting();
        self.visit_expr(&e.expr);
        self.visit_block(&e.body);
        self.exit_nesting();
    }

    fn visit_expr_while(&mut self, e: &'ast syn::ExprWhile) {
        self.cyclomatic_complexity += 1;
        self.enter_nesting();
        self.visit_expr(&e.cond);
        self.visit_block(&e.body);
        self.exit_nesting();
    }

    fn visit_expr_loop(&mut self, e: &'ast syn::ExprLoop) {
        self.cyclomatic_complexity += 1;
        self.enter_nesting();
        self.visit_block(&e.body);
        self.exit_nesting();
    }

    fn visit_expr_try(&mut self, e: &'ast syn::ExprTry) {
        self.cyclomatic_complexity += 1;
        self.visit_expr(&e.expr);
    }

    fn visit_expr(&mut self, e: &'ast syn::Expr) {
        match e {
            syn::Expr::Binary(bin) => {
                // Visit children
                self.visit_expr(&bin.left);
                self.visit_expr(&bin.right);
                // && and || add +1 each
                let op_str = quote::quote!(#bin.op).to_string();
                if op_str == "&&" || op_str == "||" {
                    self.cyclomatic_complexity += 1;
                }
            }
            syn::Expr::If(e) => self.visit_expr_if(e),
            syn::Expr::Match(e) => self.visit_expr_match(e),
            syn::Expr::ForLoop(e) => self.visit_expr_for_loop(e),
            syn::Expr::While(e) => self.visit_expr_while(e),
            syn::Expr::Loop(e) => self.visit_expr_loop(e),
            syn::Expr::Try(e) => self.visit_expr_try(e),
            _ => syn::visit::visit_expr(self, e),
        }
    }

    fn visit_item_fn(&mut self, _item: &'ast syn::ItemFn) {}
    fn visit_impl_item_fn(&mut self, _item: &'ast syn::ImplItemFn) {}
}

/// Parse a Rust source file and extract complexity metrics per function.
fn extract_complexity(content: &str, file_path: &Path) -> Vec<FunctionComplexity> {
    use syn::spanned::Spanned;

    let mut functions = Vec::new();
    let file_stem = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let Ok(syn_file) = syn::parse_file(content) else {
        return functions;
    };

    for item in &syn_file.items {
        if let syn::Item::Fn(item_fn) = item {
            let name = item_fn.sig.ident.to_string();
            let span = item_fn.span();
            let start_line = span.start().line as u32;

            let mut walker = ComplexityWalker::new();
            for stmt in &item_fn.block.stmts {
                walker.visit_stmt(stmt);
            }

            functions.push(FunctionComplexity {
                name,
                file: file_stem.clone(),
                start_line,
                cyclomatic_complexity: walker.cyclomatic_complexity,
                nesting_depth: walker.max_nesting_depth,
            });
        }
    }

    for item in &syn_file.items {
        if let syn::Item::Impl(item_impl) = item {
            for item_impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(item_fn) = item_impl_item {
                    let name = item_fn.sig.ident.to_string();
                    let span = item_fn.span();
                    let start_line = span.start().line as u32;

                    let mut walker = ComplexityWalker::new();
                    for stmt in &item_fn.block.stmts {
                        walker.visit_stmt(stmt);
                    }

                    functions.push(FunctionComplexity {
                        name,
                        file: file_stem.clone(),
                        start_line,
                        cyclomatic_complexity: walker.cyclomatic_complexity,
                        nesting_depth: walker.max_nesting_depth,
                    });
                }
            }
        }
    }

    functions
}

// ---------------------------------------------------------------------------
// Collector
// ---------------------------------------------------------------------------

pub struct ComplexityCollector {
    config: ComplexityCollectorConfig,
}

impl ComplexityCollector {
    pub fn new() -> Self {
        Self {
            config: ComplexityCollectorConfig::default(),
        }
    }

    pub fn with_config(config: ComplexityConfig) -> Self {
        Self {
            config: ComplexityCollectorConfig {
                max_cyclomatic_per_function: config.max_cyclomatic_per_function,
                max_nesting_depth: config.max_nesting_depth,
            },
        }
    }

    fn collect_impl(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();

        let mut all_functions: Vec<FunctionComplexity> = Vec::new();
        let mut violations: Vec<ComplexityViolation> = Vec::new();

        let mut max_cyclomatic: u32 = 0;
        let mut max_nesting: u32 = 0;

        if let Ok(entries) = fs::read_dir(&ctx.workspace_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let funcs = extract_complexity(&content, &path);
                        for func in funcs {
                            if func.cyclomatic_complexity > max_cyclomatic {
                                max_cyclomatic = func.cyclomatic_complexity;
                            }
                            if func.nesting_depth > max_nesting {
                                max_nesting = func.nesting_depth;
                            }

                            if let Some(max) = self.config.max_cyclomatic_per_function
                                && func.cyclomatic_complexity > max
                            {
                                violations.push(ComplexityViolation {
                                    rule_id: "complexity:max-cyclomatic-per-function"
                                        .to_string(),
                                    file: func.file.clone(),
                                    line: func.start_line,
                                    function: Some(func.name.clone()),
                                    message: format!(
                                        "Function `{}` has cyclomatic complexity {}; maximum allowed is {}",
                                        func.name, func.cyclomatic_complexity, max
                                    ),
                                    actual: func.cyclomatic_complexity,
                                    threshold: max,
                                    severity: "major".to_string(),
                                });
                            }
                            if let Some(max) = self.config.max_nesting_depth
                                && func.nesting_depth > max
                            {
                                violations.push(ComplexityViolation {
                                    rule_id: "complexity:max-nesting-depth".to_string(),
                                    file: func.file.clone(),
                                    line: func.start_line,
                                    function: Some(func.name.clone()),
                                    message: format!(
                                        "Function `{}` has nesting depth {}; maximum allowed is {}",
                                        func.name, func.nesting_depth, max
                                    ),
                                    actual: func.nesting_depth,
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

        let status = if violations.is_empty() {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let complex_functions = all_functions
            .iter()
            .filter(|f| {
                self.config
                    .max_cyclomatic_per_function
                    .is_some_and(|max| f.cyclomatic_complexity > max)
            })
            .count() as u32;

        let details = serde_json::json!({
            "functions": all_functions.len() as u32,
            "maxCyclomaticComplexity": max_cyclomatic,
            "maxNestingDepth": max_nesting,
            "complexFunctions": complex_functions,
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

impl Collector for ComplexityCollector {
    fn name(&self) -> &'static str {
        "complexity"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        self.collect_impl(ctx)
    }
}

impl Default for ComplexityCollector {
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

    fn temp_file(content: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, content).unwrap();
        dir
    }

    fn run_on_content(content: &str) -> CollectorOutput {
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        ComplexityCollector::new().collect(&ctx).unwrap()
    }

    #[test]
    fn test_empty_function_has_complexity_one() {
        let content = "fn empty() {}";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].cyclomatic_complexity, 1);
    }

    #[test]
    fn test_if_increments_complexity() {
        let content = "fn f(x: i32) { if x > 0 { println!(\"pos\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_else_if_increments_complexity() {
        let content =
            "fn f(x: i32) { if x > 0 { println!(\"pos\"); } else if x < 0 { println!(\"neg\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 3);
    }

    #[test]
    fn test_match_increments_complexity() {
        let content = "fn f(x: i32) { match x { 1 => {}, 2 => {}, _ => {} } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));

        println!("Match test - functions: {:?}", funcs);
        assert_eq!(funcs[0].cyclomatic_complexity, 5);
    }

    #[test]
    fn test_match_arm_increments_complexity() {
        let content = "fn f(x: i32) { match x { 1 => {}, 2 => {}, 3 => {}, _ => {} } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 6);
    }

    #[test]
    fn test_for_increments_complexity() {
        let content = "fn f() { for i in 0..10 { let x = i; } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_while_increments_complexity() {
        let content = "fn f() { while true { println!(\"loop\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_loop_increments_complexity() {
        let content = "fn f() { loop { break; } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_and_operator_increments_complexity() {
        let content = "fn f(a: bool, b: bool) { if a && b { println!(\"both\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_or_operator_increments_complexity() {
        let content = "fn f(a: bool, b: bool) { if a || b { println!(\"either\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_question_operator_increments_complexity() {
        let content = "fn f(x: Option<i32>) -> Option<i32> { Some(x?) }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].cyclomatic_complexity, 2);
    }

    #[test]
    fn test_nesting_depth_simple_if() {
        let content = "fn f(x: i32) { if x > 0 { println!(\"hi\"); } }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs[0].nesting_depth, 1);
    }

    #[test]
    fn test_nesting_depth_if_for_if() {
        let content = r#"
fn example(x: i32) {
    if x > 0 {
        for i in 0..x {
            if i % 2 == 0 {
                println!("{i}");
            }
        }
    }
}
"#;
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert!(funcs[0].nesting_depth >= 3);
    }

    #[test]
    fn test_detect_method_in_impl() {
        let content = r#"
impl Foo {
    fn method(&self, x: i32) -> i32 { x }
}
"#;
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "method");
    }

    #[test]
    fn test_detect_async_fn() {
        let content = "async fn async_main() { println!(\"hello\"); }";
        let dir = temp_file(content);
        let funcs = extract_complexity(content, &dir.path().join("test.rs"));
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "async_main");
    }

    #[test]
    fn test_violation_max_cyclomatic() {
        let mut collector = ComplexityCollector::new();
        collector.config.max_cyclomatic_per_function = Some(3);

        let content =
            "fn f(x: i32) { if x > 0 { if x > 10 { if x > 100 { println!(\"hi\"); } } } }";
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        let violations = details["violations"].as_array().unwrap();
        assert!(!violations.is_empty());
        assert!(
            violations
                .iter()
                .any(|v| v["ruleId"] == "complexity:max-cyclomatic-per-function")
        );
    }

    #[test]
    fn test_violation_max_nesting() {
        let mut collector = ComplexityCollector::new();
        collector.config.max_nesting_depth = Some(2);

        let content = "fn f() { if true { if true { if true { println!(\"deep\"); } } } }";
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let output = collector.collect(&ctx).unwrap();
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        let violations = details["violations"].as_array().unwrap();
        assert!(!violations.is_empty());
        assert!(
            violations
                .iter()
                .any(|v| v["ruleId"] == "complexity:max-nesting-depth")
        );
    }

    #[test]
    fn test_no_config_no_violations() {
        let content = "fn f() { if true { for i in 0..10 { if i > 5 { println!(\"{i}\"); } } } } }";
        let output = run_on_content(content);
        let details: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();

        assert_eq!(details["violations"].as_array().unwrap().len(), 0);
        assert_eq!(output.status, crate::schema::CollectorStatus::Pass);
    }

    #[test]
    fn test_collector_name() {
        assert_eq!(ComplexityCollector::new().name(), "complexity");
    }

    #[test]
    fn test_collector_available() {
        assert!(ComplexityCollector::new().is_available());
    }
}
