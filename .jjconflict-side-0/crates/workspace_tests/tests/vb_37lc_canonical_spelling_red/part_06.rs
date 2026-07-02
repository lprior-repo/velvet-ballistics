use super::*;

#[test]
fn scan_file_returns_input_read_failed_when_selected_input_is_undecodable() {
    let config = minimum_valid_scan_config();
    let input = binary_scan_input("fixtures/invalid-utf8.bin", vec![0xff, 0xfe, 0xfd]);

    let result = scan_file(input, &config);

    assert_eq!(
        result,
        Err(NamingScanError::InputReadFailed {
            path: repo_path("fixtures/invalid-utf8.bin"),
            source: "input is not supported UTF-8 text".to_string(),
        })
    );
}

#[test]
fn scan_repository_returns_invalid_canonical_spelling_when_legacy_spelling_is_outside_allowlist()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture_file(
        temp.path(),
        "docs/naming.md",
        "ok\nok\nlegacy Velvet-Ballastics\n",
    )?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();

    let result = scan_repository(root, config);

    assert_eq_scan_repository_result(
        result,
        Err(NamingScanError::InvalidCanonicalSpelling {
            findings: vec![finding("docs/naming.md", 3, 8, CANONICAL_HYPHEN)],
        }),
    );
    Ok(())
}

#[test]
fn render_scan_report_returns_report_write_failed_when_destination_parent_is_missing() {
    let report = zero_finding_report_with_destination("missing-parent/report.txt");

    let result = render_scan_report(&report);

    assert_eq!(
        result,
        Err(NamingScanError::ReportWriteFailed {
            path: PathBuf::from("missing-parent/report.txt"),
            source: "parent directory does not exist".to_string(),
        })
    );
}

#[test]
fn render_scan_report_returns_report_write_failed_when_destination_is_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let mut report = zero_finding_report();
    report.report_destination = Some(temp.path().to_path_buf());

    let result = render_scan_report(&report);

    assert_directory_destination_write_failed(result, temp.path().to_path_buf());
    Ok(())
}

#[test]
fn render_scan_report_writes_report_when_destination_has_empty_parent_component()
-> Result<(), Box<dyn std::error::Error>> {
    let destination = PathBuf::from("vb-37lc-parentless-naming-scan.txt");
    remove_file_if_present(&destination)?;
    let mut report = zero_finding_report();
    report.report_destination = Some(destination.clone());

    let result = render_scan_report(&report);
    let written = std::fs::read_to_string(&destination)?;
    remove_file_if_present(&destination)?;

    assert_eq_render_scan_report_result(
        result,
        Ok(RenderedReport {
            body: "canonical spelling scan: 0 findings; selected=0; scanned=0\n".to_string(),
        }),
    );
    assert_eq_rendered_report_body(
        written,
        "canonical spelling scan: 0 findings; selected=0; scanned=0\n",
    );
    Ok(())
}
