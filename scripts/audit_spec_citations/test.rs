#![forbid(unsafe_code)]
#![deny(unused_must_use)]

mod cli_extra;
mod cli_revision_c;
mod cli_test_support;
use cli_test_support::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn explicit_path_citation_passes() -> TestResult {
    let root = make_fixture_dir("explicit-path")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for make_widget function\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn missing_file_citation_fails() -> TestResult {
    let root = make_fixture_dir("missing-file")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/nonexistent/src/main.rs:1 for Widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "cited file missing")
}

#[test]
fn invalid_line_range_fails() -> TestResult {
    let root = make_fixture_dir("invalid-range")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:100 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "out of range")
}

#[test]
fn reversed_line_range_fails() -> TestResult {
    let root = make_fixture_dir("reversed-range")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:5-1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "out of range")
}

#[test]
fn zero_line_range_fails() -> TestResult {
    let root = make_fixture_dir("zero-range")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:0 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "out of range")
}

#[test]
fn beyond_eof_range_fails() -> TestResult {
    let root = make_fixture_dir("beyond-eof")?;
    make_basic_workspace(&root)?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1-100 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "out of range")
}

#[test]
fn multiple_ranges_pass() -> TestResult {
    cli_extra::multiple_ranges_pass()
}

#[test]
fn context_identifier_matching_passes() -> TestResult {
    cli_extra::context_identifier_matching_passes()
}

#[test]
fn context_identifier_matching_fails() -> TestResult {
    cli_extra::context_identifier_matching_fails()
}

#[test]
fn mirror_suffix_handling_passes() -> TestResult {
    let root = make_fixture_dir("mirror-suffix")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn process_data() -> i32 { 0 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for Mirror_process_data\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn underscore_component_matching_passes() -> TestResult {
    let root = make_fixture_dir("underscore")?;
    write_file(
        &root,
        "crates/test/src/main.rs",
        "fn validate_input() -> bool { true }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See crates/test/src/main.rs:1 for validate function\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn case_fold_matching_passes() -> TestResult {
    cli_extra::case_fold_matching_passes()
}

#[test]
fn code_line_citation_ignored() -> TestResult {
    let root = make_fixture_dir("code-line")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "let x = 1; // See crates/missing/src/main.rs:1 for Widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 0)
}

#[test]
fn default_discovery_scans_verus_dir() -> TestResult {
    cli_extra::default_discovery_scans_verus_dir()
}

#[test]
fn explicit_file_args_override_default() -> TestResult {
    cli_extra::explicit_file_args_override_default()
}

#[test]
fn ambiguous_basename_fails() -> TestResult {
    let root = make_fixture_dir("ambiguous-basename")?;
    write_file(&root, "crates/a/src/main.rs", "fn foo() {}\n")?;
    write_file(&root, "crates/b/src/main.rs", "fn bar() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See main.rs:1 for foo\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "ambiguous/unresolvable short path")
}

#[test]
fn unique_basename_resolves() -> TestResult {
    let root = make_fixture_dir("unique-basename")?;
    write_file(
        &root,
        "crates/test/src/unique.rs",
        "fn make_widget() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See unique.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn suffix_path_resolves() -> TestResult {
    let root = make_fixture_dir("suffix-path")?;
    write_file(
        &root,
        "crates/test/src/suffix.rs",
        "fn make_widget() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See src/suffix.rs:1 for make_widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn verus_relative_path_resolves() -> TestResult {
    let root = make_fixture_dir("verus-relative")?;
    write_file(
        &root,
        "verification/verus/dep.rs",
        "fn dep_fn() -> i32 { 42 }\n",
    )?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// See dep.rs:1 for dep_fn\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn target_directory_excluded() -> TestResult {
    cli_extra::target_directory_excluded()
}

#[test]
fn resolved_aliases_and_repeated_ranges_count_once_per_file() -> TestResult {
    cli_extra::resolved_aliases_and_repeated_ranges_count_once_per_file()
}

#[test]
fn invalid_ranges_are_counted_and_later_ranges_still_run() -> TestResult {
    cli_extra::invalid_ranges_are_counted_and_later_ranges_still_run()
}

#[test]
fn ambiguous_suffix_uses_first_candidate_after_verus_lookup() -> TestResult {
    cli_extra::ambiguous_suffix_uses_first_candidate_after_verus_lookup()
}

#[test]
fn duplicate_explicit_file_keeps_one_row_and_counts_both_scans() -> TestResult {
    cli_extra::duplicate_explicit_file_keeps_one_row_and_counts_both_scans()
}

#[test]
fn malformed_numeric_range_fails_through_the_cli() -> TestResult {
    cli_extra::malformed_numeric_range_fails_through_the_cli()
}

#[test]
fn empty_default_discovery_fails_closed() -> TestResult {
    cli_extra::empty_default_discovery_fails_closed()
}

#[test]
fn registry_fallback_uses_lock_pin_through_the_cli() -> TestResult {
    cli_extra::registry_fallback_uses_lock_pin_through_the_cli()
}

#[test]
fn registry_lock_mismatch_fails_through_the_cli() -> TestResult {
    cli_extra::registry_lock_mismatch_fails_through_the_cli()
}

#[cfg(unix)]
#[test]
fn unreadable_utf8_source_reports_cli_io_error() -> TestResult {
    cli_extra::unreadable_utf8_source_reports_cli_io_error()
}

#[cfg(unix)]
#[test]
fn source_index_does_not_follow_symlink_directory() -> TestResult {
    cli_extra::source_index_does_not_follow_symlink_directory()
}
