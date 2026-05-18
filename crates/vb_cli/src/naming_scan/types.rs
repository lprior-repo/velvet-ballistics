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
