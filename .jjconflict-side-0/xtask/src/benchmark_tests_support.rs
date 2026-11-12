use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

/// A unique per-test git repository (tempdir-backed), so no two tests share a
/// working directory and no global `cwd`/env mutation is ever required.
pub(crate) struct GitFixture {
    dir: tempfile::TempDir,
}

impl GitFixture {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let fixture = Self {
            dir: tempfile::tempdir().context("create benchmark test temp dir")?,
        };
        fixture.run_git(&["init", "-q"])?;
        Ok(fixture)
    }

    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(crate) fn run_git(&self, args: &[&str]) -> anyhow::Result<String> {
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

    pub(crate) fn write(&self, relative: &str, content: &str) -> anyhow::Result<()> {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent {}", parent.display()))?;
        }
        fs::write(&path, content).with_context(|| format!("write {}", path.display()))
    }

    pub(crate) fn commit_all(&self, message: &str) -> anyhow::Result<()> {
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

    pub(crate) fn head(&self) -> anyhow::Result<String> {
        self.run_git(&["rev-parse", "HEAD"])
    }
}

const MINIMAL_BUDGET: &str = "\
current_milestone: ir-interpreter
generated_maxperf_deferred: true
deferred_modes:
  - generated
  - maxperf
required_evidence_fields:
  - metric
benchmarks:
  transition_set:
    max_regression_percent: 3
";

pub(crate) fn valid_repo() -> anyhow::Result<GitFixture> {
    let fixture = GitFixture::new()?;
    fixture.write("Cargo.toml", "[workspace]\n")?;
    fixture.write("Cargo.lock", "version = 4\n")?;
    fixture.write(
        "rust-toolchain.toml",
        "[toolchain]\nchannel = \"nightly\"\n",
    )?;
    fixture.write("contracts/perf-budget.yaml", MINIMAL_BUDGET)?;
    fixture.write("crates/vb_core/src/lib.rs", "pub fn hello() {}\n")?;
    fixture.write("evidence/benchmark-logs/test.log", "criterion output\n")?;
    fixture.commit_all("initial")?;
    Ok(fixture)
}
