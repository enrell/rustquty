# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/rustquty/rustquty/releases/tag/v0.1.0