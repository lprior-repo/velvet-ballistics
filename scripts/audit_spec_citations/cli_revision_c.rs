use super::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Output;

fn write_spec(root: &Path, citation: &str, symbol: &str) -> TestResult {
    write_file(
        root,
        "verification/verus/test_spec.rs",
        &format!("// See {citation} for {symbol}\n"),
    )?;
    Ok(())
}

fn counts(output: &Output, checks: usize, failures: usize) -> TestResult {
    let stdout = stdout_str(output)?;
    assert!(stdout.contains(&format!(
        "test_spec.rs: {checks} checks, {failures} failures"
    )));
    assert!(stdout.contains(&format!("TOTAL: {checks} checks, {failures} failures")));
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn python_unit_separator_missing_comment_matches_normal_control() -> TestResult {
    let root = make_fixture_dir("python-whitespace")?;
    for prefix in ["", "\u{001f}"] {
        write_file(
            &root,
            "verification/verus/test_spec.rs",
            &format!("{prefix}// See crates/missing/src/lib.rs:1 for Widget\n"),
        )?;
        let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
        check_fail(
            &output,
            "line 1: cited file missing: crates/missing/src/lib.rs",
        )?;
        counts(&output, 0, 1)?;
    }
    Ok(())
}

#[test]
fn adjacent_citations_resume_after_previous_match() -> TestResult {
    let root = make_fixture_dir("adjacent-citations")?;
    write_file(&root, "a.rs", "pub fn Widget() {}\n")?;
    write_file(&root, "b.rs", "pub fn Widget() {}\n")?;
    for delimiter in ["", " "] {
        write_spec(&root, &format!("a.rs:1{delimiter}b.rs:1"), "Widget")?;
        let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
        check_pass(&output, "test_spec.rs", 2)?;
        counts(&output, 2, 0)?;
    }
    Ok(())
}

#[test]
fn ordinary_absolute_path_cannot_reset_verus_prefix() -> TestResult {
    let outer = make_fixture_dir("absolute-citation")?;
    let outside = write_file(&outer, "outside.rs", "fn Outsidesymbol() {}\n")?;
    let root = outer.join("repo");
    write_file(
        &root,
        "verification/verus/relative/source.rs",
        "fn Outsidesymbol() {}\n",
    )?;
    write_spec(&root, "relative/source.rs:1", "Outsidesymbol")?;
    check_pass(
        &run_scanner(&root, &["verification/verus/test_spec.rs"])?,
        "test_spec.rs",
        1,
    )?;
    write_spec(&root, &format!("{}:1", outside.display()), "Outsidesymbol")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(
        &output,
        &format!("ambiguous/unresolvable short path {}", outside.display()),
    )?;
    counts(&output, 0, 1)
}

#[test]
fn verus_relative_file_beats_competing_suffix_contents() -> TestResult {
    let root = make_fixture_dir("verus-priority")?;
    write_file(&root, "crates/z/src/main.rs", "fn Zebrasignal() {}\n")?;
    write_file(&root, "crates/a/src/main.rs", "fn Alphasignal() {}\n")?;
    write_file(
        &root,
        "verification/verus/src/main.rs",
        "fn Omegasignal() {}\n",
    )?;
    write_spec(&root, "src/main.rs:1", "Omegasignal")?;
    check_pass(
        &run_scanner(&root, &["verification/verus/test_spec.rs"])?,
        "test_spec.rs",
        1,
    )?;
    write_spec(&root, "src/main.rs:1", "Alphasignal")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(
        &output,
        "verification/verus/src/main.rs:1-1 span contains none of context idents",
    )?;
    counts(&output, 1, 1)
}

#[test]
fn registry_fallback_rejects_two_indexes_but_literal_wins() -> TestResult {
    let root = make_fixture_dir("registry-two-indexes")?;
    write_file(
        &root,
        "cargo-home/registry/src/index-z/forge-1.2.3/src/lib.rs",
        "fn Zebrasignal() {}\n",
    )?;
    write_file(
        &root,
        "cargo-home/registry/src/index-a/forge-1.2.3/src/lib.rs",
        "fn Alphasignal() {}\n",
    )?;
    write_file(
        &root,
        "Cargo.lock",
        "name = \"forge\"\nversion = \"1.2.3\"\n",
    )?;
    write_spec(
        &root,
        "/.cargo/registry/src/missing-index/forge-1.2.3/src/lib.rs:1",
        "Zebrasignal",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(
        &output,
        "line 1: registry path ambiguous across 2 index-hash dirs",
    )?;
    counts(&output, 0, 1)?;
    write_spec(
        &root,
        "/.cargo/registry/src/index-z/forge-1.2.3/src/lib.rs:1",
        "Zebrasignal",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)?;
    counts(&output, 1, 0)?;
    write_spec(
        &root,
        "/.cargo/registry/src/index-z/forge-1.2.3/src/./lib.rs:1 /.cargo/registry/src/index-z/forge-1.2.3/src//lib.rs:1",
        "Zebrasignal",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 1)?;
    counts(&output, 1, 0)
}

pub(super) fn traversal_first_suffix() -> TestResult {
    let root = make_fixture_dir("suffix-traversal-order")?;
    write_file(&root, "crates/z/src/main.rs", "fn Zebrasignal() {}\n")?;
    write_file(&root, "crates/a/src/main.rs", "fn Alphasignal() {}\n")?;
    let order = fs::read_dir(root.join("crates"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    let first = order.last().ok_or("no reference directory entries")?;
    let (symbol, other) = match first.to_str() {
        Some("z") => ("Zebrasignal", "Alphasignal"),
        Some("a") => ("Alphasignal", "Zebrasignal"),
        _ => return Err("unexpected reference directory".into()),
    };
    let trace = format!("created=z,a; read_dir={order:?}; CPython3.14 LIFO first={first:?}\n");
    write_file(&root, "traversal-reference.txt", &trace)?;
    writeln!(std::io::stdout().lock(), "{}: {trace}", root.display())?;
    write_spec(&root, "src/main.rs:1", symbol)?;
    check_pass(
        &run_scanner(&root, &["verification/verus/test_spec.rs"])?,
        "test_spec.rs",
        1,
    )?;
    write_spec(&root, "src/main.rs:1", other)?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(
        &output,
        &format!(
            "crates/{}/src/main.rs:1-1 span contains none",
            first.to_string_lossy()
        ),
    )?;
    counts(&output, 1, 1)
}

#[test]
fn shallow_match_precedes_nested_lexical_candidate() -> TestResult {
    let root = make_fixture_dir("suffix-shallow-before-deep")?;
    write_file(&root, "crates/z/src/main.rs", "fn Zebrasignal() {}\n")?;
    write_file(&root, "crates/z/src/a/src/main.rs", "fn Alphasignal() {}\n")?;
    write_spec(&root, "src/main.rs:1", "Zebrasignal")?;
    check_pass(
        &run_scanner(&root, &["verification/verus/test_spec.rs"])?,
        "test_spec.rs",
        1,
    )?;
    write_spec(&root, "src/main.rs:1", "Alphasignal")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(
        &output,
        "crates/z/src/main.rs:1-1 span contains none of context idents",
    )?;
    counts(&output, 1, 1)
}

#[test]
fn suffix_requires_literal_leading_slash_in_indexed_relative_path() -> TestResult {
    let root = make_fixture_dir("suffix-slash-boundary")?;
    write_file(&root, "src/main.rs", "fn Rootsymbol() {}\n")?;
    write_spec(&root, "src/main.rs:1", "Rootsymbol")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "ambiguous/unresolvable short path src/main.rs")?;
    counts(&output, 0, 1)?;
    write_file(&root, "crates/z/src/main.rs", "fn Nestedsymbol() {}\n")?;
    write_spec(&root, "src/main.rs:1", "Nestedsymbol")?;
    check_pass(
        &run_scanner(&root, &["verification/verus/test_spec.rs"])?,
        "test_spec.rs",
        1,
    )
}

#[test]
fn rs_named_directory_participates_in_basename_ambiguity() -> TestResult {
    let root = make_fixture_dir("directory-rs-match")?;
    write_file(&root, "crates/data.rs", "fn Widget() {}\n")?;
    fs::create_dir_all(root.join("specs/data.rs"))?;
    write_spec(&root, "data.rs:1", "Widget")?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_fail(&output, "ambiguous/unresolvable short path data.rs")?;
    counts(&output, 0, 1)
}

#[test]
fn explicit_path_spelling_keeps_original_string_dedup_identity() -> TestResult {
    let root = make_fixture_dir("explicit-spelling-identity")?;
    write_file(&root, "crates/test/main.rs", "fn Widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        "// crates/test/main.rs:1 crates/test/./main.rs:1 crates/test//main.rs:1 crates/test/main.rs:1-1 for Widget\n",
    )?;
    let output = run_scanner(&root, &["verification/verus/test_spec.rs"])?;
    check_pass(&output, "test_spec.rs", 3)?;
    counts(&output, 3, 0)
}
