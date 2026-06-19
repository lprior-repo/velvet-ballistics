use super::config_build::*;
use super::config_types::*;
use super::types::*;

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

#[test]
fn table_from_entries_accepts_all_required_kinds() {
    let result = table_from_entries(&valid_entries());
    assert!(
        result.is_ok(),
        "table_from_entries should accept all 7 required kinds"
    );
}

#[test]
fn table_from_entries_rejects_duplicate_kind() {
    let entries = vec![
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
    ];
    let result = table_from_entries(&entries);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn table_from_entries_rejects_missing_bead_database() {
    let entries = vec![
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Binary, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::Package, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::BeadRig, CANONICAL_HYPHEN),
        CanonicalEntry::new(CanonicalNameKind::CrateModule, CANONICAL_UNDERSCORE),
        CanonicalEntry::new(
            CanonicalNameKind::LanguageVersion,
            CANONICAL_LANGUAGE_VERSION,
        ),
    ];
    let result = table_from_entries(&entries);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}

#[test]
fn table_from_entries_rejects_contradictory_token() {
    let mut entries = valid_entries();
    entries[0] = CanonicalEntry::new(CanonicalNameKind::Product, "wrong-name");
    let result = table_from_entries(&entries);
    assert!(matches!(
        result.unwrap_err(),
        NamingScanError::InvalidConfiguration { .. }
    ));
}
