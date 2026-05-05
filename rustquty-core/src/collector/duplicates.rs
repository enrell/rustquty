//! Duplicates collector — detects code duplication.

// This collector uses a simple hash-based approach to detect duplicate lines
// and blocks of code. It walks Rust source files and finds exact duplicates.

use super::{Collector, CollectorError, CollectorOutput};
use crate::context::Context;
use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;

pub struct DuplicatesCollector;

impl DuplicatesCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Collector for DuplicatesCollector {
    fn name(&self) -> &'static str {
        "duplicates"
    }

    fn is_available(&self) -> bool {
        // Always available - we implement the detection ourselves
        true
    }

    fn collect(&self, ctx: &Context) -> Result<CollectorOutput, CollectorError> {
        let start = std::time::Instant::now();

        let mut total_lines: u32 = 0;
        let mut duplicate_lines: u32 = 0;
        let mut files_with_duplicates: u32 = 0;
        let mut duplicate_files: Vec<String> = Vec::new();

        // Hash to count line appearances
        let mut line_counts: HashMap<String, u32> = HashMap::new();
        let mut file_lines: HashMap<String, Vec<String>> = HashMap::new();

        // Walk all Rust files recursively
        for entry in WalkDir::new(&ctx.workspace_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !path.extension().is_some_and(|e| e == "rs") {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            let lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).collect();
            total_lines += lines.len() as u32;

            // Count line frequencies
            for line in &lines {
                if !line.is_empty() && !line.starts_with("//") && !line.starts_with("/*") {
                    *line_counts.entry(line.clone()).or_insert(0) += 1;
                }
            }

            file_lines.insert(path.to_string_lossy().to_string(), lines);
        }

        // Find lines that appear more than once (potential duplicates)
        let duplicate_line_keys: HashMap<String, u32> = line_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .collect();

        // Count duplicate lines per file
        for (file, lines) in &file_lines {
            let file_dup_lines: Vec<String> = lines
                .iter()
                .filter(|line| {
                    !line.is_empty()
                        && !line.starts_with("//")
                        && !line.starts_with("/*")
                        && duplicate_line_keys
                            .get(line.as_str())
                            .is_some_and(|&c| c > 1)
                })
                .cloned()
                .collect();

            if !file_dup_lines.is_empty() {
                duplicate_lines += file_dup_lines.len() as u32;
                files_with_duplicates += 1;
                duplicate_files.push(file.clone());
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Determine status based on findings
        let status = if files_with_duplicates > 0 {
            crate::schema::CollectorStatus::Fail
        } else {
            crate::schema::CollectorStatus::Pass
        };

        let details = serde_json::json!({
            "totalLines": total_lines,
            "duplicateLines": duplicate_lines,
            "filesWithDuplicates": files_with_duplicates,
            "duplicateFiles": duplicate_files,
        });

        Ok(CollectorOutput {
            status,
            duration_ms,
            stdout: serde_json::to_string(&details).unwrap_or_default(),
            stderr: String::new(),
        })
    }
}

impl Default for DuplicatesCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicates_collector_name() {
        let collector = DuplicatesCollector::new();
        assert_eq!(collector.name(), "duplicates");
    }

    #[test]
    fn test_duplicates_collector_available() {
        let collector = DuplicatesCollector::new();
        assert!(collector.is_available());
    }
}
