//! Process-level contract of `diff <workflow.yaml> (--against | --db)`:
//! exit 0 no semantic changes, 1 semantic changes, 2 error.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SCHEMA: &str = "velvet-ballistics/cli-output/v1";

const OLD: &str = "version: velvet-ballistics/v1
name: demo
when: {manual: {}}
steps:
  - id: seed
    set: {output: base, value: '1'}
  - id: act
    do: {action: '1', input: '0'}
    try_again: {max_attempts: 3, delay: 1s}
  - id: done
    finish: {result: '$base'}
";

const PLAIN: &str = "version: velvet-ballistics/v1
name: plain
when: {manual: {}}
steps:
  - id: seed
    set: {output: base, value: '1'}
  - id: done
    finish: {result: '$base'}
";

fn write(dir: &Path, name: &str, text: &str) -> TestResult<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, text)?;
    Ok(path)
}

fn diff(args: &[&Path], extra: &[&str]) -> TestResult<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.arg("diff").args(args).args(extra);
    Ok(command.output()?)
}

fn against(new: &Path, old: &Path, extra: &[&str]) -> TestResult<Output> {
    diff(&[new, Path::new("--against"), old], extra)
}

/// Decodes a `--emit yaml|postcard` stream into its JSON document.
fn decode(bytes: &[u8], emit: &str) -> TestResult<Value> {
    if emit == "yaml" {
        return Ok(serde_saphyr::from_str(std::str::from_utf8(bytes)?)?);
    }
    let header = bytes.get(..52).ok_or("postcard header")?;
    assert_eq!(header.get(..4), Some(b"VCLA".as_slice()));
    let payload = bytes.get(52..).ok_or("postcard payload")?;
    assert_eq!(
        header.get(12..16),
        Some(u32::try_from(payload.len())?.to_le_bytes().as_slice())
    );
    assert_eq!(
        header.get(16..48),
        Some(blake3::hash(payload).as_bytes().as_slice())
    );
    let (_version, _kind, _content, json): (u16, u16, u32, Vec<u8>) =
        postcard::from_bytes(payload)?;
    Ok(serde_json::from_slice(&json)?)
}

fn grown(yaml: &str) -> String {
    yaml.replace(
        "  - id: done",
        "  - id: more\n    set: {output: x, value: '2'}\n  - id: done",
    )
}

#[test]
fn identical_files_exit_zero_with_exact_text_report() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", OLD)?;
    let output = against(&new, &old, &[])?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 4, "{stdout}");
    assert_eq!(
        lines.first().copied(),
        Some(format!("workflow diff: {}", new.display()).as_str())
    );
    assert!(
        lines.get(1).is_some_and(|l| l.ends_with("(identical)")),
        "{stdout}"
    );
    assert_eq!(lines.get(2).copied(), Some("text changes: +0 -0"));
    assert_eq!(
        lines.get(3).copied(),
        Some("no semantic changes (steps: 3)")
    );
    Ok(())
}

#[test]
fn comment_only_edit_is_textual_but_exits_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", &format!("# reviewed\n{OLD}"))?;
    let output = against(&new, &old, &[])?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("text changes: +1 -0"), "{stdout}");
    assert!(stdout.contains("  + 1: # reviewed"), "{stdout}");
    assert!(
        stdout.contains("no semantic changes (steps: 3)"),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn retry_policy_change_exits_one_and_names_the_step() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let tuned = OLD.replace("max_attempts: 3", "max_attempts: 2");
    let new = write(dir.path(), "new.yaml", &tuned)?;
    let output = against(&new, &old, &[])?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("  [retry_policy] step act:do changed: max_attempts=3 delay_ms=1000 -> max_attempts=2 delay_ms=1000"),
        "{stdout}"
    );
    assert!(
        stdout.contains("semantic changes: 1 (steps 3 -> 3)"),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn yaml_and_postcard_emit_carry_the_same_machine_readable_envelope() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", &grown(OLD))?;
    let yaml = against(&new, &old, &["--emit", "yaml"])?;
    let postcard = against(&new, &old, &["--emit", "postcard"])?;
    for output in [&yaml, &postcard] {
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stderr.is_empty());
    }
    let from_yaml = decode(&yaml.stdout, "yaml")?;
    let from_postcard = decode(&postcard.stdout, "postcard")?;
    assert_eq!(from_yaml, from_postcard);
    assert_eq!(from_yaml["kind"], "WorkflowDiff");
    assert_eq!(from_yaml["schema_version"], SCHEMA);
    assert_eq!(from_yaml["workflow"], new.display().to_string());
    assert_eq!(from_yaml["semantic_changes"], true);
    assert_eq!(
        from_yaml["baseline"],
        json!({"mode": "against", "path": old.display().to_string()})
    );
    assert_eq!(from_yaml["digest"]["equal"], false);
    assert_eq!(from_yaml["text"]["available"], true);
    assert_eq!(from_yaml["text"]["added"], 2);
    assert_eq!(from_yaml["text"]["removed"], 0);
    assert_eq!(from_yaml["semantic"]["old_step_count"], 3);
    assert_eq!(from_yaml["semantic"]["new_step_count"], 4);
    assert_eq!(
        from_yaml["semantic"]["changes"][0],
        json!({"category": "step_count", "message": "step count changed: 3 -> 4"})
    );
    Ok(())
}

#[test]
fn clean_structured_report_exits_zero_with_no_changes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", OLD)?;
    let output = against(&new, &old, &["--emit", "postcard"])?;
    assert_eq!(output.status.code(), Some(0));
    let value = decode(&output.stdout, "postcard")?;
    assert_eq!(value["semantic_changes"], false);
    assert_eq!(value["semantic"]["changes"], json!([]));
    assert_eq!(value["digest"]["equal"], true);
    Ok(())
}

#[test]
fn missing_baseline_file_exits_two_with_diagnostic_on_stderr() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new = write(dir.path(), "new.yaml", OLD)?;
    let missing = dir.path().join("gone.yaml");
    let output = against(&new, &missing, &[])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with(&format!("error reading {}:", missing.display())),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn structured_failure_is_a_stderr_document_with_exit_two() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new = write(dir.path(), "new.yaml", OLD)?;
    let missing = dir.path().join("gone.yaml");
    for emit in ["yaml", "postcard"] {
        let output = against(&new, &missing, &["--emit", emit])?;
        assert_eq!(output.status.code(), Some(2), "{emit}");
        assert!(output.stdout.is_empty(), "{emit}");
        let value = decode(&output.stderr, emit)?;
        assert_eq!(value["success"], false);
        assert_eq!(value["error"], "ValidationFailed");
        assert_eq!(value["exit_code"], 2);
        let message = value["message"].as_str().ok_or("message")?;
        assert!(message.contains("gone.yaml"), "{message}");
    }
    Ok(())
}

#[test]
fn uncompilable_workflow_exits_two_not_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", "name: [unterminated\n")?;
    let output = against(&new, &old, &[])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("new.yaml"), "{stderr}");
    Ok(())
}

#[test]
fn db_baseline_maps_identical_to_zero_and_changed_to_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let db = dir.path().join("journal");
    let compiled = vb_compile::compile_workflow(PLAIN.as_bytes())?;
    let journal = vb_storage::FjallJournal::open(&db, None)?;
    vb_storage::submit_artifact(&journal, &compiled, vb_core::RuntimePolicy::Journaled)?;
    drop(journal);
    let same = write(dir.path(), "same.yaml", PLAIN)?;
    let output = diff(&[&same, Path::new("--db"), &db], &[])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let changed = write(dir.path(), "changed.yaml", &grown(PLAIN))?;
    let output = diff(&[&changed, Path::new("--db"), &db], &[])?;
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("step count changed: 2 -> 3"), "{stdout}");
    assert!(stdout.contains("text changes: unavailable"), "{stdout}");
    Ok(())
}

#[test]
fn missing_or_conflicting_baseline_is_a_usage_error_with_exit_two() -> TestResult {
    let dir = tempfile::tempdir()?;
    let new = write(dir.path(), "new.yaml", OLD)?;
    let output = diff(&[&new], &[])?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("--against") && stderr.contains("--db"),
        "{stderr}"
    );
    let both = diff(
        &[
            &new,
            Path::new("--against"),
            &new,
            Path::new("--db"),
            dir.path(),
        ],
        &[],
    )?;
    assert_eq!(both.status.code(), Some(2));
    assert!(String::from_utf8(both.stderr)?.contains("mutually exclusive"));
    Ok(())
}

#[test]
fn oversized_postcard_report_is_an_output_failure_with_exit_two() -> TestResult {
    let dir = tempfile::tempdir()?;
    let noise = |tag: &str| -> String {
        (0..1500)
            .map(|n| format!("# {tag} comment line number {n:05}\n"))
            .collect()
    };
    let old = write(dir.path(), "old.yaml", &format!("{}{OLD}", noise("old")))?;
    let new = write(dir.path(), "new.yaml", &format!("{}{OLD}", noise("new")))?;
    let output = against(&new, &old, &["--emit", "postcard"])?;
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let value = decode(&output.stderr, "postcard")?;
    assert_eq!(value["error"], "ValidationFailed");
    let message = value["message"].as_str().ok_or("message")?;
    assert!(message.starts_with("output failed:"), "{message}");
    let text = against(&new, &old, &[])?;
    assert_eq!(
        text.status.code(),
        Some(0),
        "the same diff renders fine as text"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn stdout_write_failure_exits_two_instead_of_pretending_success() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "old.yaml", OLD)?;
    let new = write(dir.path(), "new.yaml", &grown(OLD))?;
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full")?;
    for emit in ["text", "yaml"] {
        let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("diff")
            .arg(&new)
            .arg("--against")
            .arg(&old)
            .args(["--emit", emit])
            .stdout(Stdio::from(full.try_clone()?))
            .output()?;
        assert_eq!(output.status.code(), Some(2), "{emit} must not exit 1 or 0");
        assert!(!output.stderr.is_empty(), "{emit} must explain the failure");
    }
    Ok(())
}
