use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, bail};

const VALID_BUDGET: &str = r#"
current_milestone: ir-interpreter
generated_maxperf_deferred: true
deferred_modes:
  - generated
  - maxperf
required_evidence_fields:
  - metric
  - claim
  - baseline_value
  - result_value
  - unit
  - threshold_percent
  - raw_log
  - command
  - commit
  - rustc_version
  - nightly_date
  - cpu_model
  - cpu_governor
  - kernel_version
  - build_profile
  - rustflags
  - benchmark_tool_version
  - sample_count_or_instruction_count
  - input_fixture_digest
  - durability_profile
  - mode
  - p50_latency
  - p95_latency
  - p99_latency
  - instruction_count
  - allocation_count
  - bytes_allocated
  - fjall_write_latency
  - direct_api_latency
  - ipc_latency
benchmarks:
  transition_set:
    max_regression_percent: 3
"#;

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn valid() -> anyhow::Result<Self> {
        let fixture = Self {
            dir: tempfile::tempdir().context("create CLI test temp dir")?,
        };
        fixture.run_git(&["init", "-q"])?;
        fixture.write("Cargo.toml", "[workspace]\n")?;
        fixture.write("Cargo.lock", "version = 4\n")?;
        fixture.write(
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"nightly\"\n",
        )?;
        fixture.write("contracts/perf-budget.yaml", VALID_BUDGET)?;
        fixture.write("crates/vb_core/src/lib.rs", "pub fn hello() {}\n")?;
        fixture.write("evidence/benchmark-logs/test.log", "criterion output\n")?;
        fixture.commit_all("initial")?;
        Ok(fixture)
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn run_git(&self, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new("/usr/bin/git")
            .args(args)
            .current_dir(self.path())
            .output()
            .with_context(|| format!("run git {}", args.join(" ")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git {} failed: {}", args.join(" "), stderr.trim());
        }
        String::from_utf8(output.stdout)
            .map(|stdout| stdout.trim().to_owned())
            .context("decode git output as UTF-8")
    }

    fn write(&self, relative: &str, content: &str) -> anyhow::Result<()> {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent {}", parent.display()))?;
        }
        fs::write(&path, content).with_context(|| format!("write {}", path.display()))
    }

    fn commit_all(&self, message: &str) -> anyhow::Result<()> {
        self.run_git(&["add", "-A"])?;
        self.run_git(&[
            "-c",
            "user.name=benchmark-test",
            "-c",
            "user.email=benchmark-test@example.com",
            "commit",
            "-m",
            message,
            "--quiet",
        ])?;
        Ok(())
    }

    fn head(&self) -> anyhow::Result<String> {
        self.run_git(&["rev-parse", "HEAD"])
    }

    fn write_evidence(&self, commit: &str) -> anyhow::Result<()> {
        let line = serde_json::json!({
            "metric": "transition_set",
            "claim": "baseline snapshot at measured commit",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": commit,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });
        self.write(
            "evidence/benchmark-evidence.jsonl",
            &format!("{}\n", serde_json::to_string(&line)?),
        )
    }

    fn run_policy(&self) -> anyhow::Result<Output> {
        let bin = xtask_binary()?;
        Command::new(bin)
            .args(["benchmark-regression-policy"])
            .args(["--budget", "contracts/perf-budget.yaml"])
            .args(["--evidence", "evidence/benchmark-evidence.jsonl"])
            .current_dir(self.path())
            .output()
            .context("run xtask benchmark-regression-policy")
    }
}

fn xtask_binary() -> anyhow::Result<PathBuf> {
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    if bin.exists() {
        Ok(bin)
    } else {
        bail!("xtask binary not built: {}", bin.display())
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn benchmark_policy_clean_same_head_passes_end_to_end() -> anyhow::Result<()> {
    let fixture = Fixture::valid()?;
    let head = fixture.head()?;
    fixture.write_evidence(&head)?;

    let output = fixture.run_policy()?;
    assert!(
        output.status.success(),
        "clean same-head policy should pass; stderr: {}",
        stderr_of(&output)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BenchmarkRegressionPolicyOk"), "{stdout}");
    Ok(())
}

#[test]
fn benchmark_policy_evidence_only_descendant_passes_end_to_end() -> anyhow::Result<()> {
    let fixture = Fixture::valid()?;
    let measured = fixture.head()?;
    fixture.write("docs/benchmark-notes.md", "evidence-only change\n")?;
    fixture.commit_all("docs only")?;
    fixture.write_evidence(&measured)?;

    let output = fixture.run_policy()?;
    assert!(
        output.status.success(),
        "evidence-only descendant should pass; stderr: {}",
        stderr_of(&output)
    );
    Ok(())
}

#[test]
fn benchmark_policy_dirty_source_same_head_rejected_end_to_end() -> anyhow::Result<()> {
    let fixture = Fixture::valid()?;
    let head = fixture.head()?;
    fixture.write_evidence(&head)?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 1; }\n",
    )?;

    let output = fixture.run_policy()?;
    assert!(!output.status.success(), "dirty source must be rejected");
    assert!(
        stderr_of(&output).contains("stale"),
        "stderr should name staleness: {}",
        stderr_of(&output)
    );
    Ok(())
}

#[test]
fn benchmark_policy_staged_source_rejected_end_to_end() -> anyhow::Result<()> {
    let fixture = Fixture::valid()?;
    let head = fixture.head()?;
    fixture.write_evidence(&head)?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 2; }\n",
    )?;
    fixture.run_git(&["add", "crates/vb_core/src/lib.rs"])?;

    let output = fixture.run_policy()?;
    assert!(!output.status.success(), "staged source must be rejected");
    assert!(
        stderr_of(&output).contains("stale"),
        "stderr should name staleness: {}",
        stderr_of(&output)
    );
    Ok(())
}

#[test]
fn benchmark_policy_missing_commit_rejected_end_to_end() -> anyhow::Result<()> {
    let fixture = Fixture::valid()?;
    fixture.write_evidence("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?;

    let output = fixture.run_policy()?;
    assert!(!output.status.success(), "missing commit must be rejected");
    assert!(
        stderr_of(&output).contains("not a valid commit"),
        "stderr should name the invalid commit: {}",
        stderr_of(&output)
    );
    Ok(())
}
