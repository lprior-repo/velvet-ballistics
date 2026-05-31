use std::path::PathBuf;

pub(crate) const CANONICAL_HYPHEN: &str = "velvet-ballastics";
pub(crate) const CANONICAL_UNDERSCORE: &str = "vb_cli";
pub(crate) const CANONICAL_LANGUAGE_VERSION: &str = "velvet-ballastics/v1";
pub(crate) const LEGACY_PROJECT: &str = "velvet-ballistics";
pub(crate) const LEGACY_CRATE: &str = "velvet_ballistics";
pub(crate) const LEGACY_LANGUAGE_VERSION: &str = "velvet-ballistics/v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalSpellingTable {
    pub product: String,
    pub binary: String,
    pub package: String,
    pub bead_rig: String,
    pub crate_module: String,
    pub bead_database: String,
    pub language_version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum CanonicalNameKind {
    Product,
    Binary,
    Package,
    BeadRig,
    CrateModule,
    BeadDatabase,
    LanguageVersion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalEntry {
    pub kind: CanonicalNameKind,
    pub token: String,
}

impl CanonicalEntry {
    #[must_use]
    pub fn new(kind: CanonicalNameKind, token: &str) -> Self {
        Self {
            kind,
            token: token.to_owned(),
        }
    }

    #[must_use]
    pub fn replace_token_when_kind_matches(self, kind: CanonicalNameKind, token: &str) -> Self {
        if self.kind == kind {
            Self::new(kind, token)
        } else {
            self
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LegacyAllowRule {
    RepositoryPath {
        path: String,
    },
    MasterFilename {
        filename: String,
    },
    MigrationReference {
        label: String,
        artifact: String,
        legacy_text: String,
    },
    Wildcard {
        pattern: String,
    },
    PrefixOnly {
        prefix: String,
    },
    Substring {
        needle: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AllowlistPolicy {
    Exact(Vec<LegacyAllowRule>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawScanConfig {
    pub canonical_entries: Vec<CanonicalEntry>,
    pub legacy_allowlist: Vec<LegacyAllowRule>,
    pub scan_patterns: Vec<String>,
    pub excluded_path_rules: Vec<String>,
    pub workspace_root: PathBuf,
    pub report_destination: Option<PathBuf>,
}

impl RawScanConfig {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            canonical_entries: Vec::new(),
            legacy_allowlist: Vec::new(),
            scan_patterns: Vec::new(),
            excluded_path_rules: Vec::new(),
            workspace_root: PathBuf::new(),
            report_destination: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanConfig {
    pub canonical_table: CanonicalSpellingTable,
    pub allowlist_policy: AllowlistPolicy,
    pub scan_patterns: Vec<String>,
    pub excluded_path_rules: Vec<String>,
    pub config_fingerprint: String,
    pub report_destination: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RepoPath(String);

impl RepoPath {
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }
}

impl std::fmt::Display for RepoPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoRoot(pub(crate) PathBuf);

impl RepoRoot {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LineNumber(pub(crate) u64);

impl LineNumber {
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ColumnNumber(pub(crate) u64);

impl ColumnNumber {
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum SpellingClass {
    LegacyProjectSpelling,
    LegacyCrateModuleSpelling,
    LegacyLanguageVersionSpelling,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamingFinding {
    pub path: RepoPath,
    pub line: LineNumber,
    pub column: ColumnNumber,
    pub spelling_class: SpellingClass,
    pub remediation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LegacyException {
    RepositoryPath {
        path: String,
    },
    MasterFilename {
        filename: String,
    },
    MigrationReference {
        artifact: String,
        label: String,
        legacy_text: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OccurrenceClass {
    NoOccurrence,
    CanonicalProduct {
        canonical: String,
        kind: CanonicalNameKind,
    },
    CanonicalCrateModule {
        canonical: String,
        kind: CanonicalNameKind,
    },
    CanonicalLanguageVersion {
        canonical: String,
        kind: CanonicalNameKind,
    },
    AllowedLegacy {
        exception: LegacyException,
    },
    InvalidLegacy {
        spelling_class: SpellingClass,
        remediation: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScanInput {
    Text {
        path: RepoPath,
        contents: String,
    },
    Bytes {
        path: RepoPath,
        bytes: Vec<u8>,
    },
    File {
        path: RepoPath,
        absolute_path: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanReport {
    pub root: RepoRoot,
    pub config_fingerprint: String,
    pub selected_input_count: usize,
    pub scanned_text_input_count: usize,
    pub findings: Vec<NamingFinding>,
    pub report_destination: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedReport {
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NamingScanError {
    InvalidRoot { root: RepoRoot },
    InvalidConfiguration { reason: String },
    FileDiscoveryFailed { path: RepoPath, source: String },
    InputReadFailed { path: RepoPath, source: String },
    PatternCompilationFailed { pattern: String, source: String },
    InvalidCanonicalSpelling { findings: Vec<NamingFinding> },
    ReportWriteFailed { path: PathBuf, source: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn canonical_entry_new_sets_fields() {
        let entry = CanonicalEntry::new(CanonicalNameKind::Product, "velvet-ballastics");
        assert_eq!(entry.kind, CanonicalNameKind::Product);
        assert_eq!(entry.token, "velvet-ballastics");
    }

    #[test]
    fn canonical_entry_replace_token_when_kind_matches_replaces() {
        let entry = CanonicalEntry::new(CanonicalNameKind::CrateModule, "old");
        let replaced = entry.replace_token_when_kind_matches(CanonicalNameKind::CrateModule, "new");
        assert_eq!(replaced.token, "new");
        assert_eq!(replaced.kind, CanonicalNameKind::CrateModule);
    }

    #[test]
    fn canonical_entry_replace_token_when_kind_differs_keeps_original() {
        let entry = CanonicalEntry::new(CanonicalNameKind::Product, "original");
        let replaced = entry.replace_token_when_kind_matches(CanonicalNameKind::CrateModule, "new");
        assert_eq!(replaced.token, "original");
        assert_eq!(replaced.kind, CanonicalNameKind::Product);
    }

    #[test]
    fn repo_path_new_stores_value() {
        let path = RepoPath::new("src/main.rs");
        assert_eq!(path.0, "src/main.rs");
    }

    #[test]
    fn repo_path_display_outputs_inner() {
        let path = RepoPath::new("src/lib.rs");
        assert_eq!(format!("{path}"), "src/lib.rs");
    }

    #[test]
    fn repo_root_new_stores_path() {
        let pb = PathBuf::from("/tmp/repo");
        let root = RepoRoot::new(pb.clone());
        assert_eq!(root.0, pb);
    }

    #[test]
    fn line_number_new_stores_value() {
        let ln = LineNumber::new(42);
        assert_eq!(ln.0, 42);
    }

    #[test]
    fn line_number_new_with_zero() {
        let ln = LineNumber::new(0);
        assert_eq!(ln.0, 0);
    }

    #[test]
    fn column_number_new_stores_value() {
        let cn = ColumnNumber::new(10);
        assert_eq!(cn.0, 10);
    }

    #[test]
    fn raw_scan_config_empty_has_no_entries() {
        let cfg = RawScanConfig::empty();
        assert!(cfg.canonical_entries.is_empty());
        assert!(cfg.legacy_allowlist.is_empty());
        assert!(cfg.scan_patterns.is_empty());
        assert!(cfg.excluded_path_rules.is_empty());
    }

    #[test]
    fn line_number_ordering() {
        let a = LineNumber::new(1);
        let b = LineNumber::new(2);
        assert!(a < b);
    }

    #[test]
    fn column_number_ordering() {
        let a = ColumnNumber::new(5);
        let b = ColumnNumber::new(10);
        assert!(a < b);
    }

    #[test]
    fn canonical_name_kind_ordering() {
        let a = CanonicalNameKind::Product;
        let b = CanonicalNameKind::Binary;
        assert_ne!(a, b);
    }

    #[test]
    fn spelling_class_debug_formats() {
        let sc = SpellingClass::LegacyProjectSpelling;
        assert_eq!(format!("{sc:?}"), "LegacyProjectSpelling");
    }

    #[test]
    fn naming_finding_equality() {
        let a = NamingFinding {
            path: RepoPath::new("a.rs"),
            line: LineNumber::new(1),
            column: ColumnNumber::new(2),
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: "correct".to_owned(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn naming_finding_inequality_by_line() {
        let a = NamingFinding {
            path: RepoPath::new("a.rs"),
            line: LineNumber::new(1),
            column: ColumnNumber::new(1),
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: "correct".to_owned(),
        };
        let b = NamingFinding {
            path: RepoPath::new("a.rs"),
            line: LineNumber::new(2),
            column: ColumnNumber::new(1),
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: "correct".to_owned(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn canonical_spelling_table_equality() {
        let a = CanonicalSpellingTable {
            product: "a".into(),
            binary: "a".into(),
            package: "a".into(),
            bead_rig: "a".into(),
            crate_module: "a".into(),
            bead_database: "a".into(),
            language_version: "a/v1".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn naming_scan_error_equality() {
        let a = NamingScanError::InvalidConfiguration {
            reason: "test".into(),
        };
        let b = NamingScanError::InvalidConfiguration {
            reason: "test".into(),
        };
        assert_eq!(a, b);
    }
}
