mod allowlist;
mod classify;
mod config;
mod discovery;
mod legacy;
mod line_scan;
mod ordering;
mod report;
mod repository;
mod types;

pub(crate) use classify::classify_occurrence;
pub(crate) use config::{canonical_spelling_table, validate_scan_config};
pub(crate) use discovery::discover_scan_inputs;
pub(crate) use line_scan::scan_file;
pub(crate) use report::render_scan_report;
pub(crate) use repository::scan_repository;
pub(crate) use types::{
    AllowlistPolicy, CanonicalEntry, CanonicalNameKind, CanonicalSpellingTable, ColumnNumber,
    LegacyAllowRule, LegacyException, LineNumber, NamingFinding, NamingScanError, OccurrenceClass,
    RawScanConfig, RenderedReport, RepoPath, RepoRoot, ScanConfig, ScanInput, ScanReport,
    SpellingClass,
};
