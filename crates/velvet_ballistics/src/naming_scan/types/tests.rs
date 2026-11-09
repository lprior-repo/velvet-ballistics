use super::*;
use std::path::PathBuf;

#[test]
fn canonical_entry_new_sets_fields() {
    let entry = CanonicalEntry::new(CanonicalNameKind::Product, "velvet-ballistics");
    assert_eq!(entry.kind, CanonicalNameKind::Product);
    assert_eq!(entry.token, "velvet-ballistics");
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
