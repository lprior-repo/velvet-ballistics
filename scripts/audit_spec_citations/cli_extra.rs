use super::*;
use std::fs;
use std::path::PathBuf;

pub(super) fn multiple_ranges_pass() -> TestResult {
    let root = make_fixture_dir("multiple-ranges")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn make_widget() -> i32 { 42 }\nfn destroy_widget() {}\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1,2 for make_widget destroy_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 2)
}

pub(super) fn context_identifier_matching_passes() -> TestResult {
    let root = make_fixture_dir("context-pass")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn create_widget() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for create_widget function\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

pub(super) fn context_identifier_matching_fails() -> TestResult {
    let root = make_fixture_dir("context-fail")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn create_widget() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for nonexistent_identifier_xyz\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "span contains none of context idents")
}

pub(super) fn case_fold_matching_passes() -> TestResult {
    let root = make_fixture_dir("case-fold")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn HELLO_WORLD() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for hello_world\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

pub(super) fn default_discovery_scans_verus_dir() -> TestResult {
    let root = make_fixture_dir("default-discovery")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/spec1.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    write_file(
        &root,
        "verification/verus/spec2.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &[])?;
    assert_eq!(exit_code(&output), 0, "expected exit 0");
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("spec1.rs"), "expected spec1 in output");
    assert!(stdout.contains("spec2.rs"), "expected spec2 in output");
    assert!(appears_before(&stdout, "spec1.rs:", "spec2.rs:"));
    assert!(
        stdout.contains("TOTAL: 2 checks, 0 failures"),
        "expected total of 2 checks"
    );
    Ok(())
}

pub(super) fn explicit_file_args_override_default() -> TestResult {
    let root = make_fixture_dir("explicit-args")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/spec1.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    write_file(
        &root,
        "verification/verus/spec2.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(
        &root,
        &["verification/verus/spec2.rs", "verification/verus/spec1.rs"],
    )?;
    assert_eq!(exit_code(&output), 0, "expected exit 0");
    let stdout = stdout_str(&output)?;
    assert!(appears_before(&stdout, "spec2.rs:", "spec1.rs:"));
    assert!(
        stdout.contains("TOTAL: 2 checks, 0 failures"),
        "expected total of 2 checks"
    );
    Ok(())
}

pub(super) fn target_directory_excluded() -> TestResult {
    let root = make_fixture_dir("target-excluded")?;
    write_file(
        &root,
        "target/generated/main.rs",
        "fn generated_widget() {}\n",
    )?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

pub(super) fn resolved_aliases_and_repeated_ranges_count_once_per_file() -> TestResult {
    let root = make_fixture_dir("range-dedup")?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n// See src/main.rs:1-1 for make_widget\n// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

pub(super) fn invalid_ranges_are_counted_and_later_ranges_still_run() -> TestResult {
    let root = make_fixture_dir("range-count-failures")?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:0,1,99 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "out of range")?;
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 3 checks, 2 failures"));
    assert!(stdout.contains("TOTAL: 3 checks, 2 failures"));
    assert!(stdout.contains(":0-0 out of range (file has 1 lines)"));
    assert!(stdout.contains(":99-99 out of range (file has 1 lines)"));
    Ok(())
}

pub(super) fn ambiguous_suffix_uses_first_candidate_after_verus_lookup() -> TestResult {
    cli_revision_c::traversal_first_suffix()
}

pub(super) fn duplicate_explicit_file_keeps_one_row_and_counts_both_scans() -> TestResult {
    let root = make_fixture_dir("duplicate-explicit")?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(
        &root,
        &[
            "verification/verus/test_spec.rs",
            "verification/verus/test_spec.rs",
        ],
    )?;
    assert_eq!(exit_code(&output), 0);
    let stdout = stdout_str(&output)?;
    assert_eq!(
        stdout.matches("test_spec.rs: 1 checks, 0 failures").count(),
        1
    );
    assert!(stdout.contains("TOTAL: 2 checks, 0 failures"));
    Ok(())
}

pub(super) fn malformed_numeric_range_fails_through_the_cli() -> TestResult {
    let root = make_fixture_dir("malformed-range")?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1---2 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "invalid range number")?;
    assert!(stdout_str(&output)?.contains("TOTAL: 0 checks, 1 failures"));
    Ok(())
}

pub(super) fn empty_default_discovery_fails_closed() -> TestResult {
    let root = make_fixture_dir("empty-default")?;
    fs::create_dir_all(root.join("verification/verus"))?;
    let output = run_scanner(&root, &[])?;
    assert_eq!(exit_code(&output), 3);
    let stderr = String::from_utf8(output.stderr.clone())?;
    assert!(stderr.contains("no verification/verus Rust specs to audit"));
    assert!(output.stdout.is_empty());
    Ok(())
}

pub(super) fn registry_fallback_uses_lock_pin_through_the_cli() -> TestResult {
    let root = registry_fixture("registry-fallback", "index-demo", "other-index", "1.2.3")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

pub(super) fn registry_lock_mismatch_fails_through_the_cli() -> TestResult {
    let root = registry_fixture("registry-mismatch", "index-demo", "index-demo", "1.2.4")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "Cargo.lock pins 1.2.4")
}

fn registry_fixture(
    name: &str,
    source_index: &str,
    cited_index: &str,
    lock_version: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = make_fixture_dir(name)?;
    let source = format!("cargo-home/registry/src/{source_index}/forge-1.2.3/src/lib.rs");
    write_file(&root, &source, "pub fn forge_symbol() {}\n")?;
    write_file(
        &root,
        "Cargo.lock",
        &format!("name = \"forge\"\nversion = \"{lock_version}\"\n"),
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        &format!(
            "// See /.cargo/registry/src/{cited_index}/forge-1.2.3/src/lib.rs:1 for forge_symbol\n"
        ),
    )?;
    Ok(root)
}

#[cfg(unix)]
pub(super) fn unreadable_utf8_source_reports_cli_io_error() -> TestResult {
    let root = make_fixture_dir("invalid-utf8-source")?;
    write_file(&root, "crates/test/src/main.rs", "")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for widget\n",
    )?;
    fs::write(root.join("crates/test/src/main.rs"), [0xff])?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    assert_eq!(exit_code(&output), 3);
    let stderr = String::from_utf8(output.stderr.clone())?;
    assert!(stderr.contains("cannot read"));
    assert!(output.stdout.is_empty());
    Ok(())
}

#[cfg(unix)]
pub(super) fn source_index_does_not_follow_symlink_directory() -> TestResult {
    let root = make_fixture_dir("symlink-directory")?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    std::os::unix::fs::symlink(root.join("crates"), root.join("crates/loop"))?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See main.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}
