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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigGateCoverage {
    #[serde(default)]
    pub min_line_percent: Option<f64>,
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
