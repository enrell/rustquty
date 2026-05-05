//! LOC collector — measures lines of code metrics.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::fs;

pub struct LocCollector {
    max_line_length: usize,
}

impl LocCollector {
    pub fn new() -> Self {
        Self { max_line_length: 120 }
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

        // Walk all Rust files
        if let Ok(entries) = fs::read_dir(&ctx.workspace_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                    files += 1;

                    if let Ok(content) = fs::read_to_string(&path) {
                        let lines: Vec<&str> = content.lines().collect();
                        total_lines += lines.len() as u32;

                        // Track module/file lines
                        let file_name = path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        let mut file_has_long_line = false;
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
                            } else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.ends_with("*/") {
                                comment_lines += 1;
                            } else {
                                code_lines += 1;
                            }
                        }

                        if file_has_long_line {
                            files_with_long_lines += 1;
                            long_line_files.push(file_name);
                        }
                    }
                }
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
}
