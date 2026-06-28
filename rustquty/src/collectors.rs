//! Registry of all collectors.

use rustquty_core::collector::{
    Collector, audit::AuditCollector, clippy::ClippyCollector, complexity::ComplexityCollector,
    coverage::CoverageCollector, deny::DenyCollector, duplicates::DuplicatesCollector,
    fmt::FmtCollector, hack::HackCollector, loc::LocCollector, mutants::MutantsCollector,
    size::SizeCollector, tests::TestCollector,
};
use rustquty_core::config::{ComplexityConfig, LocConfig, SizeConfig};

pub fn all_collectors(
    size_config: Option<SizeConfig>,
    complexity_config: Option<ComplexityConfig>,
    loc_config: Option<LocConfig>,
) -> Vec<Box<dyn Collector>> {
    vec![
        Box::new(FmtCollector::new()),
        Box::new(ClippyCollector::new()),
        Box::new(TestCollector::new()),
        Box::new(CoverageCollector::new()),
        Box::new(DenyCollector::new()),
        Box::new(AuditCollector::new()),
        Box::new(HackCollector::new()),
        Box::new(MutantsCollector::new()),
        Box::new(DuplicatesCollector::new()),
        Box::new(match loc_config {
            Some(cfg) => LocCollector::with_config(cfg),
            None => LocCollector::new(),
        }),
        Box::new(match size_config {
            Some(cfg) => SizeCollector::with_config(cfg),
            None => SizeCollector::new(),
        }),
        Box::new(match complexity_config {
            Some(cfg) => ComplexityCollector::with_config(cfg),
            None => ComplexityCollector::new(),
        }),
    ]
}
