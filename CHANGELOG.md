# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.3] - 2026-06-28

### Changed

- **Duplicates collector**: Replaced line-frequency counting with token-window duplicate block detection, avoiding false positives from repeated braces, attributes, and common punctuation.
- **LOC collector**: Uses the configured max line length during collection so metrics, gate messages, and verbose output agree on the same threshold.
- **Verbose output**: Adds capped file:line details for duplicate blocks and long-line violations.

### Fixed

- **LOC aggregation**: `maxLineLengthAllowed`, `filesWithLongLines`, `longLineFiles`, and long-line details are now preserved in `metricsSummary.json`.
- **Gate messages**: LOC failures now report the actual collector threshold instead of a baseline max observed line length.
- **File scanning**: Built-in Rust source collectors skip `target/`, `.git/`, and `quality/` directories.

## [0.4.0] - 2026-06-01

### Added

- **Absolute thresholds (SonarQube standards)**: New `[gate.defaults]` section in `rustquty.toml` for industry-standard absolute thresholds. Overrides the ratchet baseline model when set. Based on SonarQube, ESLint, Detekt, and DeepSource defaults.
- **`--verbose` / `-v` flag**: Shows detailed violations with `file:line` info for size, complexity, and loc violations.
- **`GateConfig` API**: `Gate::run_with_config()` allows passing absolute thresholds programmatically.
- **`execute_collectors` + `assemble_results`**: New public API in `rustquty-core` for running collectors and assembling results separately.
- **Config error warnings**: `rustquty.toml` parse errors now print a warning to stderr instead of being silently ignored.
- **Invalid collector name warnings**: `--disable-collector` with an unknown name now warns instead of silently ignoring.

### Changed

- **Collector data propagation**: All 6 external collectors (coverage, deny, audit, tests, hack, clippy) now propagate their parsed metrics to the JSON output instead of discarding them.
- **`run_collectors` consolidated**: The duplicated `run_collectors` logic between core and CLI has been consolidated. The CLI now delegates to the core's `execute_collectors` + `assemble_results`.
- **`Gate::run` refactored**: Reduced from ~270 lines to ~120 lines using macros for repetitive check patterns.
- **`all_collectors` consolidated**: 4 variants of `all_collectors*()` replaced with a single `all_collectors(size_config, complexity_config)` function.
- **`chrono_now` centralized**: Time utility functions moved to `rustquty-core/src/util.rs`, eliminating 4 copies (~240 lines).
- **Structs deduplicated**: `SizeCollectorConfig` and `ComplexityCollectorConfig` removed; reuses `SizeConfig` and `ComplexityConfig` from config.rs.
- **CLI version**: Uses `#[command(version)]` from clap instead of hardcoded string.
- **Variable naming**: `t` renamed to `thresholds` in gate.rs for clarity.

### Fixed

- **Block comment tracking**: LOC and size collectors now correctly track `/* ... */` block comment state. Interior lines were previously misclassified as code.
- **ISO-8601 timestamps**: `generated_at` and `created_at` fields now use ISO-8601 format (`YYYY-MM-DDTHH:MM:SSZ`) instead of raw Unix timestamps.
- **Doctor version**: `rustquty doctor` now shows the actual version instead of hardcoded "0.1.0".
- **Report display**: `print_human_report` now shows actual report data instead of hardcoded placeholder values.
- **Size/complexity violations**: Violations from size and complexity collectors are now properly parsed and included in the MetricsSummary.

### Removed

- **Dead code**: Removed unused `end_line` field from `FunctionInfo` struct.
- **Duplicated code**: Removed ~750 lines of duplicated logic across modules.

## [0.3.1] - 2026-05-05

### Fixed

- **Recursive file scanning**: Built-in collectors (`duplicates`, `loc`, `size`, `complexity`) now recursively scan member crate directories instead of only the workspace root. Fixes all-zeros metrics when source files are in subdirectories.
- **Rust edition detection**: Added support for `[workspace.package]` section in Cargo.toml (used by many Rust 2024 projects). Previously only detected edition from `[package]` section.
- **Human-readable output**: Added missing built-in collectors (duplicates, loc, size, complexity) to the terminal output.
- **Metrics JSON parsing**: Fixed `duplicates` and `loc` collectors so their full metrics (not just status) are properly parsed and included in the JSON output.

## [0.3.0] - 2026-05-04

### Added

- **complexity collector**: AST-based cyclomatic complexity and nesting depth metrics per function. Counts decision points (if, match arms, loops, &&, ||, ?) and tracks maximum nesting depth. Built-in, no external tool required.

### Configuration

```toml
[gate.complexity]
max-cyclomatic-per-function = 10  # Optional
max-nesting-depth = 5             # Optional
```

### Metrics

- `functions`: total functions analyzed
- `maxCyclomaticComplexity`: highest complexity in workspace
- `maxNestingDepth`: deepest nesting level found
- `complexFunctions`: count of functions exceeding threshold
- `violations`: per-function violations with file, line, function name, actual vs threshold

### Changed

- All 12 collectors now included by default: fmt, clippy, tests, coverage, deny, audit, hack, mutants, duplicates, loc, size, **complexity**

## [0.2.0] - 2026-05-04

### Added

- **duplicates collector**: Detects code duplication by finding identical lines across Rust source files. Tracks total lines, duplicate lines, and files with duplicates. Status passes when no duplicates are found.

- **loc collector**: Measures lines of code metrics including total, code, comment, and blank lines. Also enforces **max line length** (default 120 chars) and reports files with long lines.

- **Rust edition detection**: Properly detects Rust edition from `[package]` section in member crate Cargo.toml files, supporting Rust 2024 edition.

### Changed

- All 10 collectors now included by default: fmt, clippy, tests, coverage, deny, audit, hack, mutants, **duplicates**, **loc**
- `rustquty doctor` now shows 10 collectors

### Fixed

- Fixed hardcoded `rust_edition: "2021"` to properly detect edition from Cargo.toml
- Updated schema to include `duplicates` and `loc` result and threshold types
- Updated baseline writer to handle new collector metrics
- Fixed clippy warnings in new collectors (collapsible_if, unnecessary_map_or)

## [0.1.0] - 2026-05-04

### Added

- Initial release of rustquty, a local-first quality scanner for Rust projects.

#### Collectors
- **fmt**: Runs `cargo fmt --check` to verify code formatting
- **clippy**: Runs `cargo clippy` with JSON output parsing for warning counts
- **tests**: Runs `cargo test` (or `cargo nextest` if available) with result parsing
- **coverage**: Runs `cargo llvm-cov --json` for line coverage percentage
- **deny**: Runs `cargo deny check --format=json` for banned crates and license violations
- **audit**: Runs `cargo audit --json` for security vulnerability detection
- **hack**: Runs `cargo hack check --feature-powerset` for feature combination testing
- **mutants**: Runs `cargo mutants` with outcomes.json parsing for mutation testing

#### CLI Subcommands
- `init`: Create quality/ directory with skeleton baseline.json
- `collect`: Run all collectors and write metricsSummary.json
- `gate`: Compare metrics against baseline, write qualityReport.json, exit with code
- `qa`: Run collect then gate (default when no subcommand given)
- `init-baseline`: Create baseline.json from current metricsSummary.json
- `update-baseline`: Update baseline.json from current metricsSummary.json (prints diff)
- `doctor`: Check which collectors are available on $PATH

#### Configuration
- `rustquty.toml` support with sections for profile defaults, collector toggles, and gate overrides
- Precedence: CLI flags > rustquty.toml > built-in defaults

#### Output
- Human-readable terminal output with unicode status markers (✓, ✗, ○, ⚠)
- JSON output mode (`--json`) for machine parsing
- Newline-terminated UTF-8 JSON for all quality JSON files

#### Quality Gates
- Schema version guards on all JSON documents
- Ratchet model for threshold comparisons (equal values pass)
- Baseline initialization and update with diff output

### Exit Codes
- `0`: All quality checks passed
- `1`: Quality regression detected (violations found)
- `2`: Configuration or execution error

### Installation
- `cargo install rustquty`
- Pre-built binaries via GitHub Releases (future)

### Dependencies (by collector)
| Collector | Tool | Installation if needed |
|-----------|------|------------------------|
| fmt | `rustfmt` | Included in Rust toolchain |
| clippy | `clippy` | Included in Rust toolchain |
| tests | `cargo nextest` | `cargo install cargo-nextest` |
| coverage | `cargo-llvm-cov` | `cargo install cargo-llvm-cov` |
| deny | `cargo-deny` | `cargo install cargo-deny` |
| audit | `cargo-audit` | `cargo install cargo-audit` |
| hack | `cargo-hack` | `cargo install cargo-hack` |
| mutants | `cargo-mutants` | `cargo install cargo-mutants` |

[0.1.0]: https://github.com/enrell/rustquty/releases/tag/v0.1.0
[0.2.0]: https://github.com/enrell/rustquty/releases/tag/v0.2.0
[0.3.0]: https://github.com/enrell/rustquty/releases/tag/v0.3.0
[0.3.1]: https://github.com/enrell/rustquty/releases/tag/v0.3.1
[0.4.0]: https://github.com/enrell/rustquty/releases/tag/v0.4.0
[0.4.3]: https://github.com/enrell/rustquty/releases/tag/v0.4.3
