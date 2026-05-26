use super::*;

pub(crate) fn assert_eq_scan_repository_result(
    result: Result<ScanReport, NamingScanError>,
    expected: Result<ScanReport, NamingScanError>,
) {
    assert_eq!(result, expected);
}

pub(crate) fn assert_eq_discover_scan_inputs_result(
    result: Result<Vec<ScanInput>, NamingScanError>,
    expected: Result<Vec<ScanInput>, NamingScanError>,
) {
    assert_eq!(result, expected);
}

pub(crate) fn assert_eq_render_scan_report_result(
    result: Result<RenderedReport, NamingScanError>,
    expected: Result<RenderedReport, NamingScanError>,
) {
    assert_eq!(result, expected);
}

pub(crate) fn assert_eq_rendered_report_body(written: String, expected: &str) {
    assert_eq!(written, expected);
}

pub(crate) fn assert_missing_file_input_error(result: Result<Vec<NamingFinding>, NamingScanError>) {
    match result {
        Err(NamingScanError::InputReadFailed { path, source }) => {
            assert_eq!(path, repo_path("docs/missing.md"));
            assert_ne!(source, "");
        }
        other => assert_eq!(
            other,
            Err(NamingScanError::InputReadFailed {
                path: repo_path("docs/missing.md"),
                source: "non-empty platform error".to_string(),
            })
        ),
    }
}

pub(crate) fn assert_directory_destination_write_failed(
    result: Result<RenderedReport, NamingScanError>,
    expected_path: PathBuf,
) {
    match result {
        Err(NamingScanError::ReportWriteFailed { path, source }) => {
            assert_eq!(path, expected_path);
            assert_ne!(source, "");
        }
        other => assert_eq!(
            other,
            Err(NamingScanError::ReportWriteFailed {
                path: expected_path,
                source: "non-empty platform error".to_string(),
            })
        ),
    }
}

pub(crate) fn write_fixture_file(
    root: &std::path::Path,
    relative: &str,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn remove_fixture_tree_if_present(
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Box::new(error)),
    }
}

pub(crate) fn remove_file_if_present(
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Box::new(error)),
    }
}

pub(crate) fn minimum_valid_raw_config() -> RawScanConfig {
    RawScanConfig {
        canonical_entries: canonical_entries(),
        legacy_allowlist: exact_legacy_allowlist(),
        scan_patterns: vec![LEGACY_PROJECT.to_string()],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        workspace_root: PathBuf::from("."),
        report_destination: None,
    }
}

pub(crate) fn maximum_bounded_valid_raw_config() -> RawScanConfig {
    RawScanConfig {
        canonical_entries: canonical_entries(),
        legacy_allowlist: exact_legacy_allowlist(),
        scan_patterns: vec![
            LEGACY_PROJECT.to_string(),
            "velvet_ballastics".to_string(),
            "velvet-ballastics/v1".to_string(),
        ],
        excluded_path_rules: vec![
            ".git/**".to_string(),
            "target/**".to_string(),
            ".beads/dolt/**".to_string(),
            ".beads/backup/**".to_string(),
            ".beads/embeddeddolt/**".to_string(),
        ],
        workspace_root: PathBuf::from("."),
        report_destination: Some(PathBuf::from("target/naming-scan.txt")),
    }
}

pub(crate) fn minimum_valid_scan_config() -> ScanConfig {
    ScanConfig {
        canonical_table: CanonicalSpellingTable {
            product: CANONICAL_HYPHEN.to_string(),
            binary: CANONICAL_HYPHEN.to_string(),
            package: CANONICAL_HYPHEN.to_string(),
            bead_rig: CANONICAL_HYPHEN.to_string(),
            crate_module: CANONICAL_UNDERSCORE.to_string(),
            bead_database: CANONICAL_UNDERSCORE.to_string(),
            language_version: CANONICAL_LANGUAGE_VERSION.to_string(),
        },
        allowlist_policy: AllowlistPolicy::Exact(exact_legacy_allowlist()),
        scan_patterns: vec![LEGACY_PROJECT.to_string()],
        excluded_path_rules: vec![".git/**".to_string(), "target/**".to_string()],
        config_fingerprint: "vb-37lc-minimum-config".to_string(),
        report_destination: None,
    }
}

pub(crate) fn maximum_bounded_scan_config() -> ScanConfig {
    let mut config = minimum_valid_scan_config();
    config.scan_patterns = maximum_scan_patterns();
    config.excluded_path_rules = maximum_excluded_path_rules();
    config.config_fingerprint = "vb-37lc-maximum-bounded-config".to_string();
    config.report_destination = Some(PathBuf::from("target/naming-scan.txt"));
    config
}

pub(crate) fn maximum_scan_patterns() -> Vec<String> {
    vec![
        LEGACY_PROJECT.to_string(),
        "velvet_ballastics".to_string(),
        "velvet-ballastics/v1".to_string(),
    ]
}

pub(crate) fn maximum_excluded_path_rules() -> Vec<String> {
    vec![
        ".git/**".to_string(),
        "target/**".to_string(),
        ".beads/dolt/**".to_string(),
        ".beads/backup/**".to_string(),
        ".beads/embeddeddolt/**".to_string(),
    ]
}
