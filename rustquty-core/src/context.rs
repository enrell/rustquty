//! Runtime context passed to collectors.

use std::path::PathBuf;

/// Active quality scan profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// Fast scan: fmt + clippy only.
    Fast,
    /// Full scan: all collectors except slow ones like mutants.
    #[default]
    Full,
    /// Deep scan: all collectors including mutation testing.
    Deep,
}

impl std::str::FromStr for Profile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fast" => Ok(Profile::Fast),
            "full" => Ok(Profile::Full),
            "deep" => Ok(Profile::Deep),
            _ => Err(format!("unknown profile: {}", s)),
        }
    }
}

/// Collector names that can be skipped via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorName {
    Fmt,
    Clippy,
    Tests,
    Coverage,
    Deny,
    Audit,
    Hack,
    Mutants,
}

impl std::fmt::Display for CollectorName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectorName::Fmt => write!(f, "fmt"),
            CollectorName::Clippy => write!(f, "clippy"),
            CollectorName::Tests => write!(f, "tests"),
            CollectorName::Coverage => write!(f, "coverage"),
            CollectorName::Deny => write!(f, "deny"),
            CollectorName::Audit => write!(f, "audit"),
            CollectorName::Hack => write!(f, "hack"),
            CollectorName::Mutants => write!(f, "mutants"),
        }
    }
}

impl std::str::FromStr for CollectorName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fmt" => Ok(CollectorName::Fmt),
            "clippy" => Ok(CollectorName::Clippy),
            "tests" => Ok(CollectorName::Tests),
            "coverage" => Ok(CollectorName::Coverage),
            "deny" => Ok(CollectorName::Deny),
            "audit" => Ok(CollectorName::Audit),
            "hack" => Ok(CollectorName::Hack),
            "mutants" => Ok(CollectorName::Mutants),
            _ => Err(format!("unknown collector: {}", s)),
        }
    }
}

/// Runtime context for collector execution.
#[derive(Debug, Clone)]
pub struct Context {
    /// Root of the Cargo workspace being scanned.
    pub workspace_root: PathBuf,
    /// Active quality scan profile.
    pub profile: Profile,
    /// Explicitly disabled collectors.
    pub disabled_collectors: Vec<CollectorName>,
    /// Directory where quality JSON files are written.
    pub output_dir: PathBuf,
}

impl Context {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            profile: Profile::default(),
            disabled_collectors: Vec::new(),
            output_dir: PathBuf::from("quality"),
        }
    }

    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = dir;
        self
    }

    pub fn disable_collector(&mut self, name: CollectorName) {
        self.disabled_collectors.push(name);
    }

    pub fn is_collector_disabled(&self, name: CollectorName) -> bool {
        self.disabled_collectors.contains(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_parsing() {
        assert_eq!("fast".parse::<Profile>().unwrap(), Profile::Fast);
        assert_eq!("FULL".parse::<Profile>().unwrap(), Profile::Full);
        assert_eq!("Deep".parse::<Profile>().unwrap(), Profile::Deep);
        assert!("unknown".parse::<Profile>().is_err());
    }

    #[test]
    fn test_context_disabled_collectors() {
        let mut ctx = Context::new(PathBuf::from("/tmp"));
        assert!(!ctx.is_collector_disabled(CollectorName::Clippy));
        ctx.disable_collector(CollectorName::Clippy);
        assert!(ctx.is_collector_disabled(CollectorName::Clippy));
        assert!(!ctx.is_collector_disabled(CollectorName::Tests));
    }
}
