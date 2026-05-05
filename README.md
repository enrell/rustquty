# rustquty

Local-first quality scanner for Rust projects.

## Features

- **12 Quality Collectors**: fmt, clippy, tests, coverage, deny, audit, hack, mutants, duplicates, loc, size, complexity
- **Line Length Enforcement**: Detects lines exceeding max line length (default 120 chars)
- **Code Duplication Detection**: Finds duplicate lines across Rust source files
- **Per-file & Per-function Size Metrics**: Lines, code lines, function parameter counts via AST analysis
- **Profile-based Scanning**: fast (fmt+clippy), full (all except mutants), deep (all)
- **Baseline Comparison**: Compare current metrics against established baselines
- **CI/CD Ready**: GitHub Actions integration with artifact upload on failure
- **Local-first**: No network I/O at runtime; all calls go to local Cargo subcommands

## Installation

### From source

```bash
cargo install rustquty
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/enrell/rustquty/releases).

## Quick Start

```bash
# Navigate to your Rust project
cd my-rust-project

# Initialize quality directory with baseline
rustquty init

# Run quality scan (collect + gate)
rustquty qa

# Or run step by step
rustquty collect
rustquty gate
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All quality checks passed |
| 1 | Quality regression detected |
| 2 | Configuration or execution error |

## Usage

```
rustquty [OPTIONS] [COMMAND]

Commands:
  init             Create quality/ directory with empty baseline.json
  collect          Run collectors and write metricsSummary.json
  gate             Compare metrics against baseline, write qualityReport.json, exit with code
  qa               Run 'collect' then 'gate' (default when no subcommand given)
  init-baseline    Create baseline.json from current metricsSummary.json
  update-baseline  Update baseline.json from current metricsSummary.json (prints diff)
  doctor           Check which collectors are available on $PATH
  help             Print this message or the help of the given subcommand(s)

Options:
      --project-dir <PROJECT_DIR>
          Working directory of the Cargo workspace (default: cwd)
      --output-dir <OUTPUT_DIR>
          Directory for quality JSON output (default: <project-dir>/quality)
      --profile <PROFILE>
          Quality scan profile: fast (fmt+clippy), full (all except mutants), deep (all) [default: full]
      --json
          Output JSON to stdout instead of human-readable format
      --disable-collector <DISABLE_COLLECTOR>
          Disable a specific collector (can be specified multiple times)
  -h, --help
          Print help
  -V, --version
          Print version
```

## Configuration

Create `rustquty.toml` in your project root:

```toml
[profile]
default = "full"

[collectors]
mutants = false

[gate.coverage]
min_line_percent = 80.0

[gate.size]
max-lines-per-file = 500
max-code-lines-per-file = 400
max-lines-per-function = 80
max-parameters-per-function = 5

[gate.complexity]
max-cyclomatic-per-function = 10
max-nesting-depth = 5

[output]
dir = "quality"
```

Precedence: CLI flags > rustquty.toml > built-in defaults

## Output Files

| File | Description |
|------|-------------|
| `metricsSummary.json` | Current metrics from all collectors |
| `baseline.json` | Threshold values for gate comparison |
| `qualityReport.json` | Gate result with violations if any |

## GitHub Actions

Add to your workflow:

```yaml
jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Run rustquty
        uses: ./.github/actions/rustquty
        with:
          profile: full
```

Or use the composite action from this repository:

```yaml
- uses: rustquty/rustquty/.github/actions/rustquty@main
```

## Available Collectors

| Collector | Tool | Description |
|-----------|------|-------------|
| fmt | `cargo fmt --check` | Code formatting |
| clippy | `cargo clippy` | Linting |
| tests | `cargo test`/`nextest` | Test execution |
| coverage | `cargo llvm-cov` | Code coverage |
| deny | `cargo deny` | Banned crates & licenses |
| audit | `cargo audit` | Security vulnerabilities |
| hack | `cargo hack` | Feature powerset |
| mutants | `cargo mutants` | Mutation testing |
| duplicates | (built-in) | Detects duplicate lines across files |
| loc | (built-in) | Lines of code metrics + line length enforcement |
| size | (built-in) | Per-file and per-function size metrics via AST |
| complexity | (built-in) | Cyclomatic complexity and nesting depth via AST |

## License

[MIT](LICENSE)