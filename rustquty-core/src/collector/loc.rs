//! LOC collector — measures lines of code metrics.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::fs;
use walkdir::WalkDir;

pub struct LocCollector {
    max_line_length: usize,
}

impl LocCollector {
    pub fn new() -> Self {
        Self {
            max_line_length: 120,
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

        // Walk all Rust files recursively
        for entry in WalkDir::new(&ctx.workspace_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };

            files += 1;
            let lines: Vec<&str> = content.lines().collect();
            total_lines += lines.len() as u32;

            // Track module/file lines
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let mut file_has_long_line = false;
            let mut in_block_comment = false;
            for line in &lines {
                let trimmed = line.trim();
                let line_len = line.len();

                if line_len > max_line_len_found {
                    max_line_len_found = line_len;
                }

                if line_len > self.max_line_length {
                    long_lines += 1;
                    file_has_long_line = true;
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
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr: String::new(),
        })
    }
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
        // BUG: lines inside a /* ... */ block comment are misclassified as code.
        // Only the opening /* and closing */ are detected as comments;
        // lines between them should also be comments but are counted as code.
        let content = "/* block start\n   inside block\n   still inside\n*/\nfn main() {}";
        let output = run_on_content(content);
        let details = parse_details(&output);

        let comment_lines = details["commentLines"].as_u64().unwrap();
        let code_lines = details["codeLines"].as_u64().unwrap();

        // Expected: 4 comment lines (/* start, 2 inside, */ end), 1 code line (fn main)
        // Actual (buggy): 2 comment lines (/* and */), 3 code lines (2 inside + fn main)
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

        assert_eq!(comment_lines, 4, "Block comment interior should be counted as comments");
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
}
