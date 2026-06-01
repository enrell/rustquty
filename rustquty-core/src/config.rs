//! Configuration file parsing (`rustquty.toml`).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub profile: ConfigProfile,
    #[serde(default)]
    pub collectors: ConfigCollectors,
    #[serde(default)]
    pub gate: ConfigGate,
    #[serde(default)]
    pub output: ConfigOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigProfile {
    #[serde(default = "default_profile_default")]
    pub default: String,
}

fn default_profile_default() -> String {
    "full".to_string()
}

impl Default for ConfigProfile {
    fn default() -> Self {
        Self {
            default: default_profile_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigCollectors {
    #[serde(default)]
    pub mutants: Option<bool>,
    #[serde(default)]
    pub hack: Option<bool>,
    #[serde(default)]
    pub coverage: Option<bool>,
    #[serde(default)]
    pub deny: Option<bool>,
    #[serde(default)]
    pub audit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigGate {
    #[serde(default)]
    pub coverage: Option<ConfigGateCoverage>,
    #[serde(default)]
    pub size: Option<SizeConfig>,
    #[serde(default)]
    pub complexity: Option<ComplexityConfig>,
    #[serde(default)]
    pub defaults: Option<ConfigGateDefaults>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigGateCoverage {
    #[serde(default)]
    pub min_line_percent: Option<f64>,
}

/// Absolute thresholds based on industry standards (SonarQube, ESLint, DeepSource).
/// When present, these override the ratchet model for the specified metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigGateDefaults {
    /// Maximum cyclomatic complexity per function (SonarQube default: 15, DeepSource: 10).
    #[serde(default)]
    pub max_cyclomatic_per_function: Option<u32>,
    /// Maximum nesting depth per function (ESLint default: 4, Detekt/ReSharper: 5).
    #[serde(default)]
    pub max_nesting_depth: Option<u32>,
    /// Maximum lines per function (SonarQube default: 80, ESLint: 50, Detekt: 60).
    #[serde(default)]
    pub max_lines_per_function: Option<u32>,
    /// Maximum lines per file (SonarQube default: 1000).
    #[serde(default)]
    pub max_lines_per_file: Option<u32>,
    /// Maximum code lines per file (non-comment, non-blank).
    #[serde(default)]
    pub max_code_lines_per_file: Option<u32>,
    /// Maximum parameters per function (SonarQube default: 7, Detekt: 6, ESLint: 3).
    #[serde(default)]
    pub max_parameters_per_function: Option<u32>,
    /// Minimum line coverage percent (SonarQube default: 80.0).
    #[serde(default)]
    pub min_coverage_percent: Option<f64>,
    /// Maximum duplicate lines (SonarQube default: 3% of new code).
    #[serde(default)]
    pub max_duplicate_lines: Option<u32>,
    /// Maximum clippy warnings.
    #[serde(default)]
    pub max_clippy_warnings: Option<u32>,
    /// Maximum line length in characters (ESLint default: 80, rustfmt default: 120).
    #[serde(default)]
    pub max_line_length: Option<usize>,
}

/// Configuration for the size gate, loaded from [gate.size] in TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct SizeConfig {
    /// Maximum total lines per file.
    #[serde(default)]
    pub max_lines_per_file: Option<u32>,
    /// Maximum code lines per file (non-comment, non-blank).
    #[serde(default)]
    pub max_code_lines_per_file: Option<u32>,
    /// Maximum lines per function.
    #[serde(default)]
    pub max_lines_per_function: Option<u32>,
    /// Maximum parameters per function.
    #[serde(default)]
    pub max_parameters_per_function: Option<u32>,
}

/// Configuration for the complexity gate, loaded from [gate.complexity] in TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ComplexityConfig {
    /// Maximum cyclomatic complexity per function.
    #[serde(default)]
    pub max_cyclomatic_per_function: Option<u32>,
    /// Maximum nesting depth per function.
    #[serde(default)]
    pub max_nesting_depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigOutput {
    #[serde(default)]
    pub dir: Option<String>,
}

impl Config {
    /// Load `rustquty.toml` from the given directory.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_CONFIG: &str = r#"
[profile]
default = "deep"

[collectors]
mutants = false

[gate.coverage]
min_line_percent = 80.0

[output]
dir = "quality"
"#;

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
        assert_eq!(config.profile.default, "deep");
        assert_eq!(config.collectors.mutants, Some(false));
        assert_eq!(
            config.gate.coverage.as_ref().unwrap().min_line_percent,
            Some(80.0)
        );
        assert_eq!(config.output.dir.as_deref(), Some("quality"));
    }
}
