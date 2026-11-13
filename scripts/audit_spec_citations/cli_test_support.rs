use std::fs;
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_fixture_id() -> Result<u64, Box<dyn std::error::Error>> {
    let prior = FIXTURE_COUNTER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| "fixture counter overflow")?;
    Ok(prior)
}

pub fn tmp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let val = std::env::var("TMPDIR")?;
    let p = PathBuf::from(val);
    if !p.is_absolute() {
        return Err("TMPDIR is not absolute".into());
    }
    if !p.exists() {
        return Err(format!("TMPDIR {} does not exist", p.display()).into());
    }
    Ok(p)
}

pub fn make_fixture_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let tmp = tmp_dir()?;
    let id = next_fixture_id()?;
    let pid = std::process::id();
    let p = tmp.join(format!("citation-test-{}-{}-{}", pid, name, id));
    fs::create_dir_all(&p)?;
    Ok(p)
}

pub fn scanner_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    let p = root.join("target/gate-tools/audit-spec-citations");
    if !p.is_file() {
        return Err(format!("scanner binary not found at {}", p.display()).into());
    }
    Ok(p)
}

pub fn write_file(root: &Path, rel: &str, content: &str) -> Result<PathBuf, std::io::Error> {
    let p = root.join(rel);
    let parent = p
        .parent()
        .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidInput, "no parent"))?;
    fs::create_dir_all(parent)?;
    let mut f = fs::File::create(&p)?;
    f.write_all(content.as_bytes())?;
    Ok(p)
}

pub fn run_scanner(root: &Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    let scanner = scanner_path()?;
    let mut cmd = Command::new(&scanner);
    cmd.arg(root);
    cmd.args(args);
    cmd.current_dir(root);
    let cargo_home = root.join("cargo-home");
    fs::create_dir_all(&cargo_home)?;
    cmd.env("CARGO_HOME", &cargo_home);
    Ok(cmd.output()?)
}

pub fn exit_code(output: &Output) -> i32 {
    match output.status.code() {
        Some(c) => c,
        None => -1,
    }
}

pub fn stdout_str(output: &Output) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(output.stdout.clone())?)
}

pub fn make_basic_workspace(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        root,
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let crates_dir = root.join("crates/test/src");
    fs::create_dir_all(&crates_dir)?;
    let verus_dir = root.join("verification/verus");
    fs::create_dir_all(&verus_dir)?;
    write_file(
        root,
        "crates/test/src/main.rs",
        "fn make_widget() -> i32 { 42 }\n",
    )?;
    Ok(())
}

pub fn check_pass(
    output: &Output,
    file: &str,
    checks: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(exit_code(output), 0, "expected exit 0");
    let stdout = stdout_str(output)?;
    let expected = format!("{}: {} checks, 0 failures", file, checks);
    assert!(
        stdout.contains(&expected),
        "missing pass line: {}",
        expected
    );
    let total = format!("TOTAL: {} checks, 0 failures", checks);
    assert!(stdout.contains(&total), "missing total: {}", total);
    Ok(())
}

pub fn check_fail(output: &Output, pattern: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(exit_code(output), 1, "expected exit 1");
    let stdout = stdout_str(output)?;
    assert!(
        stdout.contains(pattern),
        "expected '{}', got:\n{}",
        pattern,
        stdout
    );
    Ok(())
}

pub fn appears_before(output: &str, first: &str, second: &str) -> bool {
    matches!((output.find(first), output.find(second)), (Some(a), Some(b)) if a < b)
}

fn range_fixture(name: &str, range_text: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let root = make_fixture_dir(name)?;
    write_file(&root, "crates/test/src/main.rs", "fn make_widget() {}\n")?;
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        &format!("// See crates/test/src/main.rs:{range_text} for make_widget\n"),
    )?;
    run_scanner(&root, &["verification/verus/test_spec.rs"])
}

#[test]
fn huge_standalone_range_fails_and_counts() -> Result<(), Box<dyn std::error::Error>> {
    let output = range_fixture("huge-standalone", "18446744073709551616")?;
    assert_eq!(exit_code(&output), 1);
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 1 checks, 1 failures"));
    assert!(stdout.contains("TOTAL: 1 checks, 1 failures"));
    assert!(stdout.contains(
        "crates/test/src/main.rs:18446744073709551616-18446744073709551616 out of range (file has 1 lines)"
    ));
    Ok(())
}

#[test]
fn mixed_valid_and_huge_ranges_continue_and_count_both() -> Result<(), Box<dyn std::error::Error>> {
    let output = range_fixture("mixed-huge", "1,18446744073709551616")?;
    assert_eq!(exit_code(&output), 1);
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 2 checks, 1 failures"));
    assert!(stdout.contains("TOTAL: 2 checks, 1 failures"));
    assert!(stdout.contains("18446744073709551616-18446744073709551616 out of range"));
    Ok(())
}

#[test]
fn leading_zero_spellings_deduplicate_after_resolution() -> Result<(), Box<dyn std::error::Error>> {
    let output = range_fixture("leading-zero-dedup", "1,01,0001,1-1")?;
    check_pass(&output, "test_spec.rs", 1)
}

#[test]
fn signed_negative_endpoint_keeps_out_of_range_diagnostic() -> Result<(), Box<dyn std::error::Error>>
{
    let output = range_fixture("negative-endpoint", "1--2")?;
    assert_eq!(exit_code(&output), 1);
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 1 checks, 1 failures"));
    assert!(stdout.contains(":1--2 out of range (file has 1 lines)"));
    Ok(())
}

#[test]
fn negative_zero_deduplicates_with_zero_and_preserves_first_display()
-> Result<(), Box<dyn std::error::Error>> {
    let output = range_fixture("negative-zero", "1--0,1-0")?;
    assert_eq!(exit_code(&output), 1);
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 1 checks, 1 failures"));
    assert!(stdout.contains("TOTAL: 1 checks, 1 failures"));
    assert!(stdout.contains(":1--0 out of range (file has 1 lines)"));
    Ok(())
}

fn mixed_identity_scan(
    registry_first: bool,
    second_symbol: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let root = make_fixture_dir(if registry_first {
        "registry-first"
    } else {
        "repository-first"
    })?;
    let ordinary = "crates/cargo-home/registry/src/index-demo/forge-1.2.3/src/lib.rs";
    let registry = "/.cargo/registry/src/index-demo/forge-1.2.3/src/lib.rs";
    write_file(&root, ordinary, "pub fn Widget() {}\n")?;
    write_file(
        &root,
        "Cargo.lock",
        "name = \"forge\"\nversion = \"1.2.3\"\n",
    )?;
    let (first, second) = if registry_first {
        (registry, ordinary)
    } else {
        (ordinary, registry)
    };
    write_file(
        &root,
        "verification/verus/test_spec.rs",
        &format!(
            "// See {first}:1 for Widget\n{}// See {second}:1 for {second_symbol}\n",
            "\n".repeat(12)
        ),
    )?;
    let scanner = scanner_path()?;
    let cargo_home = root.join("crates/cargo-home");
    let output = Command::new(&scanner)
        .arg(&root)
        .arg("verification/verus/test_spec.rs")
        .current_dir(&root)
        .env("CARGO_HOME", &cargo_home)
        .output()?;
    write_file(
        &root,
        "mixed-domain-process.txt",
        &format!(
            "cwd={}\nCARGO_HOME={}\ncommand={} {} verification/verus/test_spec.rs\nexit={}\nstdout:\n{}stderr:\n{}",
            root.display(),
            cargo_home.display(),
            scanner.display(),
            root.display(),
            exit_code(&output),
            stdout_str(&output)?,
            String::from_utf8(output.stderr.clone())?,
        ),
    )?;
    Ok(output)
}

fn mixed_identity_failure(registry_first: bool) -> Result<(), Box<dyn std::error::Error>> {
    let output = mixed_identity_scan(registry_first, "AbsentMarker")?;
    check_fail(&output, "line 14:")?;
    check_fail(&output, "span contains none of context idents")?;
    let stdout = stdout_str(&output)?;
    assert!(stdout.contains("test_spec.rs: 2 checks, 1 failures"));
    assert!(stdout.contains("TOTAL: 2 checks, 1 failures"));
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn repository_then_registry_preserves_bad_context_check() -> Result<(), Box<dyn std::error::Error>>
{
    mixed_identity_failure(false)
}

#[test]
fn registry_then_repository_preserves_bad_context_check() -> Result<(), Box<dyn std::error::Error>>
{
    mixed_identity_failure(true)
}

#[test]
fn both_identity_orders_count_two_good_citations() -> Result<(), Box<dyn std::error::Error>> {
    for registry_first in [false, true] {
        let output = mixed_identity_scan(registry_first, "Widget")?;
        check_pass(&output, "test_spec.rs", 2)?;
        assert!(output.stderr.is_empty());
    }
    Ok(())
}
