use super::*;

#[test]
fn classify_occurrence_returns_allowed_migration_reference_with_exact_payload_when_explicit_migration_reference_is_seen()
 {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/migration.md");
    let text = "MIGRATION-REFERENCE external-preexisting-artifact Velvet-Ballastics";

    let result = classify_occurrence(path, line(6), column(1), text, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::AllowedLegacy {
            exception: LegacyException::MigrationReference {
                artifact: MIGRATION_ARTIFACT.to_string(),
                label: MIGRATION_LABEL.to_string(),
                legacy_text: LEGACY_PROJECT.to_string(),
            },
        })
    );
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_repository_path_is_only_a_substring() {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/bad.md");
    let text = "prefix-https://github.com/priorlewis43/Velvet-Ballastics-suffix";

    let result = classify_occurrence(path, line(1), column(1), text, &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_master_filename_is_embedded_in_unrelated_path() {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/bad.md");
    let text = "archive/velvet-ballistics-MASTER.md.copy";

    let result = classify_occurrence(path, line(2), column(9), text, &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_migration_label_is_absent() {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/bad.md");
    let text = "external-preexisting-artifact Velvet-Ballastics";

    let result = classify_occurrence(path, line(3), column(31), text, &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_no_occurrence_when_text_is_empty() {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/empty.md");

    let result = classify_occurrence(path, line(1), column(1), "", &config);

    assert_eq!(result, Ok(OccurrenceClass::NoOccurrence));
}

#[test]
fn classify_occurrence_returns_no_occurrence_when_text_has_no_legacy_token() {
    let config = minimum_valid_scan_config();
    let path = repo_path("docs/clean.md");

    let result = classify_occurrence(path, line(1), column(1), "ballastics only", &config);

    assert_eq!(result, Ok(OccurrenceClass::NoOccurrence));
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_case_variant_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("README.md");

    let result = classify_occurrence(path, line(1), column(1), "Velvet-Ballastics", &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_uppercase_variant_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("README.md");

    let result = classify_occurrence(path, line(1), column(1), "VELVET-BALLASTICS", &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_invalid_legacy_when_unicode_confusable_is_seen() {
    let config = minimum_valid_scan_config();
    let path = repo_path("README.md");

    let result = classify_occurrence(path, line(1), column(1), "velvet‐ballistics", &config);

    assert_eq!(result, Ok(invalid_legacy_occurrence(CANONICAL_HYPHEN)));
}

#[test]
fn classify_occurrence_returns_legacy_crate_module_class_when_legacy_crate_module_token_is_seen() {
    let config = maximum_bounded_scan_config();
    let path = repo_path("crates/legacy/src/lib.rs");

    let result = classify_occurrence(path, line(1), column(9), "velvet_ballistics", &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyCrateModuleSpelling,
            remediation: CANONICAL_UNDERSCORE.to_string(),
        })
    );
}

#[test]
fn classify_occurrence_returns_legacy_language_version_class_when_legacy_language_version_token_is_seen()
 {
    let config = maximum_bounded_scan_config();
    let path = repo_path("fixtures/workflow.yaml");

    let result = classify_occurrence(path, line(1), column(11), "velvet-ballistics/v1", &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyLanguageVersionSpelling,
            remediation: CANONICAL_LANGUAGE_VERSION.to_string(),
        })
    );
}

#[test]
fn classify_occurrence_returns_legacy_language_version_class_when_token_is_embedded() {
    let config = maximum_bounded_scan_config();
    let path = repo_path("fixtures/workflow.yaml");
    let text = "language=velvet-ballistics/v1";

    let result = classify_occurrence(path, line(1), column(10), text, &config);

    assert_eq!(
        result,
        Ok(OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyLanguageVersionSpelling,
            remediation: CANONICAL_LANGUAGE_VERSION.to_string(),
        })
    );
}

#[test]
fn scan_file_allows_documented_external_legacy_repository_url_inside_sentence() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input(
        "docs/migration.md",
        "legacy source: https://github.com/priorlewis43/Velvet-Ballastics.\n",
    );

    let result = scan_file(input, &config);

    assert_eq!(result, Ok(Vec::new()));
}

#[test]
fn scan_file_allows_documented_external_legacy_repository_url_when_exact_boundary_text_is_seen() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input(
        "docs/migration.md",
        "https://github.com/priorlewis43/Velvet-Ballastics\n",
    );

    let result = scan_file(input, &config);

    assert_eq!(result, Ok(Vec::new()));
}

#[test]
fn scan_file_rejects_documented_external_legacy_repository_url_when_left_boundary_is_missing() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input(
        "docs/migration.md",
        "xhttps://github.com/priorlewis43/Velvet-Ballastics\n",
    );

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/migration.md", 1, 34, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_rejects_documented_external_legacy_repository_url_when_right_boundary_is_missing() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input(
        "docs/migration.md",
        "https://github.com/priorlewis43/Velvet-Ballastics-extra\n",
    );

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/migration.md", 1, 33, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_ignores_non_exact_allowlist_rule_when_config_was_not_validated() {
    let mut config = maximum_bounded_scan_config();
    config.allowlist_policy = AllowlistPolicy::Exact(vec![LegacyAllowRule::Wildcard {
        pattern: "*".to_string(),
    }]);
    let input = text_scan_input("docs/bad.md", "legacy velvet_ballistics\n");

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding_with_class(
            "docs/bad.md",
            1,
            8,
            SpellingClass::LegacyCrateModuleSpelling,
            CANONICAL_UNDERSCORE,
        )])
    );
}
