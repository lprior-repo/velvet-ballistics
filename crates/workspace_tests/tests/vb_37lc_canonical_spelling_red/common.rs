#![allow(dead_code)]

pub(crate) use std::path::PathBuf;

pub(crate) use vb_cli::naming_scan::{
    AllowlistPolicy, CanonicalEntry, CanonicalNameKind, CanonicalSpellingTable, ColumnNumber,
    LegacyAllowRule, LegacyException, LineNumber, NamingFinding, NamingScanError, OccurrenceClass,
    RawScanConfig, RenderedReport, RepoPath, RepoRoot, ScanConfig, ScanInput, ScanReport,
    SpellingClass, canonical_spelling_table, classify_occurrence, discover_scan_inputs,
    render_scan_report, scan_file, scan_repository, validate_scan_config,
};

pub(crate) const CANONICAL_HYPHEN: &str = "velvet-ballistics";
pub(crate) const CANONICAL_UNDERSCORE: &str = "vb_cli";
pub(crate) const CANONICAL_LANGUAGE_VERSION: &str = "velvet-ballistics/v1";
pub(crate) const LEGACY_PROJECT: &str = "velvet-ballistics";
pub(crate) const CURRENT_REPOSITORY_PATH: &str =
    "https://github.com/priorlewis43/velvet-ballistics";
pub(crate) const CURRENT_MASTER_FILENAME: &str = "velvet-ballistics-MASTER.md";
pub(crate) const MIGRATION_LABEL: &str = "MIGRATION-REFERENCE";
pub(crate) const MIGRATION_ARTIFACT: &str = "external-preexisting-artifact";

#[path = "common_part_01.rs"]
mod common_part_01;
pub(crate) use common_part_01::*;
#[path = "common_part_02.rs"]
mod common_part_02;
pub(crate) use common_part_02::*;
