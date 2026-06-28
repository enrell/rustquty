//! LOC collector — measures lines of code metrics.

use super::{Collector, CollectorError, CollectorOutput, is_scannable_rust_file};
use crate::config::LocConfig;
use crate::context::Context;
use crate::schema::LongLineDetail;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct LocCollector {
    max_line_length: usize,
}

const DEFAULT_MAX_LINE_LENGTH: usize = 120;
const LONG_LINE_DETAIL_CAP: usize = 200;

impl LocCollector {
    pub fn new() -> Self {
        Self {
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
        }
    }

    pub fn with_config(config: LocConfig) -> Self {
        Self {
            max_line_length: config.max_line_length.unwrap_or(DEFAULT_MAX_LINE_LENGTH),
        }
    }
}

impl Collector for LocCollector {
    fn name(&self) -> &'static str {
        "loc"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();

        let mut total_lines: u32 = 0;
        let mut code_lines: u32 = 0;
        let mut comment_lines: u32 = 0;
        let mut blank_lines: u32 = 0;
        let mut long_lines: u32 = 0;
        let mut files: u32 = 0;
        let mut max_line_len_found: usize = 0;
        let mut files_with_long_lines: u32 = 0;
        let mut long_line_files: Vec<String> = Vec::new();
        let mut long_line_details: Vec<LongLineDetail> = Vec::new();
        let mut long_line_details_omitted: u32 = 0;

        // Walk all Rust files recursively
        for entry in WalkDir::new(&ctx.workspace_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !is_scannable_rust_file(path) {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };

            files += 1;
            let lines: Vec<&str> = content.lines().collect();
            total_lines += lines.len() as u32;

            // Track module/file lines
            let file_name = relative_path(path, &ctx.workspace_root);

            let mut file_has_long_line = false;
            let mut in_block_comment = false;
            for (line_idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                let line_len = line.len();

                if line_len > max_line_len_found {
                    max_line_len_found = line_len;
                }

                if line_len > self.max_line_length {
                    long_lines += 1;
                    file_has_long_line = true;
                    if long_line_details.len() < LONG_LINE_DETAIL_CAP {
                        long_line_details.push(LongLineDetail {
                            file: file_name.clone(),
                            line: line_idx as u32 + 1,
                            length: line_len,
                            threshold: self.max_line_length,
                        });
                    } else {
                        long_line_details_omitted += 1;
                    }
                }

                if trimmed.is_empty() {
                    blank_lines += 1;
                } else if in_block_comment {
                    comment_lines += 1;
                    if trimmed.ends_with("*/") {
                        in_block_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    comment_lines += 1;
                } else if trimmed.starts_with("/*") {
                    comment_lines += 1;
                    if !trimmed.ends_with("*/") {
                        in_block_comment = true;
                    }
                } else {
                    code_lines += 1;
                }
            }

            if file_has_long_line {
                files_with_long_lines += 1;
                long_line_files.push(file_name);
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Determine status based on line length compliance
        let status = if long_lines == 0 {
            crate::schema::CollectorStatus::Pass
        } else {
            crate::schema::CollectorStatus::Fail
        };

        let details = serde_json::json!({
            "totalLines": total_lines,
            "codeLines": code_lines,
            "commentLines": comment_lines,
            "blankLines": blank_lines,
            "longLines": long_lines,
            "maxLineLengthFound": max_line_len_found,
            "maxLineLengthAllowed": self.max_line_length,
            "files": files,
            "filesWithLongLines": files_with_long_lines,
            "longLineFiles": long_line_files,
            "longLineDetails": long_line_details,
            "longLineDetailsOmitted": long_line_details_omitted,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr: String::new(),
        })
    }
}

fn relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

impl Default for LocCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    fn temp_file(content: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, content).unwrap();
        dir
    }

    fn run_on_content(content: &str) -> CollectorOutput {
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let collector = LocCollector::new();
        collector.collect(&ctx).unwrap()
    }

    fn run_on_content_with_threshold(content: &str, max_line_length: usize) -> CollectorOutput {
        let dir = temp_file(content);
        let ctx = Context::new(dir.path().to_path_buf());
        let collector = LocCollector::with_config(LocConfig {
            max_line_length: Some(max_line_length),
        });
        collector.collect(&ctx).unwrap()
    }

    fn parse_details(output: &CollectorOutput) -> serde_json::Value {
        serde_json::from_str(&output.stdout).unwrap()
    }

    #[test]
    fn test_loc_collector_name() {
        let collector = LocCollector::new();
        assert_eq!(collector.name(), "loc");
    }

    #[test]
    fn test_loc_collector_available() {
        let collector = LocCollector::new();
        assert!(collector.is_available());
    }

    #[test]
    fn test_block_comment_lines_counted_as_code() {
        let content = "/* block start\n   inside block\n   still inside\n*/\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);

        let comment_lines = details["commentLines"].as_u64().unwrap();
        let code_lines = details["codeLines"].as_u64().unwrap();

        assert_eq!(
            comment_lines, 4,
            "Block comment interior lines should be counted as comments"
        );
        assert_eq!(
            code_lines, 1,
            "Only 'fn main() {{}}' should be code, not block comment lines"
        );
    }

    #[test]
    fn test_single_line_block_comment() {
        // Single-line /* ... */ should work correctly
        let content = "/* single line comment */\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);

        let comment_lines = details["commentLines"].as_u64().unwrap();
        let code_lines = details["codeLines"].as_u64().unwrap();

        assert_eq!(comment_lines, 1);
        assert_eq!(code_lines, 1);
    }

    #[test]
    fn test_mixed_comments() {
        let content = "// line comment\nfn main() {}\n/* block\n   interior\n*/";
        let output = run_on_content(content);
        let details = parse_details(&output);

        let comment_lines = details["commentLines"].as_u64().unwrap();
        let code_lines = details["codeLines"].as_u64().unwrap();

        assert_eq!(
            comment_lines, 4,
            "Block comment interior should be counted as comments"
        );
        assert_eq!(code_lines, 1);
    }

    // --- Regression tests ---

    #[test]
    fn test_loc_regression_block_comment_at_eof() {
        // Block comment that ends at EOF without trailing newline
        let content = "fn main() {}\n/* comment\n   interior */";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 2);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_regression_code_before_and_after_block() {
        let content = "fn first() {}\n/* block\n   inside\n*/\nfn second() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 3);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 2);
    }

    #[test]
    fn test_loc_regression_empty_block_comment() {
        let content = "/**/\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 1);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_regression_single_line_block_with_code() {
        let content = "let x = 1; /* inline */ let y = 2;";
        let output = run_on_content(content);
        let details = parse_details(&output);
        // The line starts with code, so it's counted as code (inline comment not stripped)
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_regression_multiple_block_comments() {
        let content = "/* first\n   inside\n*/\nfn main() {}\n/* second\n   inside\n*/";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 6);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_regression_blank_lines_not_miscounted() {
        let content = "/* start\n\n   after blank\n*/\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["blankLines"].as_u64().unwrap(), 1);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 3);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_regression_doc_comments_counted() {
        let content = "/// doc\n//! inner\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);
        assert_eq!(details["commentLines"].as_u64().unwrap(), 2);
        assert_eq!(details["codeLines"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_loc_configured_max_line_length_and_details() {
        let content = "fn main() {}\nlet very_long_value = \"abcdef\";\n";
        let output = run_on_content_with_threshold(content, 20);
        let details = parse_details(&output);

        assert_eq!(details["longLines"].as_u64().unwrap(), 1);
        assert_eq!(details["maxLineLengthAllowed"].as_u64().unwrap(), 20);
        assert_eq!(details["filesWithLongLines"].as_u64().unwrap(), 1);
        assert_eq!(details["longLineFiles"].as_array().unwrap().len(), 1);
        let first = &details["longLineDetails"].as_array().unwrap()[0];
        assert_eq!(first["file"].as_str().unwrap(), "test.rs");
        assert_eq!(first["line"].as_u64().unwrap(), 2);
        assert!(first["length"].as_u64().unwrap() > 20);
        assert_eq!(first["threshold"].as_u64().unwrap(), 20);
    }
}
