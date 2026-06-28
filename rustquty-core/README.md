# rustquty-core

Core library for [rustquty](https://github.com/enrell/rustquty), a local-first quality scanner for Rust projects.

## Collectors

This crate provides 12 quality collectors:

| Collector | Description | External tool |
|-----------|-------------|---------------|
| `fmt` | Code formatting | `cargo fmt` |
| `clippy` | Linting | `cargo clippy` |
| `tests` | Test execution | `cargo test` / `cargo nextest` |
| `coverage` | Code coverage | `cargo llvm-cov` |
| `deny` | Banned crates & licenses | `cargo deny` |
| `audit` | Security vulnerabilities | `cargo audit` |
| `hack` | Feature powerset testing | `cargo hack` |
| `mutants` | Mutation testing | `cargo mutants` |
| `duplicates` | Token-based duplicate block detection | built-in |
| `loc` | Lines of code + configurable line length | built-in |
| `size` | Per-file/per-function size (AST) | built-in |
| `complexity` | Cyclomatic complexity + nesting (AST) | built-in |

## Features

- **Zero network I/O**: All collectors execute local Cargo subcommands
- **Parallel execution**: Collectors run concurrently via `rayon`
- **TOML configuration**: Gates configured via `rustquty.toml`
- **Ratchet model**: Thresholds set from current metrics; gate fails if quality degrades
- **Absolute thresholds**: Industry-standard defaults (SonarQube, ESLint, Detekt)
- **AST-based analysis**: Uses `syn` v2 for precise function-level metrics

## API

```rust
use rustquty_core::{
    Context, Profile, Gate, GateConfig,
    collector::{execute_collectors, assemble_results},
};
use std::path::PathBuf;

// Create context
let ctx = Context::new(PathBuf::from("/path/to/project"))
    .with_profile(Profile::Full);

// Build collectors
let collectors = vec![
    // ... your collectors
];

// Execute and assemble
let results = execute_collectors(&collectors, &ctx, true);
let summary = assemble_results(&results, "my-project", "2024", "/path/to/project");

// Gate with ratchet baseline
let report = Gate::run(&summary, &baseline);

// Or gate with absolute thresholds (SonarQube standards)
let config = GateConfig {
    max_cyclomatic_per_function: Some(15),
    max_nesting_depth: Some(5),
    max_lines_per_function: Some(80),
    min_coverage_percent: Some(80.0),
    ..Default::default()
};
let report = Gate::run_with_config(&summary, &baseline, Some(&config));
```

## Modules

| Module | Description |
|--------|-------------|
| `collector` | Collector trait, implementations, execution, and assembly |
| `config` | `rustquty.toml` parsing |
| `context` | Runtime context (profile, paths, disabled collectors) |
| `gate` | Gate logic for comparing metrics against baselines |
| `schema` | JSON schemas for MetricsSummary, Baseline, QualityReport |
| `baseline` | Baseline file creation and update |
| `util` | Shared utilities (ISO-8601 timestamps) |

## License

[MIT](https://github.com/enrell/rustquty/blob/main/LICENSE)
