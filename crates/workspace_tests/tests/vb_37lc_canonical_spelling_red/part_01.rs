use super::*;

#[test]
fn canonical_table_returns_ballistics_for_product_when_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.product, CANONICAL_HYPHEN);
}

#[test]
fn canonical_table_returns_ballistics_for_binary_when_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.binary, CANONICAL_HYPHEN);
}

#[test]
fn canonical_table_returns_ballistics_for_package_when_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.package, CANONICAL_HYPHEN);
}

#[test]
fn canonical_table_returns_ballistics_for_bead_rig_when_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.bead_rig, CANONICAL_HYPHEN);
}

#[test]
fn canonical_table_returns_underscore_ballistics_for_crate_module_and_database_when_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.crate_module, CANONICAL_UNDERSCORE);
    assert_eq!(table.bead_database, CANONICAL_UNDERSCORE);
}

#[test]
fn canonical_table_returns_ballistics_v1_when_language_version_is_loaded() {
    let table = canonical_spelling_table();

    assert_eq!(table.language_version, CANONICAL_LANGUAGE_VERSION);
}

#[test]
fn validate_scan_config_returns_scan_config_when_minimum_valid_config_is_supplied() {
    let raw = minimum_valid_raw_config();
    let expected = minimum_valid_scan_config();

    let result = validate_scan_config(raw);

    assert_eq!(result, Ok(expected));
}

#[test]
fn canonical_entry_replaces_token_only_when_kind_matches() {
    let entry = CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN);

    let replaced = entry
        .clone()
        .replace_token_when_kind_matches(CanonicalNameKind::Product, "replacement-token");
    let preserved = entry
        .replace_token_when_kind_matches(CanonicalNameKind::LanguageVersion, "replacement-token");

    assert_eq!(
        replaced,
        CanonicalEntry::new(CanonicalNameKind::Product, "replacement-token")
    );
    assert_eq!(
        preserved,
        CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN)
    );
}

#[test]
fn validate_scan_config_returns_scan_config_when_maximum_bounded_valid_config_is_supplied() {
    let raw = maximum_bounded_valid_raw_config();
    let expected = maximum_bounded_scan_config();

    let result = validate_scan_config(raw);

    assert_eq!(result, Ok(expected));
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_config_is_empty() {
    let raw = RawScanConfig::empty();

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "empty scan configuration".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_product_kind_is_missing() {
    let raw = raw_config_missing_kind(CanonicalNameKind::Product);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "missing canonical kind: product".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_crate_module_kind_is_missing() {
    let raw = raw_config_missing_kind(CanonicalNameKind::CrateModule);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "missing canonical kind: crate_module".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_language_version_kind_is_missing() {
    let raw = raw_config_missing_kind(CanonicalNameKind::LanguageVersion);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "missing canonical kind: language_version".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_below_required() {
    let raw = raw_config_missing_kind(CanonicalNameKind::BeadDatabase);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "canonical kind count one below required: bead_database missing".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_kind_is_duplicated() {
    let raw = raw_config_with_duplicate_kind(CanonicalNameKind::Product, CANONICAL_HYPHEN);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "duplicate canonical kind: product".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_above_required() {
    let raw = raw_config_with_duplicate_kind(
        CanonicalNameKind::LanguageVersion,
        CANONICAL_LANGUAGE_VERSION,
    );

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "canonical kind count one above required: duplicate language_version"
                .to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_canonical_token_contradicts_kind() {
    let raw = raw_config_with_contradictory_token(CanonicalNameKind::CrateModule, LEGACY_PROJECT);

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "contradictory token for crate_module: velvet-ballastics".to_string()
        })
    );
}

#[test]
fn validate_scan_config_names_binary_package_and_bead_rig_when_tokens_contradict_kind() {
    assert_contradictory_token_reason(
        CanonicalNameKind::Binary,
        "not-the-binary",
        "contradictory token for binary: not-the-binary",
    );
    assert_contradictory_token_reason(
        CanonicalNameKind::Package,
        "not-the-package",
        "contradictory token for package: not-the-package",
    );
    assert_contradictory_token_reason(
        CanonicalNameKind::BeadRig,
        "not-the-rig",
        "contradictory token for bead_rig: not-the-rig",
    );
}

fn assert_contradictory_token_reason(kind: CanonicalNameKind, token: &str, reason: &str) {
    let result = validate_scan_config(raw_config_with_contradictory_token(kind, token));

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: reason.to_string()
        })
    );
}
