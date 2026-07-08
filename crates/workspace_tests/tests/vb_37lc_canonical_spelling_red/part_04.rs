use super::*;

#[test]
fn scan_file_does_not_report_canonical_crate_module_even_when_pattern_selects_it() {
    let mut config = maximum_bounded_scan_config();
    config.scan_patterns = vec![CANONICAL_UNDERSCORE.to_string()];
    let input = text_scan_input("crates/vb_cli/src/lib.rs", "pub mod vb_cli;\n");

    let result = scan_file(input, &config);

    assert_eq!(result, Ok(Vec::new()));
}

#[test]
fn scan_file_returns_legacy_crate_module_finding_when_legacy_crate_module_token_is_seen() {
    let config = maximum_bounded_scan_config();
    let input = text_scan_input("crates/legacy/src/lib.rs", "pub mod velvet_ballistics;\n");

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding_with_class(
            "crates/legacy/src/lib.rs",
            1,
            9,
            SpellingClass::LegacyCrateModuleSpelling,
            CANONICAL_UNDERSCORE,
        )])
    );
}

#[test]
fn scan_file_decodes_valid_utf8_bytes_and_reports_exact_finding() {
    let config = minimum_valid_scan_config();
    let input = binary_scan_input(
        "docs/bytes.md",
        "legacy Velvet-Ballastics\n".as_bytes().to_vec(),
    );

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/bytes.md", 1, 8, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_returns_input_read_failed_when_file_input_is_missing() {
    let config = minimum_valid_scan_config();
    let input = ScanInput::File {
        path: repo_path("docs/missing.md"),
        absolute_path: PathBuf::from("target/vb-37lc-definitely-missing.md"),
    };

    let result = scan_file(input, &config);

    assert_missing_file_input_error(result);
}

#[test]
fn scan_file_returns_legacy_language_version_finding_when_legacy_language_version_token_is_seen() {
    let mut config = maximum_bounded_scan_config();
    config.scan_patterns = vec![LEGACY_LANGUAGE_VERSION.to_string()];
    let input = text_scan_input(
        "fixtures/workflow.yaml",
        &format!("language: {LEGACY_LANGUAGE_VERSION}\n"),
    );

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding_with_class(
            "fixtures/workflow.yaml",
            1,
            11,
            SpellingClass::LegacyLanguageVersionSpelling,
            CANONICAL_LANGUAGE_VERSION,
        )])
    );
}

#[test]
fn scan_repository_returns_invalid_canonical_spelling_when_legacy_crate_module_is_in_real_file()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture_file(
        temp.path(),
        "crates/legacy/src/lib.rs",
        "pub mod velvet_ballistics;\n",
    )?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = maximum_bounded_scan_config();

    let result = scan_repository(root, config);

    assert_legacy_crate_module_repository_result(result);
    Ok(())
}

#[test]
fn scan_repository_returns_invalid_canonical_spelling_when_legacy_language_version_is_in_real_file()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture_file(
        temp.path(),
        "fixtures/workflow.yaml",
        &format!("language: {LEGACY_LANGUAGE_VERSION}\n"),
    )?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let mut config = maximum_bounded_scan_config();
    config.scan_patterns = vec![LEGACY_LANGUAGE_VERSION.to_string()];

    let result = scan_repository(root, config);

    assert_legacy_language_version_repository_result(result);
    Ok(())
}

#[test]
fn scan_file_location_kernel_returns_column_one_when_token_starts_at_first_column() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input("docs/naming.md", "Velvet-Ballastics must be fixed\n");

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/naming.md", 1, 1, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_location_kernel_preserves_crlf_columns_when_invalid_token_is_seen() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input("docs/naming.md", "ok\r\nfix: Velvet-Ballastics\r\n");

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/naming.md", 2, 6, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_location_kernel_preserves_final_line_without_newline_when_invalid_token_is_seen() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input("docs/naming.md", "first line\nfinal Velvet-Ballastics");

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![finding("docs/naming.md", 2, 7, CANONICAL_HYPHEN)])
    );
}

#[test]
fn scan_file_finding_kernel_returns_all_occurrences_when_many_tokens_share_one_line() {
    let config = minimum_valid_scan_config();
    let input = text_scan_input(
        "docs/naming.md",
        "Velvet-Ballastics then Velvet-Ballastics\n",
    );

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Ok(vec![
            finding("docs/naming.md", 1, 1, CANONICAL_HYPHEN),
            finding("docs/naming.md", 1, 24, CANONICAL_HYPHEN),
        ])
    );
}

#[test]
fn scan_repository_report_kernel_returns_zero_findings_with_exact_input_count_when_inputs_are_empty()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();
    let expected = ScanReport {
        root: root.clone(),
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        selected_input_count: 0,
        scanned_text_input_count: 0,
        findings: Vec::new(),
        report_destination: None,
    };

    let result = scan_repository(root, config);

    assert_eq_scan_repository_result(result, Ok(expected));
    Ok(())
}

#[test]
fn scan_repository_report_kernel_preserves_config_identity_when_report_is_successful()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture_file(
        temp.path(),
        "docs/naming.md",
        "velvet-ballistics is absent\n",
    )?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();
    let expected = ScanReport {
        root: root.clone(),
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        selected_input_count: 1,
        scanned_text_input_count: 1,
        findings: Vec::new(),
        report_destination: None,
    };

    let result = scan_repository(root, config);

    assert_eq_scan_repository_result(result, Ok(expected));
    Ok(())
}
