mod allowlist;
mod classify;
mod config_build;
mod config_build_tests;
mod config_types;
mod config_validate;
mod config_validate_tests;
mod discovery;
mod legacy;
mod line_scan;
mod ordering;
mod report;
mod repository;
mod types;

pub use classify::classify_occurrence;
pub use config_build::canonical_spelling_table;
pub use config_validate::validate_scan_config;
pub use discovery::discover_scan_inputs;
pub use line_scan::scan_file;
pub use report::render_scan_report;
pub use repository::scan_repository;
pub use types::{
    AllowlistPolicy, CanonicalEntry, CanonicalNameKind, CanonicalSpellingTable, ColumnNumber,
    LegacyAllowRule, LegacyException, LineNumber, NamingFinding, NamingScanError, OccurrenceClass,
    RawScanConfig, RenderedReport, RepoPath, RepoRoot, ScanConfig, ScanInput, ScanReport,
    SpellingClass,
};
