use anyhow::Context;

use super::{resolve_commit, validate_evidence_freshness};
use crate::benchmark_tests_support::{GitFixture, valid_repo};

fn stale_rejection(fixture: &GitFixture, measured: &str, head: &str) -> anyhow::Result<String> {
    match validate_evidence_freshness(fixture.path(), measured, head, 1) {
        Ok(()) => anyhow::bail!("expected a staleness rejection"),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn clean_same_head_is_fresh() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    validate_evidence_freshness(fixture.path(), &head, &head, 1)
}

#[test]
fn evidence_only_descendant_is_fresh() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let measured = fixture.head()?;
    fixture.write("docs/benchmark-notes.md", "evidence-only change\n")?;
    fixture.commit_all("docs only")?;
    let head = fixture.head()?;
    validate_evidence_freshness(fixture.path(), &measured, &head, 1)
}

#[test]
fn dirty_working_tree_same_head_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 1; }\n",
    )?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("stale"), "{message}");
    assert!(message.contains("crates/vb_core/src/lib.rs"), "{message}");
    Ok(())
}

#[test]
fn staged_change_same_head_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 2; }\n",
    )?;
    fixture.run_git(&["add", "crates/vb_core/src/lib.rs"])?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("stale"), "{message}");
    assert!(message.contains("crates/vb_core/src/lib.rs"), "{message}");
    Ok(())
}

#[test]
fn committed_source_change_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let measured = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 3; }\n",
    )?;
    fixture.commit_all("changed source")?;
    let head = fixture.head()?;
    let message = stale_rejection(&fixture, &measured, &head)?;
    assert!(message.contains("stale"), "{message}");
    Ok(())
}

#[test]
fn touched_then_reverted_source_is_fresh() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 9; }\n",
    )?;
    fixture.write("crates/vb_core/src/lib.rs", "pub fn hello() {}\n")?;
    validate_evidence_freshness(fixture.path(), &head, &head, 1)
}

#[test]
fn unrelated_dirty_file_is_fresh() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write("docs/notes.md", "unrelated change\n")?;
    validate_evidence_freshness(fixture.path(), &head, &head, 1)
}

#[test]
fn untracked_relevant_rust_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/new_module.rs",
        "pub fn new_thing() {}\n",
    )?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("untracked"), "{message}");
    assert!(message.contains("new_module.rs"), "{message}");
    Ok(())
}

#[test]
fn untracked_relevant_config_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "crates/vb_core/Cargo.toml",
        "[package]\nname = \"vb_core\"\n",
    )?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("untracked"), "{message}");
    assert!(message.contains("Cargo.toml"), "{message}");
    Ok(())
}

#[test]
fn gitignored_relevant_config_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    fixture.write(".gitignore", ".cargo/config.local.toml\n")?;
    fixture.commit_all("ignore relevant config")?;
    let head = fixture.head()?;
    fixture.write(".cargo/config.local.toml", "[env]\nFOO=1\n")?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("untracked"), "{message}");
    assert!(message.contains(".cargo/config.local.toml"), "{message}");
    Ok(())
}

#[test]
fn ignored_build_output_not_flagged() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write("crates/vb_core/target/debug/foo.o", "artifact\n")?;
    validate_evidence_freshness(fixture.path(), &head, &head, 1)
}

#[test]
fn untracked_irrelevant_file_is_fresh() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write("docs/untracked-notes.md", "notes\n")?;
    validate_evidence_freshness(fixture.path(), &head, &head, 1)
}

#[test]
fn cargo_lock_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write("Cargo.lock", "version = 5\n")?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("Cargo.lock"), "{message}");
    Ok(())
}

#[test]
fn toolchain_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write("rust-toolchain.toml", "[toolchain]\nchannel = \"beta\"\n")?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("rust-toolchain.toml"), "{message}");
    Ok(())
}

#[test]
fn bench_fixture_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    fixture.write("fixtures/bench/input.txt", "input v1\n")?;
    fixture.commit_all("add bench fixture")?;
    let measured = fixture.head()?;
    fixture.write("fixtures/bench/input.txt", "input v2\n")?;
    let message = stale_rejection(&fixture, &measured, &measured)?;
    assert!(message.contains("fixtures/bench/input.txt"), "{message}");
    Ok(())
}

#[test]
fn perf_budget_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    fixture.write(
        "contracts/perf-budget.yaml",
        "current_milestone: ir-interpreter\ngenerated_maxperf_deferred: true\ndeferred_modes:\n  - generated\n  - maxperf\nbenchmarks:\n  transition_set:\n    max_regression_percent: 9\n",
    )?;
    let message = stale_rejection(&fixture, &head, &head)?;
    assert!(message.contains("contracts/perf-budget.yaml"), "{message}");
    Ok(())
}

#[test]
fn xtask_policy_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    fixture.write("xtask/src/benchmark_policy.rs", "fn main() {}\n")?;
    fixture.commit_all("add xtask policy")?;
    let measured = fixture.head()?;
    fixture.write(
        "xtask/src/benchmark_policy.rs",
        "fn main() { let x = 1; }\n",
    )?;
    let message = stale_rejection(&fixture, &measured, &measured)?;
    assert!(
        message.contains("xtask/src/benchmark_policy.rs"),
        "{message}"
    );
    Ok(())
}

#[test]
fn script_mutation_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    fixture.write("scripts/bench.sh", "#!/bin/sh\ncargo bench\n")?;
    fixture.commit_all("add bench script")?;
    let measured = fixture.head()?;
    fixture.write(
        "scripts/bench.sh",
        "#!/bin/sh\ncargo bench --all-features\n",
    )?;
    let message = stale_rejection(&fixture, &measured, &measured)?;
    assert!(message.contains("scripts/bench.sh"), "{message}");
    Ok(())
}

#[test]
fn non_ancestor_commit_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let first = fixture.head()?;
    fixture.write(
        "crates/vb_core/src/lib.rs",
        "pub fn hello() { let x = 4; }\n",
    )?;
    fixture.commit_all("second commit")?;
    let second = fixture.head()?;
    let message = stale_rejection(&fixture, &second, &first)?;
    assert!(message.contains("not an ancestor"), "{message}");
    Ok(())
}

#[test]
fn missing_commit_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    let missing = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let result = validate_evidence_freshness(fixture.path(), missing, &head, 1);
    let error = match result {
        Ok(()) => anyhow::bail!("missing commit must be rejected"),
        Err(e) => e,
    };
    assert!(
        error.to_string().contains("not a valid commit"),
        "{}",
        error
    );
    Ok(())
}

#[test]
fn non_commit_blob_rejected() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let blob = fixture.run_git(&["rev-parse", "HEAD:crates/vb_core/src/lib.rs"])?;
    let result = resolve_commit(fixture.path(), &blob, 1);
    let error = match result {
        Ok(s) => anyhow::bail!("blob object must not resolve as a commit, got: {}", s),
        Err(e) => e,
    };
    assert!(
        error.to_string().contains("not a valid commit"),
        "{}",
        error
    );
    Ok(())
}

#[test]
fn resolve_commit_full_sha() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    let resolved = resolve_commit(fixture.path(), &head, 1)?;
    assert_eq!(resolved, head);
    Ok(())
}

#[test]
fn resolve_commit_unique_prefix() -> anyhow::Result<()> {
    let fixture = valid_repo()?;
    let head = fixture.head()?;
    let prefix = head.get(..12).context("head SHA shorter than 12 chars")?;
    let resolved = resolve_commit(fixture.path(), prefix, 1)?;
    assert_eq!(resolved, head);
    Ok(())
}
