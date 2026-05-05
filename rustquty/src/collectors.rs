//! Registry of all collectors.

use rustquty_core::collector::{
    Collector, audit::AuditCollector, clippy::ClippyCollector, coverage::CoverageCollector,
    deny::DenyCollector, duplicates::DuplicatesCollector, fmt::FmtCollector,
    hack::HackCollector, loc::LocCollector, mutants::MutantsCollector, tests::TestCollector,
};

pub fn all_collectors() -> Vec<Box<dyn Collector>> {
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
        Box::new(LocCollector::new()),
    ]
}
