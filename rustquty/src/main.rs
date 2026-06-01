//! rustquty CLI — local-first quality scanner for Rust projects.

mod collectors;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rustquty_core::{
    BaselineWriter, Gate,
    config::{ComplexityConfig, Config, SizeConfig},
    context::{CollectorName, Context, Profile},
    schema::{CollectorStatus, GateResult, MetricsSummary, QualityReport},
};
use std::path::PathBuf;

/// Detect Rust edition from Cargo.toml
fn detect_rust_edition(workspace_root: &PathBuf) -> String {
    // First try workspace Cargo.toml
    let cargo_toml = workspace_root.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo_toml)
        && let Some(edition) = parse_edition_from_content(&content)
    {
        return edition;
    }

    // Try looking for member crate Cargo.toml (rustquty/Cargo.toml or rustquty-core/Cargo.toml)
    if let Ok(entries) = std::fs::read_dir(workspace_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let member_cargo = path.join("Cargo.toml");
                if let Ok(content) = std::fs::read_to_string(&member_cargo)
                    && content.contains("[package]")
                    && content.contains("edition")
                    && let Some(edition) = parse_edition_from_content(&content)
                {
                    return edition;
                }
            }
        }
    }

    "2021".to_string()
}

fn parse_edition_from_content(content: &str) -> Option<String> {
    let mut in_package_section = false;
    let mut in_workspace_package = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Track which section we're in
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            in_package_section = section == "package";
            in_workspace_package = section == "workspace.package";
        }

        // Look for edition in [package] or [workspace.package] section
        let in_relevant_section = in_package_section || in_workspace_package;
        if in_relevant_section
            && trimmed.starts_with("edition")
            && trimmed.contains('=')
            && let Some(eq_pos) = trimmed.find('=')
        {
            let value = trimmed[eq_pos + 1..].trim();
            let edition =
                value.trim_matches(|c| c == ',' || c == '"' || c == ' ' || c == '\n' || c == '\r');
            if !edition.is_empty() {
                return Some(edition.to_string());
            }
        }
    }
    None
}

#[derive(Parser, Debug)]
#[command(name = "rustquty")]
#[command(version)]
#[command(about = "Local-first quality scanner for Rust projects")]
struct Cli {
    /// Working directory of the Cargo workspace (default: cwd)
    #[arg(long, global = true)]
    project_dir: Option<PathBuf>,

    /// Directory for quality JSON output (default: <project-dir>/quality)
    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    /// Quality scan profile: fast (fmt+clippy), full (all except mutants), deep (all)
    #[arg(long, global = true, default_value = "full")]
    profile: String,

    /// Output JSON to stdout instead of human-readable format
    #[arg(long, global = true)]
    json: bool,

    /// Disable a specific collector (can be specified multiple times)
    #[arg(long, global = true, value_delimiter = ',')]
    disable_collector: Vec<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create quality/ directory with empty baseline.json
    Init,

    /// Run collectors and write metricsSummary.json
    Collect,

    /// Compare metrics against baseline, write qualityReport.json, exit with code
    Gate,

    /// Run 'collect' then 'gate' (default when no subcommand given)
    Qa,

    /// Create baseline.json from current metricsSummary.json
    InitBaseline {
        /// Overwrite existing baseline without prompting
        #[arg(long)]
        force: bool,
    },

    /// Update baseline.json from current metricsSummary.json (prints diff)
    UpdateBaseline,

    /// Check which collectors are available on $PATH
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let project_dir = cli
        .project_dir
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let output_dir = cli
        .output_dir
        .unwrap_or_else(|| project_dir.join("quality"));

    // Load rustquty.toml config if present (Phase 6)
    let config_path = project_dir.join("rustquty.toml");
    let config = if config_path.exists() {
        Config::load(&config_path).ok()
    } else {
        None
    };

    let profile: Profile = cli.profile.parse().unwrap_or_else(|_| {
        config
            .as_ref()
            .and_then(|c| c.profile.default.parse().ok())
            .unwrap_or(Profile::Full)
    });

    // Extract size config from TOML if present.
    let size_config = config.as_ref().and_then(|c| c.gate.size.clone());
    let complexity_config = config.as_ref().and_then(|c| c.gate.complexity.clone());

    // Build context with CLI overrides
    let mut ctx = Context::new(project_dir.clone())
        .with_profile(profile)
        .with_output_dir(output_dir.clone());

    // Apply --disable-collector flags
    for name in &cli.disable_collector {
        if let Ok(cn) = name.parse() {
            ctx.disable_collector(cn);
        }
    }

    // Default to Qa if no subcommand given
    let cmd = cli.command.unwrap_or(Commands::Qa);

    match cmd {
        Commands::Init => {
            std::fs::create_dir_all(&output_dir)?;
            let baseline_path = output_dir.join("baseline.json");
            if baseline_path.exists() {
                println!(
                    "baseline.json already exists at {}",
                    baseline_path.display()
                );
            } else {
                println!("Creating baseline at {}", baseline_path.display());
            }
        }

        Commands::Collect => {
            let summary = run_collectors(&ctx, size_config.clone(), complexity_config.clone())?;
            let json = serde_json::to_string_pretty(&summary)?;
            let path = output_dir.join("metricsSummary.json");
            std::fs::write(&path, &json)?;
            if cli.json {
                println!("{}", json);
            } else {
                print_human_summary(&summary);
            }
        }

        Commands::Gate => {
            let summary_path = output_dir.join("metricsSummary.json");
            let baseline_path = output_dir.join("baseline.json");

            let summary: MetricsSummary =
                serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
            let baseline: rustquty_core::schema::Baseline =
                serde_json::from_str(&std::fs::read_to_string(&baseline_path)?)?;

            let report = Gate::run(&summary, &baseline);
            let json = serde_json::to_string_pretty(&report)?;
            let path = output_dir.join("qualityReport.json");
            std::fs::write(&path, &json)?;

            if cli.json {
                println!("{}", json);
            } else {
                print_human_report(&report);
            }

            if matches!(report.gate_result, GateResult::Fail) {
                std::process::exit(1);
            }
        }

        Commands::Qa => {
            let summary = run_collectors(&ctx, size_config.clone(), complexity_config.clone())?;
            let json = serde_json::to_string_pretty(&summary)?;
            let path = output_dir.join("metricsSummary.json");
            std::fs::write(&path, &json)?;

            let baseline_path = output_dir.join("baseline.json");
            if !baseline_path.exists() {
                BaselineWriter::init(&summary, &baseline_path, false)?;
            }
            let baseline: rustquty_core::schema::Baseline =
                serde_json::from_str(&std::fs::read_to_string(&baseline_path)?)?;

            let report = Gate::run(&summary, &baseline);
            let report_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(output_dir.join("qualityReport.json"), &report_json)?;

            if cli.json {
                println!("{}", report_json);
            } else {
                print_human_report(&report);
            }

            if matches!(report.gate_result, GateResult::Fail) {
                std::process::exit(1);
            }
        }

        Commands::InitBaseline { force } => {
            let summary_path = output_dir.join("metricsSummary.json");
            let summary: MetricsSummary =
                serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
            let baseline_path = output_dir.join("baseline.json");
            BaselineWriter::init(&summary, &baseline_path, force)?;
            println!("Baseline written to {}", baseline_path.display());
        }

        Commands::UpdateBaseline => {
            let summary_path = output_dir.join("metricsSummary.json");
            let summary: MetricsSummary =
                serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
            let baseline_path = output_dir.join("baseline.json");
            BaselineWriter::update(&summary, &baseline_path)?;
            println!("Baseline updated at {}", baseline_path.display());
        }

        Commands::Doctor => {
            let all = collectors::all_collectors(None, None);
            println!("rustquty {} — collector availability\n", env!("CARGO_PKG_VERSION"));
            for col in all {
                let available = col.is_available();
                let mark = if available { "✓" } else { "✗" };
                println!(
                    "  {} {:<12} {}",
                    mark,
                    col.name(),
                    if available { "available" } else { "not found" }
                );
            }
        }
    }

    Ok(())
}

fn run_collectors(
    ctx: &Context,
    size_config: Option<SizeConfig>,
    complexity_config: Option<ComplexityConfig>,
) -> Result<MetricsSummary> {
    use rustquty_core::collector::Collector;

    let all: Vec<Box<dyn Collector>> =
        collectors::all_collectors(size_config.clone(), complexity_config.clone());

    // Apply profile filtering
    let enabled: Vec<Box<dyn Collector>> = all
        .into_iter()
        .filter(|col| is_collector_enabled(ctx, col.name()))
        .collect();

    let parallel = matches!(ctx.profile, Profile::Full | Profile::Deep);

    let mut results: Vec<(&str, rustquty_core::collector::CollectorOutput)> = Vec::new();

    if parallel {
        use rayon::prelude::*;
        let enabled_refs: Vec<&Box<dyn rustquty_core::collector::Collector>> =
            enabled.iter().collect();
        let collected: Vec<(&str, rustquty_core::collector::CollectorOutput)> = enabled_refs
            .par_iter()
            .flat_map(|col| match col.collect(ctx) {
                Ok(o) => vec![(col.name(), o)],
                Err(e) => {
                    let output = rustquty_core::collector::CollectorOutput {
                        status: CollectorStatus::Error,
                        duration_ms: 0,
                        stdout: String::new(),
                        stderr: format!("{:?}", e),
                    };
                    vec![(col.name(), output)]
                }
            })
            .collect();
        results = collected;
    } else {
        for col in &enabled {
            if !col.is_available() {
                continue;
            }
            match col.collect(ctx) {
                Ok(o) => results.push((col.name(), o)),
                Err(e) => {
                    let output = rustquty_core::collector::CollectorOutput {
                        status: CollectorStatus::Error,
                        duration_ms: 0,
                        stdout: String::new(),
                        stderr: format!("{:?}", e),
                    };
                    results.push((col.name(), output));
                }
            }
        }
    }

    let project_name = ctx
        .workspace_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut fmt_result = rustquty_core::schema::FmtResult {
        status: CollectorStatus::Skipped,
        details: Default::default(),
    };
    let mut clippy_result = rustquty_core::schema::ClippyResult {
        status: CollectorStatus::Skipped,
        warning_count: 0,
        details: vec![],
    };
    let mut test_result = rustquty_core::schema::TestResult {
        status: CollectorStatus::Skipped,
        passed: 0,
        failed: 0,
        ignored: 0,
        runner: None,
    };
    let mut coverage_result = rustquty_core::schema::CoverageResult {
        status: CollectorStatus::Skipped,
        line_percent: 0.0,
    };
    let mut deny_result = rustquty_core::schema::DenyResult {
        status: CollectorStatus::Skipped,
        banned_count: 0,
        license_violations: 0,
    };
    let mut audit_result = rustquty_core::schema::AuditResult {
        status: CollectorStatus::Skipped,
        vulnerability_count: 0,
        critical_count: 0,
    };
    let mut hack_result = rustquty_core::schema::HackResult {
        status: CollectorStatus::Skipped,
        feature_combinations_tested: 0,
    };
    let mut mutants_result = rustquty_core::schema::MutantsResult {
        status: CollectorStatus::Skipped,
        mutation_score: 0.0,
        caught: 0,
        missed: 0,
    };
    let mut duplicates_result = rustquty_core::schema::DuplicatesResult {
        status: CollectorStatus::Skipped,
        total_lines: 0,
        duplicate_lines: 0,
        files_with_duplicates: 0,
        duplicate_files: vec![],
    };
    let mut loc_result = rustquty_core::schema::LocResult {
        status: CollectorStatus::Skipped,
        total_lines: 0,
        code_lines: 0,
        comment_lines: 0,
        blank_lines: 0,
        long_lines: 0,
        max_line_length_found: 0,
        max_line_length_allowed: 120,
        files: 0,
        files_with_long_lines: 0,
        long_line_files: vec![],
    };
    let mut size_result = rustquty_core::schema::SizeResult {
        status: CollectorStatus::Skipped,
        files: 0,
        max_lines_per_file: 0,
        max_code_lines_per_file: 0,
        max_lines_per_function: 0,
        max_parameters_per_function: 0,
        violations: vec![],
    };
    let mut complexity_result = rustquty_core::schema::ComplexityResult {
        status: CollectorStatus::Skipped,
        functions: 0,
        max_cyclomatic_complexity: 0,
        max_nesting_depth: 0,
        complex_functions: 0,
        violations: vec![],
    };

    for (name, output) in &results {
        match *name {
            "fmt" => fmt_result.status.clone_from(&output.status),
            "clippy" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    clippy_result.warning_count =
                        details["warningCount"].as_u64().unwrap_or(0) as u32;
                    if let Some(arr) = details["details"].as_array() {
                        clippy_result.details = arr
                            .iter()
                            .map(|v| rustquty_core::schema::ClippyLint {
                                code: v["code"].as_str().unwrap_or("").to_string(),
                                message: v["message"].as_str().unwrap_or("").to_string(),
                                file: v["file"].as_str().map(String::from),
                                line: v["line"].as_u64().map(|v| v as u32),
                            })
                            .collect();
                    }
                }
                clippy_result.status.clone_from(&output.status);
            }
            "tests" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    test_result.passed = details["passed"].as_u64().unwrap_or(0) as u32;
                    test_result.failed = details["failed"].as_u64().unwrap_or(0) as u32;
                    test_result.ignored = details["ignored"].as_u64().unwrap_or(0) as u32;
                    test_result.runner = details["runner"].as_str().map(String::from);
                }
                test_result.status.clone_from(&output.status);
            }
            "coverage" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    coverage_result.line_percent =
                        details["linePercent"].as_f64().unwrap_or(0.0);
                }
                coverage_result.status.clone_from(&output.status);
            }
            "deny" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    deny_result.banned_count =
                        details["bannedCount"].as_u64().unwrap_or(0) as u32;
                    deny_result.license_violations =
                        details["licenseViolations"].as_u64().unwrap_or(0) as u32;
                }
                deny_result.status.clone_from(&output.status);
            }
            "audit" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    audit_result.vulnerability_count =
                        details["vulnerabilityCount"].as_u64().unwrap_or(0) as u32;
                    audit_result.critical_count =
                        details["criticalCount"].as_u64().unwrap_or(0) as u32;
                }
                audit_result.status.clone_from(&output.status);
            }
            "hack" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    hack_result.feature_combinations_tested =
                        details["featureCombinationsTested"].as_u64().unwrap_or(0) as u32;
                }
                hack_result.status.clone_from(&output.status);
            }
            "mutants" => mutants_result.status.clone_from(&output.status),
            "duplicates" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    duplicates_result.total_lines =
                        details["totalLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.duplicate_lines =
                        details["duplicateLines"].as_u64().unwrap_or(0) as u32;
                    duplicates_result.files_with_duplicates =
                        details["filesWithDuplicates"].as_u64().unwrap_or(0) as u32;
                }
                duplicates_result.status.clone_from(&output.status);
            }
            "loc" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    loc_result.total_lines = details["totalLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.code_lines = details["codeLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.comment_lines = details["commentLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.blank_lines = details["blankLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.long_lines = details["longLines"].as_u64().unwrap_or(0) as u32;
                    loc_result.max_line_length_found =
                        details["maxLineLengthFound"].as_u64().unwrap_or(0) as usize;
                    loc_result.files = details["files"].as_u64().unwrap_or(0) as u32;
                }
                loc_result.status.clone_from(&output.status);
            }
            "size" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    size_result.files = details["files"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_file =
                        details["maxLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_code_lines_per_file =
                        details["maxCodeLinesPerFile"].as_u64().unwrap_or(0) as u32;
                    size_result.max_lines_per_function =
                        details["maxLinesPerFunction"].as_u64().unwrap_or(0) as u32;
                    size_result.max_parameters_per_function =
                        details["maxParametersPerFunction"].as_u64().unwrap_or(0) as u32;
                }
                size_result.status.clone_from(&output.status);
            }
            "complexity" => {
                if let Ok(details) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                    complexity_result.functions = details["functions"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_cyclomatic_complexity =
                        details["maxCyclomaticComplexity"].as_u64().unwrap_or(0) as u32;
                    complexity_result.max_nesting_depth =
                        details["maxNestingDepth"].as_u64().unwrap_or(0) as u32;
                    complexity_result.complex_functions =
                        details["complexFunctions"].as_u64().unwrap_or(0) as u32;
                }
                complexity_result.status.clone_from(&output.status);
            }
            _ => {}
        }
    }

    Ok(MetricsSummary {
        schema_version: "1".to_string(),
        generated_at: rustquty_core::chrono_now(),
        rustquty_version: env!("CARGO_PKG_VERSION").to_string(),
        project: rustquty_core::schema::ProjectInfo {
            name: project_name,
            rust_edition: detect_rust_edition(&ctx.workspace_root),
            workspace_root: ctx.workspace_root.to_string_lossy().to_string(),
        },
        collectors: rustquty_core::schema::CollectorsSummary {
            fmt: fmt_result,
            clippy: clippy_result,
            tests: test_result,
            coverage: coverage_result,
            deny: deny_result,
            audit: audit_result,
            hack: hack_result,
            mutants: mutants_result,
            duplicates: duplicates_result,
            loc: loc_result,
            size: size_result,
            complexity: complexity_result,
        },
    })
}

fn is_collector_enabled(ctx: &Context, name: &str) -> bool {
    // Check if explicitly disabled via CLI flag
    if let Ok(cn) = name.parse::<CollectorName>()
        && ctx.is_collector_disabled(cn)
    {
        return false;
    }

    // Apply profile filtering
    match ctx.profile {
        Profile::Fast => {
            // Fast = fmt + clippy only
            name == "fmt" || name == "clippy"
        }
        Profile::Full => {
            // Full = all except mutants
            name != "mutants"
        }
        Profile::Deep => {
            // Deep = all collectors
            true
        }
    }
}

fn print_human_summary(summary: &MetricsSummary) {
    println!("rustquty {} — metrics", env!("CARGO_PKG_VERSION"));
    println!("project: {}", summary.project.name);
    println!("collectors:");
    let c = &summary.collectors;
    print!("  fmt        ");
    print_status(&c.fmt.status, "");
    print!("  clippy     ");
    print_status(
        &c.clippy.status,
        &format!("({} warnings)", c.clippy.warning_count),
    );
    print!("  tests      ");
    print_status(
        &c.tests.status,
        &format!(
            "({} passed, {} failed, {} ignored)",
            c.tests.passed, c.tests.failed, c.tests.ignored
        ),
    );
    print!("  coverage   ");
    print_status(
        &c.coverage.status,
        &format!("({:.1}%)", c.coverage.line_percent),
    );
    print!("  deny       ");
    print_status(
        &c.deny.status,
        &format!(
            "({} banned, {} license violations)",
            c.deny.banned_count, c.deny.license_violations
        ),
    );
    print!("  audit      ");
    print_status(
        &c.audit.status,
        &format!(
            "({} vulns, {} critical)",
            c.audit.vulnerability_count, c.audit.critical_count
        ),
    );
    print!("  hack       ");
    print_status(&c.hack.status, "");
    print!("  mutants    ");
    print_status(
        &c.mutants.status,
        &format!(
            "({:.1}% score, {} caught, {} missed)",
            c.mutants.mutation_score * 100.0,
            c.mutants.caught,
            c.mutants.missed
        ),
    );
    print!("  duplicates ");
    print_status(
        &c.duplicates.status,
        &format!(
            "({} total, {} dup)",
            c.duplicates.total_lines, c.duplicates.duplicate_lines
        ),
    );
    print!("  loc        ");
    print_status(
        &c.loc.status,
        &format!("({} lines, {} files)", c.loc.total_lines, c.loc.files),
    );
    print!("  size       ");
    print_status(
        &c.size.status,
        &format!(
            "({} files, {} max lines)",
            c.size.files, c.size.max_lines_per_file
        ),
    );
    print!("  complexity ");
    print_status(
        &c.complexity.status,
        &format!(
            "({} funcs, {} max cc)",
            c.complexity.functions, c.complexity.max_cyclomatic_complexity
        ),
    );
}

fn print_human_report(report: &QualityReport) {
    println!("────────────────────────────────────────────────────");
    let s = &report.summary;
    println!(
        "  collectors: {} run, {} passed, {} failed, {} skipped",
        s.collectors_run, s.collectors_passed, s.collectors_failed, s.collectors_skipped
    );

    println!("────────────────────────────────────────────────────");
    println!("  gate result: {:?}", report.gate_result);
    println!(
        "  exit code:   {}",
        if matches!(report.gate_result, GateResult::Fail) {
            1
        } else {
            0
        }
    );

    if !report.violations.is_empty() {
        println!("\nviolations:");
        for v in &report.violations {
            println!(
                "  - {}: {} (baseline: {}, current: {})",
                v.collector, v.metric, v.baseline_value, v.current_value
            );
        }
    }
}

fn print_status(status: &CollectorStatus, detail: &str) {
    let mark = match status {
        CollectorStatus::Pass => "✓ pass",
        CollectorStatus::Fail => "✗ FAIL",
        CollectorStatus::Skipped => "○ skip",
        CollectorStatus::Error => "⚠ error",
    };
    if detail.is_empty() {
        println!("{}", mark);
    } else {
        println!("{}      {}", mark, detail);
    }
}


