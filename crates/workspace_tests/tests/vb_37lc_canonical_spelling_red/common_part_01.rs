use super::*;

pub(crate) fn canonical_entries() -> Vec<CanonicalEntry> {
    vec![
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Binary, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Package, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::BeadRig, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::CrateModule, CANONICAL_UNDERSCORE),
        CanonicalEntry::new(CanonicalNameKind::BeadDatabase, CANONICAL_UNDERSCORE),
        CanonicalEntry::new(
            CanonicalNameKind::LanguageVersion,
            CANONICAL_LANGUAGE_VERSION,
        ),
    ]
}

pub(crate) fn exact_legacy_allowlist() -> Vec<LegacyAllowRule> {
    vec![
        LegacyAllowRule::RepositoryPath {
            path: CURRENT_REPOSITORY_PATH.to_string(),
        },
        LegacyAllowRule::MasterFilename {
            filename: CURRENT_MASTER_FILENAME.to_string(),
        },
        LegacyAllowRule::MigrationReference {
            label: MIGRATION_LABEL.to_string(),
            artifact: MIGRATION_ARTIFACT.to_string(),
            legacy_text: LEGACY_PROJECT.to_string(),
        },
    ]
}

pub(crate) fn raw_config_missing_kind(kind: CanonicalNameKind) -> RawScanConfig {
    let entries = canonical_entries()
        .into_iter()
        .filter(|entry| entry.kind != kind)
        .collect();
    RawScanConfig {
        canonical_entries: entries,
        legacy_allowlist: exact_legacy_allowlist(),
        scan_patterns: vec![LEGACY_PROJECT.to_string()],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        workspace_root: PathBuf::from("."),
        report_destination: None,
    }
}

pub(crate) fn raw_config_with_duplicate_kind(
    kind: CanonicalNameKind,
    token: &str,
) -> RawScanConfig {
    let mut raw = minimum_valid_raw_config();
    raw.canonical_entries.push(CanonicalEntry::new(kind, token));
    raw
}

pub(crate) fn raw_config_with_contradictory_token(
    kind: CanonicalNameKind,
    token: &str,
) -> RawScanConfig {
    let entries = canonical_entries()
        .into_iter()
        .map(|entry| entry.replace_token_when_kind_matches(kind, token))
        .collect();
    RawScanConfig {
        canonical_entries: entries,
        legacy_allowlist: exact_legacy_allowlist(),
        scan_patterns: vec![LEGACY_PROJECT.to_string()],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        workspace_root: PathBuf::from("."),
        report_destination: None,
    }
}

pub(crate) fn raw_config_with_allow_rule(rule: LegacyAllowRule) -> RawScanConfig {
    RawScanConfig {
        canonical_entries: canonical_entries(),
        legacy_allowlist: vec![rule],
        scan_patterns: vec![LEGACY_PROJECT.to_string()],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        workspace_root: PathBuf::from("."),
        report_destination: None,
    }
}

pub(crate) fn raw_config_with_scan_pattern(pattern: String) -> RawScanConfig {
    RawScanConfig {
        canonical_entries: canonical_entries(),
        legacy_allowlist: exact_legacy_allowlist(),
        scan_patterns: vec![pattern],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        workspace_root: PathBuf::from("."),
        report_destination: None,
    }
}

pub(crate) fn invalid_legacy_occurrence(remediation: &str) -> OccurrenceClass {
    OccurrenceClass::InvalidLegacy {
        spelling_class: SpellingClass::LegacyProjectSpelling,
        remediation: remediation.to_string(),
    }
}

pub(crate) fn finding(
    path: &str,
    line_number: u64,
    column_number: u64,
    remediation: &str,
) -> NamingFinding {
    finding_with_class(
        path,
        line_number,
        column_number,
        SpellingClass::LegacyProjectSpelling,
        remediation,
    )
}

pub(crate) fn finding_with_class(
    path: &str,
    line_number: u64,
    column_number: u64,
    spelling_class: SpellingClass,
    remediation: &str,
) -> NamingFinding {
    NamingFinding {
        path: repo_path(path),
        line: line(line_number),
        column: column(column_number),
        spelling_class,
        remediation: remediation.to_string(),
    }
}

pub(crate) fn text_scan_input(path: &str, contents: &str) -> ScanInput {
    ScanInput::Text {
        path: repo_path(path),
        contents: contents.to_string(),
    }
}

pub(crate) fn binary_scan_input(path: &str, bytes: Vec<u8>) -> ScanInput {
    ScanInput::Bytes {
        path: repo_path(path),
        bytes,
    }
}

pub(crate) fn zero_finding_report() -> ScanReport {
    ScanReport {
        root: repo_root("."),
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        selected_input_count: 0,
        scanned_text_input_count: 0,
        findings: Vec::new(),
        report_destination: None,
    }
}

pub(crate) fn zero_finding_report_with_destination(path: &str) -> ScanReport {
    ScanReport {
        root: repo_root("."),
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        selected_input_count: 0,
        scanned_text_input_count: 0,
        findings: Vec::new(),
        report_destination: Some(PathBuf::from(path)),
    }
}

pub(crate) fn report_with_findings(findings: Vec<NamingFinding>) -> ScanReport {
    ScanReport {
        root: repo_root("."),
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        selected_input_count: findings.len(),
        scanned_text_input_count: findings.len(),
        findings,
        report_destination: None,
    }
}

pub(crate) fn repo_root(path: &str) -> RepoRoot {
    RepoRoot::new(PathBuf::from(path))
}

pub(crate) fn repo_path(path: &str) -> RepoPath {
    RepoPath::new(path)
}

pub(crate) fn line(value: u64) -> LineNumber {
    LineNumber::new(value)
}

pub(crate) fn column(value: u64) -> ColumnNumber {
    ColumnNumber::new(value)
}

pub(crate) fn assert_legacy_crate_module_repository_result(
    result: Result<ScanReport, NamingScanError>,
) {
    assert_eq!(
        result,
        Err(NamingScanError::InvalidCanonicalSpelling {
            findings: vec![finding_with_class(
                "crates/legacy/src/lib.rs",
                1,
                9,
                SpellingClass::LegacyCrateModuleSpelling,
                CANONICAL_UNDERSCORE,
            )],
        })
    );
}

pub(crate) fn assert_legacy_language_version_repository_result(
    result: Result<ScanReport, NamingScanError>,
) {
    assert_eq!(
        result,
        Err(NamingScanError::InvalidCanonicalSpelling {
            findings: vec![finding_with_class(
                "fixtures/workflow.yaml",
                1,
                11,
                SpellingClass::LegacyLanguageVersionSpelling,
                CANONICAL_LANGUAGE_VERSION,
            )],
        })
    );
}
