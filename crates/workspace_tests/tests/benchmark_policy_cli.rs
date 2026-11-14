#![forbid(unsafe_code)]

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

const BUDGET: &str = include_str!("../../../contracts/perf-budget.yaml");
const METRICS: [(&str, u64); 3] = [
    ("bench_engine_step_once_save_const_single_transition", 3),
    ("ipc_frame_decode", 5),
    ("engine_run_until_blocked_budget_10_small_workflow", 3),
];

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Fixture {
    directory: TempDir,
    binary: PathBuf,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        git(root, &["init", "--quiet"])?;
        write(root, "Cargo.toml", "[workspace]\nmembers = []\n")?;
        write(root, "Cargo.lock", "version = 4\n")?;
        write(
            root,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"nightly-2026-04-28\"\n",
        )?;
        write(root, "contracts/perf-budget.yaml", BUDGET)?;
        write(root, "crates/vb_core/src/lib.rs", "pub fn stable() {}\n")?;
        write(root, "evidence/raw.log", "benchmark run\n")?;
        commit(root)?;
        let head = git(root, &["rev-parse", "HEAD"])?;
        write_evidence(root, &head)?;
        Ok(Self {
            directory,
            binary: PathBuf::from(env!("CARGO_BIN_EXE_benchmark-policy")),
        })
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn run(&self) -> TestResult<Output> {
        Ok(Command::new(&self.binary)
            .args([
                "--budget",
                "contracts/perf-budget.yaml",
                "--evidence",
                "evidence/claims.jsonl",
            ])
            .current_dir(self.root())
            .output()?)
    }

    fn run_with_inline_args(&self) -> TestResult<Output> {
        Ok(Command::new(&self.binary)
            .args([
                "--budget=contracts/perf-budget.yaml",
                "--evidence=evidence/claims.jsonl",
            ])
            .current_dir(self.root())
            .output()?)
    }
}

#[test]
fn equal_form_arguments_pass_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run_with_inline_args()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("BenchmarkRegressionPolicyOk"));
    Ok(())
}

fn write(root: &Path, relative: &str, contents: &str) -> TestResult {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> TestResult<String> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn commit(root: &Path) -> TestResult {
    git(root, &["add", "-A"])?;
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=benchmark-test",
            "-c",
            "user.email=benchmark-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

fn write_evidence(root: &Path, commit: &str) -> TestResult {
    let lines = METRICS
        .iter()
        .map(|(metric, threshold)| {
            serde_json::json!({
                "metric": metric,
                "claim": "stable benchmark result",
                "baseline_value": 100,
                "result_value": 100,
                "unit": "ns",
                "threshold_percent": threshold,
                "raw_log": "evidence/raw.log",
                "command": "cargo bench",
                "commit": commit,
                "rustc_version": "rustc 1.90.0-nightly",
                "nightly_date": "2026-04-28",
                "cpu_model": "fixture-cpu",
                "cpu_governor": "performance",
                "kernel_version": "fixture-kernel",
                "build_profile": "release",
                "rustflags": "-C opt-level=3",
                "benchmark_tool_version": "criterion 0.5",
                "sample_count_or_instruction_count": 10,
                "input_fixture_digest": "sha256:fixture",
                "durability_profile": "journaled",
                "mode": "ir-interpreter",
                "p50_latency": 100,
                "p95_latency": 100,
                "p99_latency": 100,
                "instruction_count": 10,
                "allocation_count": 0,
                "bytes_allocated": 0,
                "fjall_write_latency": 0,
                "direct_api_latency": 0,
                "ipc_latency": 0
            })
        })
        .map(|value| serde_json::to_string(&value))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    write(root, "evidence/claims.jsonl", &format!("{lines}\n"))
}

#[test]
fn clean_same_head_evidence_passes_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("BenchmarkRegressionPolicyOk"));
    Ok(())
}

#[test]
fn evidence_only_descendant_commit_passes_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    let head = git(fixture.root(), &["rev-parse", "HEAD"])?;
    write_evidence(fixture.root(), &head)?;
    write(fixture.root(), "evidence/notes.txt", "report-only change\n")?;
    commit(fixture.root())?;
    let output = fixture.run()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn dirty_source_at_same_head_is_rejected_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    write(
        fixture.root(),
        "crates/vb_core/src/lib.rs",
        "pub fn changed() {}\n",
    )?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("stale"));
    Ok(())
}

#[test]
fn staged_source_at_same_head_is_rejected_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    write(
        fixture.root(),
        "crates/vb_core/src/lib.rs",
        "pub fn staged() {}\n",
    )?;
    git(fixture.root(), &["add", "crates/vb_core/src/lib.rs"])?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("stale"));
    Ok(())
}

#[test]
fn missing_evidence_commit_is_rejected_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    write_evidence(fixture.root(), &"a".repeat(40))?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a valid commit"));
    Ok(())
}

#[test]
fn non_commit_evidence_object_is_rejected_through_binary() -> TestResult {
    let fixture = Fixture::new()?;
    let blob = git(
        fixture.root(),
        &["rev-parse", "HEAD:crates/vb_core/src/lib.rs"],
    )?;
    write_evidence(fixture.root(), &blob)?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a valid commit"));
    Ok(())
}
