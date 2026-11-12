use std::path::Path;
use std::process::{Command, Output};

use anyhow::{Context, bail};

/// Explicit conservative set of performance-relevant paths.
///
/// A benchmark measurement is only sound when every byte that could influence
/// the measured program is identical between the measured commit and the actual
/// working tree. This list is intentionally conservative: any tracked or
/// untracked change under one of these roots invalidates the evidence.
///
/// Excluded are only clearly-unrelated roots: `evidence/`, `docs/`, `.beads/`,
/// reports, specs, schemas, design, reference, fuzz, and verification artifacts.
const PERF_RELEVANT_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
    ".cargo/",
    "crates/",
    "xtask/",
    "contracts/perf-budget.yaml",
    "fixtures/",
    "scripts/",
    ".moon/",
];

/// Build-output / VCS / cache directories excluded from the untracked-input
/// scan. Everything else under a relevant root is treated as an input, even
/// when `.gitignore` hides it, so an ignored `.cargo/config` or ignored
/// `include!`d source still invalidates the evidence.
const UNTRACKED_EXCLUDES: &[&str] = &["target", ".git", ".jj", ".moon/cache"];

/// Validates that `evidence_commit` resolves to a commit that is an ancestor of
/// `current_commit` and whose perf-relevant inputs are byte-identical to the
/// actual working tree (including staged changes) with no untracked relevant
/// inputs.
///
/// This always compares against the working tree — never only commit-to-commit —
/// so a dirty working tree or staged change is rejected even when the evidence
/// commit equals current HEAD.
pub(crate) fn validate_evidence_freshness(
    root: &Path,
    evidence_commit: &str,
    current_commit: &str,
    line_number: usize,
) -> anyhow::Result<()> {
    // Resolve to the actual commit object (peels tags, rejects blob/tree/missing).
    let resolved = resolve_commit(root, evidence_commit, line_number)?;

    if !is_ancestor(root, &resolved, current_commit)? {
        bail!(
            "benchmark evidence line {line_number} commit {evidence_commit} is not an ancestor of current HEAD {current_commit}"
        );
    }

    let changed = changed_paths_vs_working_tree(root, &resolved)?;
    let relevant_changed = changed
        .into_iter()
        .filter(|path| is_perf_relevant_path(path))
        .collect::<Vec<_>>();
    if !relevant_changed.is_empty() {
        bail!(
            "benchmark evidence line {line_number} is stale: performance-relevant files differ from measured commit {evidence_commit}: {}",
            relevant_changed.join(", ")
        );
    }

    let untracked = find_untracked_relevant_files(root)?;
    if !untracked.is_empty() {
        bail!(
            "benchmark evidence line {line_number} is stale: untracked performance-relevant files exist: {}",
            untracked.join(", ")
        );
    }

    Ok(())
}

/// Resolves a commit spec to a full 40-char SHA via `<commit>^{commit}`.
///
/// The `^{commit}` peel guarantees the object is a commit: tags pointing at a
/// blob/tree, raw blob/tree SHAs, missing objects, and ambiguous short prefixes
/// all fail the `git rev-parse --verify` invocation.
pub(crate) fn resolve_commit(
    root: &Path,
    commit: &str,
    line_number: usize,
) -> anyhow::Result<String> {
    let spec = format!("{commit}^{{commit}}");
    let output = run_git(root, &["rev-parse", "--verify", &spec])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "benchmark evidence line {line_number} commit {commit} is not a valid commit: {}",
            stderr.trim()
        );
    }

    let sha = String::from_utf8(output.stdout)
        .with_context(|| format!("decode git rev-parse output for commit {commit}"))?;
    let sha = sha.trim();
    if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!(
            "benchmark evidence line {line_number} git rev-parse returned unexpected value for commit {commit}: {sha}"
        );
    }
    Ok(sha.to_owned())
}

fn is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> anyhow::Result<bool> {
    let output = run_git(root, &["merge-base", "--is-ancestor", ancestor, descendant])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "git merge-base --is-ancestor {ancestor} {descendant} failed: {}",
                stderr.trim()
            );
        }
    }
}

/// Returns perf-relevant paths that differ between the measured commit's tree
/// and the actual working tree (staged and unstaged).
///
/// `git diff <commit>` compares the commit against the working tree, so it
/// captures both staged and unstaged modifications relative to that commit.
/// `--no-renames` disables rename detection (a move-out of a relevant path must
/// still surface as a delete), and `-z` emits NUL-separated raw paths so no
/// path component is ever lossy-decoded.
fn changed_paths_vs_working_tree(
    root: &Path,
    measured_commit: &str,
) -> anyhow::Result<Vec<String>> {
    let output = run_git(
        root,
        &["diff", "--no-renames", "-z", "--name-only", measured_commit],
    )?;
    match output.status.code() {
        Some(0) | Some(1) => {}
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "git diff against working tree for {measured_commit} failed: {}",
                stderr.trim()
            );
        }
    }

    let stdout = String::from_utf8(output.stdout).with_context(|| {
        format!("decode git diff paths for {measured_commit} vs working tree (path not UTF-8)")
    })?;
    Ok(stdout
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

fn is_perf_relevant_path(path: &str) -> bool {
    PERF_RELEVANT_PATHS.iter().any(|pattern| {
        let pattern = *pattern;
        if pattern.ends_with('/') {
            path.starts_with(pattern)
        } else {
            path == pattern
        }
    })
}

/// Enumerates untracked files under perf-relevant roots, including files hidden
/// by `.gitignore`.
///
/// `--exclude-standard` is intentionally NOT used: an ignored `.cargo/config` or
/// ignored `include!`d Rust source still changes the measured program, so it
/// must invalidate the evidence. Only build output and VCS/cache directories are
/// excluded, via explicit `--exclude` patterns.
fn find_untracked_relevant_files(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut args: Vec<&str> = vec!["ls-files", "--others", "-z"];
    for exclude in UNTRACKED_EXCLUDES {
        args.push("--exclude");
        args.push(exclude);
    }
    args.push("--");
    for pattern in PERF_RELEVANT_PATHS {
        args.push(pattern.trim_end_matches('/'));
    }

    let output = run_git(root, &args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git ls-files untracked files failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| "decode git ls-files untracked paths (path not UTF-8)")?;
    Ok(stdout
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

pub(crate) fn current_head_commit(root: &Path) -> anyhow::Result<String> {
    let output = run_git(root, &["rev-parse", "HEAD"])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-parse HEAD failed: {}", stderr.trim());
    }
    let stdout = String::from_utf8(output.stdout).context("decode git rev-parse HEAD output")?;
    let commit = stdout.trim();
    validate_commit_format(commit, "current HEAD")?;
    Ok(commit.to_owned())
}

pub(crate) fn validate_commit_format(commit: &str, label: &str) -> anyhow::Result<()> {
    let len = commit.len();
    if !(7..=40).contains(&len) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} must be 7-40 ASCII hex chars");
    }
    Ok(())
}

fn run_git(root: &Path, args: &[&str]) -> anyhow::Result<Output> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("run git {}", args.join(" ")))
}

#[cfg(test)]
#[path = "benchmark_freshness_tests.rs"]
mod tests;
