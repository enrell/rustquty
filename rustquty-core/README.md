# rustquty-core

Core library for [rustquty](https://github.com/rustquty/rustquty), a local-first quality scanner for Rust projects.

## Collectors

This crate provides the following quality collectors:

| Collector | Description |
|-----------|-------------|
| `fmt` | Code formatting via `cargo fmt --check` |
| `clippy` | Linting via `cargo clippy` |
| `tests` | Test execution via `cargo test` or `cargo nextest` |
| `coverage` | Code coverage via `cargo llvm-cov` |
| `deny` | Banned crates & license checks via `cargo deny` |
| `audit` | Security vulnerability scanning via `cargo audit` |
| `hack` | Feature powerset testing via `cargo hack` |
| `mutants` | Mutation testing via `cargo mutants` |
| `duplicates` | Built-in duplicate line detection |
| `loc` | Lines of code metrics + line length enforcement |
| `size` | Per-file and per-function size metrics via AST analysis |

## Features

- **Zero network I/O**: All collectors execute local Cargo subcommands
- **Parallel execution**: Collectors run concurrently when using `full` or `deep` profiles
- **TOML configuration**: Gates can be configured via `rustquty.toml`
- **Baseline ratchet model**: Thresholds compared against established baselines
- **AST-based analysis**: Uses `syn` v2 for precise function-level metrics

## Usage

```rust
use rustquty_core::{Context, Profile, Gate};
use rustquty_core::collector::Collector;

let ctx = Context::new("/path/to/rust/project".into())
    .with_profile(Profile::Full)
    .with_output_dir("quality".into());

// Collectors implement the Collector trait
for col in collectors {
    if col.is_available() {
        let result = col.collect(&ctx)?;
    }
}
```

## License

[MIT](https://github.com/rustquty/rustquty/blob/main/LICENSE)