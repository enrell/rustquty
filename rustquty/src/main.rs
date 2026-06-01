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
    let all: Vec<Box<dyn rustquty_core::collector::Collector>> =
        collectors::all_collectors(size_config, complexity_config);

    // Apply profile filtering
    let enabled: Vec<Box<dyn rustquty_core::collector::Collector>> = all
        .into_iter()
        .filter(|col| is_collector_enabled(ctx, col.name()))
        .collect();

    let parallel = matches!(ctx.profile, Profile::Full | Profile::Deep);
    let results = rustquty_core::collector::execute_collectors(&enabled, ctx, parallel);

    let project_name = ctx
        .workspace_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let rust_edition = detect_rust_edition(&ctx.workspace_root);

    Ok(rustquty_core::collector::assemble_results(
        &results,
        &project_name,
        &rust_edition,
        &ctx.workspace_root.to_string_lossy(),
    ))
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


