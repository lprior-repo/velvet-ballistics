use super::*;

#[test]
fn validate_scan_config_names_bead_database_when_token_contradicts_kind() {
    let result = validate_scan_config(raw_config_with_contradictory_token(
        CanonicalNameKind::BeadDatabase,
        "not_the_database",
    ));

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "contradictory token for bead_database: not_the_database".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_wildcard() {
    let raw = raw_config_with_allow_rule(LegacyAllowRule::Wildcard {
        pattern: "*".to_string(),
    });

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "broad wildcard allowlist rule: *".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_prefix_only_rule() {
    let raw = raw_config_with_allow_rule(LegacyAllowRule::PrefixOnly {
        prefix: "https://github.com/priorlewis43/".to_string(),
    });

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "prefix-only allowlist rule: https://github.com/priorlewis43/".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_substring_rule() {
    let raw = raw_config_with_allow_rule(LegacyAllowRule::Substring {
        needle: LEGACY_PROJECT.to_string(),
    });

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::InvalidConfiguration {
            reason: "substring allowlist rule: velvet-ballistics".to_string()
        })
    );
}

#[test]
fn validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid() {
    let raw = raw_config_with_scan_pattern("[unterminated".to_string());

    let result = validate_scan_config(raw);

    assert_eq!(
        result,
        Err(NamingScanError::PatternCompilationFailed {
            pattern: "[unterminated".to_string(),
            source: "unclosed character class".to_string()
        })
    );
}

#[test]
fn classify_occurrence_returns_canonical_product_when_ballistics_product_token_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("README.md");

    let result = classify_occurrence(path, line(1), column(9), CANONICAL_HYPHEN, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::CanonicalProduct {
            canonical: CANONICAL_HYPHEN.to_string(),
            kind: CanonicalNameKind::Product,
        })
    );
}

#[test]
fn classify_occurrence_returns_canonical_product_for_binary_package_and_bead_rig_aliases() {
    let mut config = minimum_valid_scan_config();
    config.canonical_table.binary = "velvet-ballistics-bin".to_string();
    config.canonical_table.package = "velvet-ballistics-package".to_string();
    config.canonical_table.bead_rig = "velvet-ballistics-rig".to_string();

    assert_canonical_product_alias("Cargo.toml", 1, "velvet-ballistics-bin", &config);
    assert_canonical_product_alias("Cargo.toml", 2, "velvet-ballistics-package", &config);
    assert_canonical_product_alias(".beads/config.yaml", 3, "velvet-ballistics-rig", &config);
}

fn assert_canonical_product_alias(path: &str, line_number: u64, token: &str, config: &ScanConfig) {
    let result = classify_occurrence(repo_path(path), line(line_number), column(1), token, config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::CanonicalProduct {
            canonical: token.to_string(),
            kind: CanonicalNameKind::Product,
        })
    );
}

#[test]
fn classify_occurrence_returns_canonical_crate_module_when_underscore_ballistics_token_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("crates/vb_cli/src/lib.rs");

    let result = classify_occurrence(path, line(1), column(13), CANONICAL_UNDERSCORE, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::CanonicalCrateModule {
            canonical: CANONICAL_UNDERSCORE.to_string(),
            kind: CanonicalNameKind::CrateModule,
        })
    );
}

#[test]
fn classify_occurrence_returns_canonical_crate_module_when_database_alias_is_seen() {
    let mut config = minimum_valid_scan_config();
    config.canonical_table.bead_database = "velvet_ballistics_db".to_string();
    let path = repo_path(".beads/config.yaml");

    let result = classify_occurrence(path, line(1), column(1), "velvet_ballistics_db", &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::CanonicalCrateModule {
            canonical: "velvet_ballistics_db".to_string(),
            kind: CanonicalNameKind::CrateModule,
        })
    );
}

#[test]
fn classify_occurrence_returns_canonical_language_version_when_v1_token_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("fixtures/workflow.yaml");

    let result = classify_occurrence(
        path,
        line(1),
        column(10),
        CANONICAL_LANGUAGE_VERSION,
        &config,
    );

    assert_eq!(
        result,
        Ok(OccurrenceClass::CanonicalLanguageVersion {
            canonical: CANONICAL_LANGUAGE_VERSION.to_string(),
            kind: CanonicalNameKind::LanguageVersion,
        })
    );
}

#[test]
fn classify_occurrence_returns_allowed_repository_path_with_exact_payload_when_current_external_repository_path_is_seen()
 {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/migration.md");

    let result = classify_occurrence(path, line(4), column(1), CURRENT_REPOSITORY_PATH, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::AllowedLegacy {
            exception: LegacyException::RepositoryPath {
                path: CURRENT_REPOSITORY_PATH.to_string(),
            },
        })
    );
}

#[test]
fn classify_occurrence_returns_allowed_master_filename_with_exact_payload_when_current_master_filename_is_seen()
 {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/migration.md");

    let result = classify_occurrence(path, line(5), column(3), CURRENT_MASTER_FILENAME, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::AllowedLegacy {
            exception: LegacyException::MasterFilename {
                filename: CURRENT_MASTER_FILENAME.to_string(),
            },
        })
    );
}
