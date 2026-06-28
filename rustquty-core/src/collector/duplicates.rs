//! Duplicates collector — detects code duplication.

// This collector tokenizes Rust source and detects repeated blocks of code using
// exact sliding-window matches over normalized token streams.

use super::{Collector, CollectorError, CollectorOutput, is_scannable_rust_file};
use crate::context::Context;
use crate::schema::{DuplicateBlock, DuplicateOccurrence};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct DuplicatesCollector;

const MIN_DUPLICATE_TOKENS: usize = 100;
const MIN_DUPLICATE_LINES: u32 = 6;
const DUPLICATE_BLOCK_DETAIL_CAP: usize = 100;

#[derive(Debug, Clone)]
struct TokenInfo {
    normalized: String,
    line: u32,
}

#[derive(Debug)]
struct SourceFile {
    path: String,
    total_lines: u32,
    tokens: Vec<TokenInfo>,
}

#[derive(Debug, Clone)]
struct WindowInfo {
    file_idx: usize,
    start_token: usize,
    end_token: usize,
}

#[derive(Debug, Clone)]
struct TokenInterval {
    start_token: usize,
    end_token: usize,
}

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

        let mut files = Vec::new();

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
            let path = relative_path(path, &ctx.workspace_root);
            let tokens = tokenize_source(&content);
            files.push(SourceFile {
                path,
                total_lines: content.lines().count() as u32,
                tokens,
            });
        }

        let total_lines: u32 = files.iter().map(|file| file.total_lines).sum();
        let intervals_by_file = find_duplicate_intervals(&files);
        let duplicate_lines = count_duplicate_lines(&files, &intervals_by_file);
        let duplicate_files: Vec<String> = intervals_by_file
            .iter()
            .enumerate()
            .filter(|(_, intervals)| !intervals.is_empty())
            .map(|(idx, _)| files[idx].path.clone())
            .collect();
        let files_with_duplicates = duplicate_files.len() as u32;
        let (duplicate_blocks, duplicate_blocks_omitted) =
            build_duplicate_blocks(&files, &intervals_by_file);
        let duration_ms = start.elapsed().as_millis() as u64;

        let status = if duplicate_lines > 0 {
            crate::schema::CollectorStatus::Fail
        } else {
            crate::schema::CollectorStatus::Pass
        };

        let details = serde_json::json!({
            "totalLines": total_lines,
            "duplicateLines": duplicate_lines,
            "filesWithDuplicates": files_with_duplicates,
            "duplicateFiles": duplicate_files,
            "duplicateBlocks": duplicate_blocks,
            "duplicateBlocksOmitted": duplicate_blocks_omitted,
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

fn tokenize_source(content: &str) -> Vec<TokenInfo> {
    let Ok(stream) = content.parse::<TokenStream>() else {
        return Vec::new();
    };
    let mut tokens = Vec::new();
    flatten_token_stream(stream, &mut tokens);
    tokens
}

fn flatten_token_stream(stream: TokenStream, tokens: &mut Vec<TokenInfo>) {
    for token in stream {
        match token {
            TokenTree::Group(group) => {
                let (open, close) = delimiter_tokens(group.delimiter());
                if !open.is_empty() {
                    tokens.push(TokenInfo {
                        normalized: open.to_string(),
                        line: group.span_open().start().line as u32,
                    });
                }
                flatten_token_stream(group.stream(), tokens);
                if !close.is_empty() {
                    tokens.push(TokenInfo {
                        normalized: close.to_string(),
                        line: group.span_close().end().line as u32,
                    });
                }
            }
            TokenTree::Ident(ident) => tokens.push(TokenInfo {
                normalized: format!("ident:{ident}"),
                line: ident.span().start().line as u32,
            }),
            TokenTree::Punct(punct) => tokens.push(TokenInfo {
                normalized: format!("punct:{}:{:?}", punct.as_char(), punct.spacing()),
                line: punct.span().start().line as u32,
            }),
            TokenTree::Literal(literal) => tokens.push(TokenInfo {
                normalized: format!("literal:{literal}"),
                line: literal.span().start().line as u32,
            }),
        }
    }
}

fn delimiter_tokens(delimiter: Delimiter) -> (&'static str, &'static str) {
    match delimiter {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("", ""),
    }
}

fn find_duplicate_intervals(files: &[SourceFile]) -> Vec<Vec<TokenInterval>> {
    let mut windows_by_key: HashMap<String, Vec<WindowInfo>> = HashMap::new();

    for (file_idx, file) in files.iter().enumerate() {
        if file.tokens.len() < MIN_DUPLICATE_TOKENS {
            continue;
        }

        for start_token in 0..=file.tokens.len() - MIN_DUPLICATE_TOKENS {
            let end_token = start_token + MIN_DUPLICATE_TOKENS;
            let start_line = file.tokens[start_token].line;
            let end_line = file.tokens[end_token - 1].line;
            if end_line.saturating_sub(start_line) + 1 < MIN_DUPLICATE_LINES {
                continue;
            }

            let key = token_key(&file.tokens[start_token..end_token]);
            windows_by_key.entry(key).or_default().push(WindowInfo {
                file_idx,
                start_token,
                end_token,
            });
        }
    }

    let mut intervals_by_file = vec![Vec::new(); files.len()];
    for windows in windows_by_key.values() {
        if windows.len() < 2 || !has_non_overlapping_pair(windows) {
            continue;
        }

        for window in windows {
            intervals_by_file[window.file_idx].push(TokenInterval {
                start_token: window.start_token,
                end_token: window.end_token,
            });
        }
    }

    for intervals in &mut intervals_by_file {
        merge_token_intervals(intervals);
    }

    intervals_by_file
}

fn token_key(tokens: &[TokenInfo]) -> String {
    let mut key = String::new();
    for token in tokens {
        key.push_str(&token.normalized);
        key.push('\0');
    }
    key
}

fn has_non_overlapping_pair(windows: &[WindowInfo]) -> bool {
    let mut first_by_file: HashMap<usize, (usize, usize)> = HashMap::new();
    for window in windows {
        if first_by_file
            .keys()
            .any(|file_idx| *file_idx != window.file_idx)
        {
            return true;
        }

        let entry = first_by_file
            .entry(window.file_idx)
            .or_insert((window.start_token, window.end_token));
        if window.start_token >= entry.1 || window.end_token <= entry.0 {
            return true;
        }
        if window.end_token < entry.1 {
            *entry = (window.start_token, window.end_token);
        }
    }
    false
}

fn merge_token_intervals(intervals: &mut Vec<TokenInterval>) {
    intervals.sort_by_key(|interval| interval.start_token);
    let mut merged: Vec<TokenInterval> = Vec::new();

    for interval in intervals.drain(..) {
        if let Some(last) = merged.last_mut()
            && interval.start_token <= last.end_token
        {
            last.end_token = last.end_token.max(interval.end_token);
            continue;
        }
        merged.push(interval);
    }

    *intervals = merged;
}

fn count_duplicate_lines(files: &[SourceFile], intervals_by_file: &[Vec<TokenInterval>]) -> u32 {
    intervals_by_file
        .iter()
        .enumerate()
        .map(|(file_idx, intervals)| {
            let mut line_intervals = intervals
                .iter()
                .filter_map(|interval| line_interval(&files[file_idx], interval))
                .collect::<Vec<_>>();
            line_intervals.sort_by_key(|(start, _)| *start);

            let mut total = 0;
            let mut current: Option<(u32, u32)> = None;
            for (start, end) in line_intervals {
                if let Some((current_start, current_end)) = current {
                    if start <= current_end + 1 {
                        current = Some((current_start, current_end.max(end)));
                    } else {
                        total += current_end - current_start + 1;
                        current = Some((start, end));
                    }
                } else {
                    current = Some((start, end));
                }
            }
            if let Some((start, end)) = current {
                total += end - start + 1;
            }
            total
        })
        .sum()
}

fn build_duplicate_blocks(
    files: &[SourceFile],
    intervals_by_file: &[Vec<TokenInterval>],
) -> (Vec<DuplicateBlock>, u32) {
    let mut occurrences_by_block: HashMap<String, Vec<DuplicateOccurrence>> = HashMap::new();
    let mut metadata_by_block: HashMap<String, (u32, u32)> = HashMap::new();

    for (file_idx, intervals) in intervals_by_file.iter().enumerate() {
        for interval in intervals {
            let Some((start_line, end_line)) = line_interval(&files[file_idx], interval) else {
                continue;
            };
            let lines = end_line.saturating_sub(start_line) + 1;
            if lines < MIN_DUPLICATE_LINES {
                continue;
            }

            let block_key =
                token_key(&files[file_idx].tokens[interval.start_token..interval.end_token]);
            occurrences_by_block
                .entry(block_key.clone())
                .or_default()
                .push(DuplicateOccurrence {
                    file: files[file_idx].path.clone(),
                    start_line,
                    end_line,
                });
            metadata_by_block.insert(
                block_key,
                (lines, (interval.end_token - interval.start_token) as u32),
            );
        }
    }

    let mut blocks: Vec<DuplicateBlock> = occurrences_by_block
        .into_iter()
        .filter_map(|(key, occurrences)| {
            if occurrences.len() < 2 {
                return None;
            }
            let (lines, tokens) = metadata_by_block.get(&key).copied().unwrap_or_default();
            Some(DuplicateBlock {
                lines,
                tokens,
                occurrences,
            })
        })
        .collect();

    blocks.sort_by(|a, b| {
        b.lines
            .cmp(&a.lines)
            .then_with(|| b.occurrences.len().cmp(&a.occurrences.len()))
    });

    let omitted = blocks.len().saturating_sub(DUPLICATE_BLOCK_DETAIL_CAP) as u32;
    blocks.truncate(DUPLICATE_BLOCK_DETAIL_CAP);
    (blocks, omitted)
}

fn line_interval(file: &SourceFile, interval: &TokenInterval) -> Option<(u32, u32)> {
    if interval.start_token >= interval.end_token || interval.end_token > file.tokens.len() {
        return None;
    }
    let start = file.tokens[interval.start_token].line;
    let end = file.tokens[interval.end_token - 1].line;
    Some((start, end))
}

impl Default for DuplicatesCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    fn run_on_files(files: &[(&str, &str)]) -> serde_json::Value {
        let dir = tempfile::TempDir::new().unwrap();
        for (path, content) in files {
            let file_path = dir.path().join(path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(file_path, content).unwrap();
        }

        let ctx = Context::new(dir.path().to_path_buf());
        let output = DuplicatesCollector::new().collect(&ctx).unwrap();
        serde_json::from_str(&output.stdout).unwrap()
    }

    fn duplicated_block(name: &str, comments: bool) -> String {
        let mut block = format!("pub fn {name}() -> usize {{\n");
        block.push_str("    let mut total = 0usize;\n");
        for idx in 0..36 {
            if comments && idx % 8 == 0 {
                block.push_str("    // intentionally ignored by token duplicate detection\n");
            }
            block.push_str(&format!("    total += items::{idx}().unwrap_or({idx});\n"));
        }
        block.push_str("    total\n}\n");
        block
    }

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

    #[test]
    fn test_common_punctuation_is_not_duplicate_code() {
        let details = run_on_files(&[(
            "src/lib.rs",
            r#"
#[test]
fn first() {
    assert_eq!(1, 1);
}

#[test]
fn second() {
    assert_eq!(2, 2);
}
"#,
        )]);

        assert_eq!(details["duplicateLines"].as_u64().unwrap(), 0);
        assert_eq!(details["filesWithDuplicates"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_detects_duplicate_token_blocks_across_files() {
        let block = duplicated_block("parse_config", false);
        let details = run_on_files(&[("src/a.rs", &block), ("src/b.rs", &block)]);

        assert!(details["duplicateLines"].as_u64().unwrap() >= 12);
        assert_eq!(details["filesWithDuplicates"].as_u64().unwrap(), 2);
        assert!(!details["duplicateBlocks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_duplicate_detection_ignores_whitespace_and_comments() {
        let first = duplicated_block("parse_config", false);
        let second = duplicated_block("parse_config", true).replace("    ", "        ");
        let details = run_on_files(&[("src/a.rs", &first), ("src/b.rs", &second)]);

        assert!(details["duplicateLines"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_detects_same_file_non_overlapping_blocks() {
        let block = duplicated_block("parse_config", false);
        let content = format!("{block}\n{block}");
        let details = run_on_files(&[("src/lib.rs", &content)]);

        assert!(details["duplicateLines"].as_u64().unwrap() > 0);
        assert_eq!(details["filesWithDuplicates"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_duplicate_line_count_does_not_exceed_total_lines() {
        let block = duplicated_block("parse_config", false);
        let details = run_on_files(&[("src/a.rs", &block), ("src/b.rs", &block)]);

        let duplicate_lines = details["duplicateLines"].as_u64().unwrap();
        let total_lines = details["totalLines"].as_u64().unwrap();
        assert!(duplicate_lines <= total_lines);
    }
}
