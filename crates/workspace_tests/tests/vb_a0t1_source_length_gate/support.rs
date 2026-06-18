use crate::fixture_sources::{
    adversarial_hot_function_source, hot_function_source, long_file_source, non_utf8_source_bytes,
    unsafe_hot_function_source,
};
use chrono::{Datelike, Utc};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub(crate) type TestResult<T> = Result<T, Box<dyn Error>>;

const CHECK_SOURCE_LENGTH_SH: &str = include_str!("../../../../scripts/check-source-length.sh");
const CHECK_SOURCE_LENGTH_RS: &str = include_str!("../../../../scripts/check-source-length.rs");
const SOURCE_LENGTH_GATE_RS: &str = include_str!("../../../../scripts/source_length_gate.rs");
const SOURCE_LENGTH_LEDGER_RS: &str = include_str!("../../../../scripts/source_length_ledger.rs");
const SOURCE_LENGTH_SCAN_RS: &str = include_str!("../../../../scripts/source_length_scan.rs");
const SOURCE_LEDGER_HEADER: &str = "# file_path|owner|split_bead|removal_plan|reason\n# empty fixture ledger\n# row 3\n# row 4\n# row 5\n";
const HOT_LEDGER_HEADER: &str = "# file_path|start_line|owner|split_bead|removal_plan|reason\n# empty fixture ledger\n# row 3\n# row 4\n# row 5\n";
pub(crate) const QUARTERLY_2026_Q2: &str =
    "{\"quarter\":\"2026-Q2\",\"count\":705,\"date\":\"2026-06-18\"}\n";

pub(crate) struct GateOutput {
    pub(crate) code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) elapsed: Duration,
}

pub(crate) fn fixture_repo() -> TestResult<TempDir> {
    let temp = TempDir::new()?;
    run_git(
        temp.path(),
        &["-c", "init.defaultBranch=main", "init", "-q"],
    )?;
    write_gate_scripts(temp.path())?;
    write_ledgers(temp.path())?;
    write_compile_split_sources(temp.path())?;
    Ok(temp)
}

pub(crate) fn write_clean_tree(root: &Path) -> TestResult<()> {
    write_file(
        &root.join("crates/vb_core/Cargo.toml"),
        "[package]\nname = \"vb_core\"\nedition = \"2024\"\n",
    )?;
    write_file(
        &root.join("crates/vb_core/src/lib.rs"),
        "pub fn short_function() -> u8 {\n    let value = 0;\n    value\n}\n",
    )?;
    write_file(
        &root.join("crates/vb_core/src/tests.rs"),
        &hot_function_source("long_but_test_path", 30),
    )?;
    Ok(())
}

pub(crate) fn write_long_function_tree(root: &Path) -> TestResult<()> {
    write_clean_tree(root)?;
    write_file(
        &root.join("crates/vb_core/src/engine.rs"),
        &hot_function_source("over_limit", 30),
    )?;
    write_file(
        &root.join("crates/vb_core/src/tests.rs"),
        &hot_function_source("also_over_limit", 30),
    )?;
    Ok(())
}

pub(crate) fn write_unsafe_long_function_tree(root: &Path) -> TestResult<()> {
    write_clean_tree(root)?;
    write_file(
        &root.join("crates/vb_core/src/engine.rs"),
        &unsafe_hot_function_source("over_limit", 30),
    )?;
    Ok(())
}

pub(crate) fn write_hot_function_tree_with_logical_lines(
    root: &Path,
    lines: usize,
) -> TestResult<()> {
    write_clean_tree(root)?;
    write_file(
        &root.join("crates/vb_core/src/engine.rs"),
        &hot_function_source("boundary", lines),
    )?;
    Ok(())
}

pub(crate) fn write_long_file_tree(root: &Path) -> TestResult<()> {
    write_long_file_tree_with_lines(root, 350)
}

pub(crate) fn write_long_file_tree_with_lines(root: &Path, lines: u16) -> TestResult<()> {
    write_clean_tree(root)?;
    write_file(
        &root.join("crates/vb_core/src/long_file.rs"),
        &long_file_source(lines),
    )?;
    Ok(())
}

pub(crate) fn write_adversarial_hot_tree(root: &Path) -> TestResult<()> {
    write_clean_tree(root)?;
    write_file(
        &root.join("crates/vb_core/src/engine.rs"),
        &adversarial_hot_function_source(),
    )?;
    Ok(())
}

pub(crate) fn write_non_utf8_tree(root: &Path) -> TestResult<()> {
    write_clean_tree(root)?;
    write_bytes(
        &root.join("crates/vb_core/src/engine.rs"),
        &non_utf8_source_bytes(),
    )?;
    Ok(())
}

pub(crate) fn write_split_or_retire_marker_ledgers(
    root: &Path,
    source_markers: usize,
    hot_markers: usize,
) -> TestResult<()> {
    let mut source = SOURCE_LEDGER_HEADER.to_string();
    for marker in 0..source_markers {
        source.push_str(&format!(
            "# split-or-retire-before-release source marker {marker}\n"
        ));
    }
    let mut hot = HOT_LEDGER_HEADER.to_string();
    for marker in 0..hot_markers {
        hot.push_str(&format!(
            "# split-or-retire-before-release hot marker {marker}\n"
        ));
    }
    write_source_ledger(root, &source)?;
    write_file(
        &root.join(".config/hot-function-length-exceptions.txt"),
        &hot,
    )?;
    Ok(())
}

pub(crate) fn write_dedup_source_exception_fixture(
    root: &Path,
    removal_plans: &[&str],
) -> TestResult<()> {
    write_clean_tree(root)?;
    let mut rows = Vec::new();
    for (index, removal_plan) in removal_plans.iter().enumerate() {
        let file = format!("crates/vb_core/src/dedup_exception_{index}.rs");
        write_file(&root.join(&file), &long_file_source(350))?;
        rows.push(format!(
            "{file}|owner|split-bead|{removal_plan}|active exception row"
        ));
    }
    let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    write_source_ledger(root, &source_ledger_text(&row_refs))?;
    Ok(())
}

pub(crate) fn source_ledger_text(rows: &[&str]) -> String {
    let mut text = SOURCE_LEDGER_HEADER.to_string();
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    text
}

pub(crate) fn write_source_ledger(root: &Path, text: &str) -> TestResult<()> {
    write_file(&source_ledger_path(root), text)
}

pub(crate) fn source_ledger_path(root: &Path) -> PathBuf {
    root.join(".config/source-length-exceptions.txt")
}

pub(crate) fn finish_git_fixture(root: &Path) -> TestResult<()> {
    run_git(root, &["add", "."])
}

pub(crate) fn fixture_env(root: &Path) -> Vec<(&'static str, OsString)> {
    let mut envs = fixture_env_without_budget_overrides(root);
    envs.push(("SOURCE_LENGTH_FILE_LIMIT", OsString::from("300")));
    envs.push(("SOURCE_LENGTH_HOT_FUNCTION_LIMIT", OsString::from("25")));
    envs
}

pub(crate) fn fixture_env_without_budget_overrides(root: &Path) -> Vec<(&'static str, OsString)> {
    vec![
        (
            "SOURCE_LENGTH_LEDGER",
            source_ledger_path(root).into_os_string(),
        ),
        (
            "SOURCE_LENGTH_HOT_FUNCTION_LEDGER",
            root.join(".config/hot-function-length-exceptions.txt")
                .into_os_string(),
        ),
        (
            "SOURCE_LENGTH_QUARTERLY_STATE",
            root.join(".config/source-length-quarterly-counts.jsonl")
                .into_os_string(),
        ),
    ]
}

pub(crate) fn quarterly_state_env(path: &Path) -> Vec<(&'static str, OsString)> {
    vec![(
        "SOURCE_LENGTH_QUARTERLY_STATE",
        path.to_path_buf().into_os_string(),
    )]
}

pub(crate) fn run_gate(
    root: &Path,
    envs: &[(&str, OsString)],
    output_dir: &Path,
) -> TestResult<GateOutput> {
    let stdout_path = output_dir.join("gate-stdout.txt");
    let stderr_path = output_dir.join("gate-stderr.txt");
    let stdout_file = File::create(&stdout_path)?;
    let stderr_file = File::create(&stderr_path)?;
    let mut command = Command::new("bash");
    command
        .arg("scripts/check-source-length.sh")
        .current_dir(root);
    for name in [
        "SOURCE_LENGTH_LEDGER",
        "SOURCE_LENGTH_HOT_FUNCTION_LEDGER",
        "SOURCE_LENGTH_QUARTERLY_STATE",
        "SOURCE_LENGTH_FILE_LIMIT",
        "SOURCE_LENGTH_HOT_FUNCTION_LIMIT",
    ] {
        command.env_remove(name);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    let started = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        if started.elapsed() >= Duration::from_secs(60) {
            child.kill()?;
            let _status = child.wait()?;
            return Err(test_error(format!(
                "source-length gate exceeded 60s in {}",
                root.display()
            ))
            .into());
        }
        thread::sleep(Duration::from_millis(50));
    };
    let elapsed = started.elapsed();
    Ok(GateOutput {
        code,
        stdout: fs::read_to_string(stdout_path)?,
        stderr: fs::read_to_string(stderr_path)?,
        elapsed,
    })
}

pub(crate) fn real_pipeline_root() -> TestResult<PathBuf> {
    if let Some(value) = env::var_os("VELVET_BALLISTICS_SOURCE_CHECKOUT") {
        let candidate = PathBuf::from(value);
        if candidate.is_dir() && is_source_work_tree(&candidate)? {
            return Ok(candidate);
        }
    }
    let workspace = workspace_root()?;
    if workspace.is_dir() && is_source_work_tree(&workspace)? {
        return Ok(workspace);
    }
    let default_source = PathBuf::from("/home/lewis/src/velvet-ballistics");
    if default_source.is_dir() && is_source_work_tree(&default_source)? {
        return Ok(default_source);
    }
    Err(
        test_error("no git or jj source checkout available for full source-length pipeline test")
            .into(),
    )
}

pub(crate) fn assert_absent_runtime_failure_text(stderr: &str) {
    assert_eq!(stderr.contains("panic"), false, "stderr:\n{stderr}");
    assert_eq!(stderr.contains("panicked"), false, "stderr:\n{stderr}");
    assert_eq!(stderr.contains("unimplemented"), false, "stderr:\n{stderr}");
    assert_eq!(stderr.contains("unwrap"), false, "stderr:\n{stderr}");
}

pub(crate) fn assert_state_file_is_valid_jsonl(path: &Path) -> TestResult<()> {
    for (index, line) in fs::read_to_string(path)?.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let display_line = index.saturating_add(1);
        let value: Value = serde_json::from_str(line)
            .map_err(|err| test_error(format!("invalid JSONL line {display_line}: {err}")))?;
        assert!(
            value.get("quarter").and_then(Value::as_str).is_some(),
            "line {display_line}: {line}"
        );
        assert!(
            value.get("count").and_then(Value::as_u64).is_some(),
            "line {display_line}: {line}"
        );
        assert!(
            value.get("date").and_then(Value::as_str).is_some(),
            "line {display_line}: {line}"
        );
    }
    Ok(())
}

pub(crate) fn state_file_jsonl_row_count(path: &Path) -> TestResult<usize> {
    Ok(state_file_lines(path)?.len())
}

pub(crate) fn state_file_lines(path: &Path) -> TestResult<Vec<String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect())
}

pub(crate) fn quarterly_state_line(quarter: &str, count: u32, date: &str) -> String {
    format!("{{\"quarter\":\"{quarter}\",\"count\":{count},\"date\":\"{date}\"}}\n")
}

pub(crate) fn current_date() -> TestResult<String> {
    Ok(Utc::now().format("%Y-%m-%d").to_string())
}

pub(crate) fn current_and_previous_quarter_labels() -> TestResult<(String, String)> {
    let now = Utc::now();
    let quarter = now
        .month0()
        .checked_div(3)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| test_error("quarter calculation overflowed"))?;
    let current = format!("{}-Q{}", now.year(), quarter);
    let previous = if quarter == 1 {
        let year = now
            .year()
            .checked_sub(1)
            .ok_or_else(|| test_error("previous quarter year underflowed"))?;
        format!("{year}-Q4")
    } else {
        let previous_quarter = quarter
            .checked_sub(1)
            .ok_or_else(|| test_error("previous quarter underflowed"))?;
        format!("{}-Q{}", now.year(), previous_quarter)
    };
    Ok((current, previous))
}

pub(crate) fn line_index(text: &str, needle: &str) -> TestResult<usize> {
    text.lines()
        .enumerate()
        .find_map(|(idx, line)| (line == needle).then_some(idx))
        .ok_or_else(|| test_error(format!("missing line: {needle}")).into())
}

fn write_gate_scripts(root: &Path) -> TestResult<()> {
    write_file(
        &root.join("scripts/check-source-length.sh"),
        CHECK_SOURCE_LENGTH_SH,
    )?;
    write_file(
        &root.join("scripts/check-source-length.rs"),
        CHECK_SOURCE_LENGTH_RS,
    )?;
    write_file(
        &root.join("scripts/source_length_gate.rs"),
        SOURCE_LENGTH_GATE_RS,
    )?;
    write_file(
        &root.join("scripts/source_length_ledger.rs"),
        SOURCE_LENGTH_LEDGER_RS,
    )?;
    write_file(
        &root.join("scripts/source_length_scan.rs"),
        SOURCE_LENGTH_SCAN_RS,
    )?;
    Ok(())
}

fn write_ledgers(root: &Path) -> TestResult<()> {
    write_source_ledger(root, SOURCE_LEDGER_HEADER)?;
    write_file(
        &root.join(".config/hot-function-length-exceptions.txt"),
        HOT_LEDGER_HEADER,
    )?;
    Ok(())
}

fn write_compile_split_sources(root: &Path) -> TestResult<()> {
    for file in [
        "mod_compile_core.rs",
        "mod_compile_errors.rs",
        "mod_compile_validation.rs",
        "mod_compile_lowering.rs",
    ] {
        write_file(
            &root.join("crates/vb_compile/src").join(file),
            "mod generated_split_fixture;\n",
        )?;
    }
    Ok(())
}

fn write_file(path: &Path, text: &str) -> TestResult<()> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> TestResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn run_git(root: &Path, args: &[&str]) -> TestResult<()> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(test_error(format!(
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    ))
    .into())
}

fn is_git_work_tree(root: &Path) -> TestResult<bool> {
    match Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
    {
        Ok(output) => {
            Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
        }
        Err(_) => Ok(false),
    }
}

fn is_source_work_tree(root: &Path) -> TestResult<bool> {
    Ok(is_git_work_tree(root)? || is_jj_work_tree(root))
}

fn is_jj_work_tree(root: &Path) -> bool {
    match Command::new("jj").arg("root").current_dir(root).output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn workspace_root() -> TestResult<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| test_error("workspace root could not be derived").into())
}

fn test_error(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}
