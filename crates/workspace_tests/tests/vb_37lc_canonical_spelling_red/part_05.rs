use super::*;

#[test]
fn scan_repository_report_kernel_returns_all_findings_when_valid_and_invalid_inputs_are_mixed()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture_file(temp.path(), "docs/good.md", "velvet-ballistics is absent\n")?;
    write_fixture_file(
        temp.path(),
        "docs/bad.md",
        "ok\nok\nlegacy velvet-ballistics\n",
    )?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();
    let expected_findings = vec![finding("docs/bad.md", 3, 8, CANONICAL_HYPHEN)];

    let result = scan_repository(root, config);

    assert_eq_scan_repository_result(
        result,
        Err(NamingScanError::InvalidCanonicalSpelling {
            findings: expected_findings,
        }),
    );
    Ok(())
}

#[test]
fn render_scan_report_returns_empty_body_when_report_has_zero_findings() {
    let report = zero_finding_report();

    let result = render_scan_report(&report);

    assert_eq!(
        result,
        Ok(RenderedReport {
            body: "canonical spelling scan: 0 findings; selected=0; scanned=0\n".to_string(),
        })
    );
}

#[test]
fn render_scan_report_preserves_single_finding_fields_when_report_has_one_finding() {
    let report = report_with_findings(vec![finding("docs/naming.md", 3, 7, CANONICAL_HYPHEN)]);

    let result = render_scan_report(&report);

    assert_eq!(
        result,
        Ok(RenderedReport {
            body: "docs/naming.md:3:7 LegacyProjectSpelling -> velvet-ballistics\n".to_string(),
        })
    );
}

#[test]
fn render_scan_report_orders_duplicate_sort_keys_deterministically_when_findings_have_equal_path_line_column()
 {
    let report = report_with_findings(equal_sort_key_findings());

    let result = render_scan_report(&report);

    assert_eq!(
        result,
        Ok(RenderedReport {
            body: format!(
                "docs/naming.md:3:7 LegacyProjectSpelling -> velvet-ballistics\ndocs/naming.md:3:7 LegacyCrateModuleSpelling -> {CANONICAL_UNDERSCORE}\n"
            ),
        })
    );
}

fn equal_sort_key_findings() -> Vec<NamingFinding> {
    vec![
        finding_with_class(
            "docs/naming.md",
            3,
            7,
            SpellingClass::LegacyCrateModuleSpelling,
            CANONICAL_UNDERSCORE,
        ),
        finding_with_class(
            "docs/naming.md",
            3,
            7,
            SpellingClass::LegacyProjectSpelling,
            CANONICAL_HYPHEN,
        ),
    ]
}

#[test]
fn scan_repository_returns_invalid_root_when_root_is_missing() {
    let root = repo_root("/definitely/missing/vb-37lc-root");
    let config = minimum_valid_scan_config();

    let result = scan_repository(root.clone(), config);

    assert_eq!(result, Err(NamingScanError::InvalidRoot { root }));
}

#[test]
fn discover_scan_inputs_excludes_builtin_and_configured_paths_but_keeps_eligible_files()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_excluded_fixture_files(temp.path())?;
    write_fixture_file(temp.path(), "docs/visible.md", "velvet-ballistics\n")?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let mut config = minimum_valid_scan_config();
    config.excluded_path_rules = vec![
        "excluded/exact.md".to_string(),
        "excluded/tree/**".to_string(),
    ];
    let expected = vec![ScanInput::File {
        path: repo_path("docs/visible.md"),
        absolute_path: temp.path().join("docs/visible.md"),
    }];

    let result = discover_scan_inputs(root, &config);

    assert_eq_discover_scan_inputs_result(result, Ok(expected));
    Ok(())
}

fn write_excluded_fixture_files(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    for path in excluded_fixture_paths() {
        write_fixture_file(root, path, "hidden\n")?;
    }
    write_fixture_file(root, "docs/notes.txt", "ineligible\n")?;
    Ok(())
}

fn excluded_fixture_paths() -> [&'static str; 7] {
    [
        ".git/config",
        "target/generated.rs",
        ".beads/dolt/state.md",
        ".beads/backup/state.md",
        ".beads/embeddeddolt/state.md",
        "excluded/exact.md",
        "excluded/tree/hidden.md",
    ]
}

#[test]
fn discover_scan_inputs_keeps_every_eligible_extension_and_rejects_ineligible_text()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_eligible_extension_fixtures(temp.path())?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();
    let expected = eligible_extension_inputs(temp.path());

    let result = discover_scan_inputs(root, &config);

    assert_eq_discover_scan_inputs_result(result, Ok(expected));
    Ok(())
}

fn write_eligible_extension_fixtures(
    root: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, contents) in eligible_extension_fixtures() {
        write_fixture_file(root, path, contents)?;
    }
    write_fixture_file(root, "notes/plain.txt", "ignored\n")?;
    Ok(())
}

fn eligible_extension_fixtures() -> [(&'static str, &'static str); 6] {
    [
        ("Cargo.toml", "[package]\nname = 'fixture'\n"),
        ("scripts/check.sh", "#!/bin/sh\n"),
        ("config/app.toml", "name = 'fixture'\n"),
        ("config/app.yml", "name: fixture\n"),
        ("config/app.yaml", "name: fixture\n"),
        ("src/lib.rs", "pub fn fixture() {}\n"),
    ]
}

fn eligible_extension_inputs(root: &std::path::Path) -> Vec<ScanInput> {
    [
        "Cargo.toml",
        "config/app.toml",
        "config/app.yaml",
        "config/app.yml",
        "scripts/check.sh",
        "src/lib.rs",
    ]
    .into_iter()
    .map(|path| ScanInput::File {
        path: repo_path(path),
        absolute_path: root.join(path),
    })
    .collect()
}

#[test]
fn discover_scan_inputs_returns_invalid_root_when_root_path_is_a_file()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root_file = temp.path().join("not-a-directory");
    std::fs::write(&root_file, "not a directory\n")?;
    let root = RepoRoot::new(root_file);
    let config = minimum_valid_scan_config();

    let result = scan_repository(root.clone(), config);

    assert_eq_scan_repository_result(result, Err(NamingScanError::InvalidRoot { root }));
    Ok(())
}

#[cfg(unix)]
#[test]
fn discover_scan_inputs_skips_unreadable_child_directory_without_failing_root_scan()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir()?;
    write_fixture_file(temp.path(), "docs/visible.md", "velvet-ballistics\n")?;
    let unreadable = temp.path().join("restricted");
    std::fs::create_dir_all(&unreadable)?;
    std::fs::write(unreadable.join("hidden.md"), "velvet-ballistics\n")?;
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))?;
    let root = RepoRoot::new(temp.path().to_path_buf());
    let config = minimum_valid_scan_config();
    let expected = vec![ScanInput::File {
        path: repo_path("docs/visible.md"),
        absolute_path: temp.path().join("docs/visible.md"),
    }];

    let result = discover_scan_inputs(root, &config);

    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o700))?;
    assert_eq_discover_scan_inputs_result(result, Ok(expected));
    Ok(())
}

#[test]
fn discover_scan_inputs_scans_real_fixture_tree_when_root_name_matches_prior_shortcut()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root_path = temp.path().join("unreadable-tree");
    std::fs::create_dir_all(&root_path)?;
    write_fixture_file(
        &root_path,
        "docs/naming.md",
        "velvet-ballistics is canonical\n",
    )?;
    let root = RepoRoot::new(root_path.clone());
    let config = minimum_valid_scan_config();
    let expected = vec![ScanInput::File {
        path: repo_path("docs/naming.md"),
        absolute_path: root_path.join("docs/naming.md"),
    }];

    let result = discover_scan_inputs(root, &config);

    assert_eq_discover_scan_inputs_result(result, Ok(expected));
    Ok(())
}
