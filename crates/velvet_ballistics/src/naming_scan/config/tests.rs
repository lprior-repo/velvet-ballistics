use super::*;

#[test]
fn canonical_spelling_table_uses_correct_names() {
    let table = canonical_spelling_table();
    assert_eq!(table.product, CANONICAL_HYPHEN);
    assert_eq!(table.binary, CANONICAL_HYPHEN);
    assert_eq!(table.package, CANONICAL_HYPHEN);
    assert_eq!(table.bead_rig, CANONICAL_HYPHEN);
    assert_eq!(table.crate_module, CANONICAL_UNDERSCORE);
    assert_eq!(table.bead_database, CANONICAL_UNDERSCORE);
    assert_eq!(table.language_version, CANONICAL_LANGUAGE_VERSION);
}

fn valid_entries() -> Vec<CanonicalEntry> {
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

fn valid_config() -> RawScanConfig {
    RawScanConfig {
        canonical_entries: valid_entries(),
        legacy_allowlist: vec![],
        scan_patterns: vec!["test".into()],
        excluded_path_rules: vec![],
        workspace_root: PathBuf::from("/tmp"),
        report_destination: None,
    }
}

#[test]
fn validate_scan_config_rejects_empty_entries() {
    let raw = RawScanConfig::empty();
    let result = validate_scan_config(raw);
    assert!(result.is_err());
    match result.unwrap_err() {
        NamingScanError::InvalidConfiguration { reason } => {
            assert_eq!(reason, "empty scan configuration");
        }
        _ => panic!("wrong error variant"),
    }
}

#[test]
fn validate_scan_config_rejects_wildcard_allowlist() {
    let mut raw = valid_config();
    raw.legacy_allowlist = vec![LegacyAllowRule::Wildcard {
        pattern: "*".into(),
    }];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn validate_scan_config_rejects_prefix_only_allowlist() {
    let mut raw = valid_config();
    raw.legacy_allowlist = vec![LegacyAllowRule::PrefixOnly {
        prefix: "old".into(),
    }];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn validate_scan_config_rejects_substring_allowlist() {
    let mut raw = valid_config();
    raw.legacy_allowlist = vec![LegacyAllowRule::Substring {
        needle: "bad".into(),
    }];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn validate_scan_config_accepts_repository_path_allowlist() {
    let mut raw = valid_config();
    raw.legacy_allowlist = vec![LegacyAllowRule::RepositoryPath {
        path: "src/main.rs".into(),
    }];
    let result = validate_scan_config(raw);
    assert!(result.is_ok());
}

#[test]
fn validate_scan_config_accepts_migration_reference_allowlist() {
    let mut raw = valid_config();
    raw.legacy_allowlist = vec![LegacyAllowRule::MigrationReference {
        label: "mig".into(),
        artifact: "file".into(),
        legacy_text: "old".into(),
    }];
    let result = validate_scan_config(raw);
    assert!(result.is_ok());
}

#[test]
fn validate_scan_config_rejects_unclosed_character_class() {
    let mut raw = valid_config();
    raw.scan_patterns = vec!["[abc".into()];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::PatternCompilationFailed { .. }
    ));
}

#[test]
fn validate_scan_config_accepts_closed_character_class() {
    let mut raw = valid_config();
    raw.scan_patterns = vec!["[abc]".into()];
    let result = validate_scan_config(raw);
    assert!(result.is_ok());
}

#[test]
fn validate_scan_config_rejects_contradictory_token() {
    let mut raw = valid_config();
    raw.canonical_entries[0] = CanonicalEntry::new(CanonicalNameKind::Product, "wrong-name");
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn validate_scan_config_rejects_duplicate_kind() {
    let mut raw = valid_config();
    raw.canonical_entries = vec![
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
    ];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn validate_scan_config_rejects_missing_kind() {
    let mut raw = valid_config();
    raw.canonical_entries = vec![CanonicalEntry::new(
        CanonicalNameKind::Product,
        CANONICAL_HYPHEN,
    )];
    let result = validate_scan_config(raw);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn fingerprint_for_destination_yields_minimum_when_none() {
    assert_eq!(fingerprint_for_destination(None), "vb-37lc-minimum-config");
}

#[test]
fn fingerprint_for_destination_yields_maximum_when_some() {
    let dst = PathBuf::from("/tmp/report.txt");
    assert_eq!(
        fingerprint_for_destination(Some(&dst)),
        "vb-37lc-maximum-bounded-config"
    );
}
