use super::config_types::*;
use super::config_validate::*;
use super::types::*;
use std::path::PathBuf;

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
    match result {
        Err(NamingScanError::InvalidConfiguration { reason }) => {
            assert_eq!(reason, "empty scan configuration");
        }
        _ => panic!("expected InvalidConfiguration error variant"),
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
    let config = result.expect("repository_path allowlist must be accepted");
    assert!(
        matches!(config.allowlist_policy, AllowlistPolicy::Exact(rules) if !rules.is_empty()),
        "allowlist_policy must contain the allowed rule"
    );
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
    let config = result.expect("migration_reference allowlist must be accepted");
    assert!(
        matches!(config.allowlist_policy, AllowlistPolicy::Exact(rules) if !rules.is_empty()),
        "allowlist_policy must contain the allowed rule"
    );
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
    let config = result.expect("closed character class must be accepted");
    assert_eq!(config.scan_patterns, vec!["[abc]".to_string()]);
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
