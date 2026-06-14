// SPDX-License-Identifier: MIT
// moon-task-coverage-audit: every master-mandatory tool must have a Moon task.
//
// Master quote: "every mandatory command must be represented as a Moon task
// before release" (velvet-ballistics-MASTER.md §4 "Mandatory Rust Tooling").
//
// This binary parses .moon/tasks/*.yml for top-level task IDs, then for each
// master-mandatory tool/category records whether a corresponding task exists.
// Reports `GAP: <tool>` for any mandatory category missing a task; exits 1.
//
// Advisory tools (cargo-bloat, cargo-insta, flamegraph, hyperfine, valgrind
// family, cargo-semver-checks, cargo-public-api, cargo-machete) are explicitly
// non-blocking under the 2026-05-23 owner waiver and are NOT flagged.
//
// Hand-rolled, no regex crate, no Cargo, no unsafe/unwrap/expect/panic.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MANDATORY_TOOLS: &[(&str, &[&str])] = &[
    ("rustfmt", &["fmt"]),
    ("clippy (source lint)", &["lint-src"]),
    ("cargo test (workspace)", &["test"]),
    ("miri", &["miri"]),
    ("cargo-fuzz smoke", &["fuzz-smoke"]),
    ("cargo-hack (feature powerset)", &["feature-powerset"]),
    ("cargo-mutants smoke", &["mutants-smoke"]),
    ("cargo-llvm-cov (coverage)", &["coverage"]),
    ("cargo-audit", &["supply-chain"]),
    ("cargo-deny", &["supply-chain"]),
    ("cargo-vet", &["supply-chain"]),
    (
        "cargo-geiger (unsafe audit)",
        &["unsafe-audit", "supply-chain"],
    ),
    ("panic surface scan", &["panic-surface"]),
    (
        "ignored fallible results scan",
        &["ignored-fallible-results"],
    ),
    ("nightly feature gate", &["nightly-feature-gate"]),
    ("Kani list", &["kani-list"]),
    (
        "Kani verify",
        &[
            "verify-kani",
            "verify-kani-vb-validate",
            "verify-kani-shard-command-queue-standin-model",
        ],
    ),
    ("Loom smoke", &["loom-run", "loom-list-smoke"]),
    (
        "Flux check",
        &["flux-check-vb-compile", "flux-check-vb-runtime"],
    ),
    ("Verus verify", &["verify-verus", "verify-verus-all"]),
    (
        "TLC verify",
        &[
            "run-tlc-checks",
            "verify-tlc",
            "verify-tlc-idempotency",
            "verify-tlc-workflow",
        ],
    ),
    ("moon ci (orchestration)", &["check", "ci", "quick"]),
    ("criterion / bench build", &["bench-build"]),
    ("maxperf / hardened build", &["maxperf", "hardened-build"]),
    (
        "PGO build",
        &["pgo-instrument-build", "pgo-optimized-build"],
    ),
    (
        "section36-39 coverage audit",
        &["section36-39-coverage-audit"],
    ),
    ("moon task coverage (this)", &["moon-task-coverage-audit"]),
    ("spelling gate", &["check-spelling-gate"]),
    ("test density", &["check-test-density"]),
    ("test determinism", &["test-determinism"]),
    ("test integrity", &["test-integrity"]),
    ("primitive durability doc", &["primitive-durability-doc"]),
    ("doc taint consistency", &["check-doc-taint-consistency"]),
    ("error exhaustiveness", &["check-error-exhaustiveness"]),
    ("dead IR duplicates", &["check-no-dead-ir-duplicates"]),
    ("kani shape vacuity", &["check-kani-shape-vacuity"]),
    (
        "forbidden API scan",
        &["forbidden-scan", "hot-cold-forbidden-apis"],
    ),
    ("hot path scan", &["hotpath-scan"]),
    (
        "source length",
        &["source-length", "source-length-self-test"],
    ),
    ("workspace assertions", &["workspace-assertions"]),
    ("bench registration", &["check-bench-registration"]),
    ("agent CLI contract", &["agent-cli-contract"]),
    ("beads server mode", &["beads-server-mode"]),
    ("blocker closure evidence", &["blocker-closure-evidence"]),
    ("stepstate matrix", &["check-stepstate-matrix"]),
    (
        "verify proof (deep)",
        &["verify-deep", "verify-all", "verify-proof"],
    ),
    ("verify fast", &["verify-fast"]),
    ("verify standard", &["verify-standard"]),
    ("no-legacy-primitives", &["verify-no-legacy-primitives"]),
    ("verify Lean", &["verify-lean"]),
    ("contracts task", &["contracts"]),
    (
        "generate queue state verus helpers",
        &["generate-queue-state-verus-helpers"],
    ),
    ("guard zero tests", &["guard-zero-tests"]),
    ("doc", &["doc", "doc-test"]),
    ("sanitizer address check", &["sanitizer-address-check"]),
    ("bench alloc evidence", &["bench-alloc-evidence"]),
    ("bench instruction counts", &["bench-instruction-counts"]),
    ("build section39 latency", &["build-section39-latency"]),
    ("benchmark proof", &["benchmark-proof"]),
    (
        "benchmark regression policy",
        &["benchmark-regression-policy"],
    ),
    ("fuzz minimization", &["fuzz-minimization"]),
];

fn collect_defined_tasks(moon_dir: &Path) -> Result<BTreeSet<String>, String> {
    let tasks_dir = moon_dir.join("tasks");
    if !tasks_dir.is_dir() {
        return Err(format!("missing directory: {}", tasks_dir.display()));
    }
    let mut defined = BTreeSet::new();
    let entries =
        fs::read_dir(&tasks_dir).map_err(|e| format!("read_dir({}): {e}", tasks_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yml") {
            continue;
        }
        let content =
            fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        // Hand-rolled: scan for lines matching `  <name>:` at exactly 2-space indent (top-level task).
        for line in content.lines() {
            if !line.starts_with("  ") || line.starts_with("    ") {
                continue;
            }
            // Strip "  " prefix and trailing ":"
            let trimmed = &line[2..];
            let Some(colon_pos) = trimmed.find(':') else {
                continue;
            };
            let name = &trimmed[..colon_pos];
            // Skip yaml keys that are clearly not task names: empty, contains space, contains non-id chars
            if name.is_empty() || name.contains(' ') || name.contains('\t') {
                continue;
            }
            if !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                continue;
            }
            defined.insert(name.to_owned());
        }
    }
    Ok(defined)
}

fn run_audit() -> Result<u8, String> {
    let moon_dir = PathBuf::from(".moon");
    let defined = collect_defined_tasks(&moon_dir)?;
    println!(
        "moon-task-coverage-audit: parsed {} tasks from {} files",
        defined.len(),
        count_yml_files(&moon_dir)?
    );
    println!("defined: {:?}", defined);
    println!();
    let mut gaps: Vec<String> = Vec::new();
    for (tool, candidates) in MANDATORY_TOOLS {
        if !candidates.iter().any(|c| defined.contains(*c)) {
            gaps.push(format!("GAP: {tool}  expected_one_of={candidates:?}"));
        }
    }
    if !gaps.is_empty() {
        println!("=== GAPS ===");
        for g in &gaps {
            println!("{g}");
        }
        println!("FAIL: {} mandatory tools have no Moon task", gaps.len());
        return Ok(1);
    }
    println!(
        "PASS: all {} mandatory tool categories have at least one Moon task",
        MANDATORY_TOOLS.len()
    );
    Ok(0)
}

fn count_yml_files(moon_dir: &Path) -> Result<usize, String> {
    let tasks_dir = moon_dir.join("tasks");
    let mut count = 0;
    for entry in fs::read_dir(&tasks_dir).map_err(|e| format!("read_dir: {e}"))? {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yml") {
            count += 1;
        }
    }
    Ok(count)
}

fn run_self_test() -> Result<u8, String> {
    // Self-test: verify the parser extracts task IDs from a known fixture.
    // Does NOT run the full audit (that happens in the second invocation
    // against the real `.moon/tasks/` of the repo).
    let temp = std::env::temp_dir().join(format!("moon-coverage-selftest-{}", std::process::id()));
    fs::create_dir_all(&temp).map_err(|e| format!("mkdir: {e}"))?;
    let moon_tasks_dir = temp.join(".moon").join("tasks");
    fs::create_dir_all(&moon_tasks_dir).map_err(|e| format!("mkdir: {e}"))?;
    fs::write(
        moon_tasks_dir.join("all.yml"),
        "tasks:\n  fmt:\n    command: 'cargo fmt'\n  lint-src:\n    command: 'cargo clippy'\n  coverage:\n    command: 'cargo llvm-cov'\n",
    ).map_err(|e| format!("write: {e}"))?;
    fs::write(
        moon_tasks_dir.join("aux.yml"),
        "tasks:\n  miri:\n    command: 'cargo miri'\n",
    )
    .map_err(|e| format!("write: {e}"))?;

    let defined = collect_defined_tasks(&temp.join(".moon"))?;
    let expected: BTreeSet<String> = ["fmt", "lint-src", "coverage", "miri"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    if defined != expected {
        println!("FixtureFail: parsed {:?}, expected {:?}", defined, expected);
        let _ = fs::remove_dir_all(&temp);
        return Ok(1);
    }
    let _ = fs::remove_dir_all(&temp);
    println!("FixturePass: moon-task-coverage scanner");
    println!("Self-test PASSED");
    Ok(0)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let result = if args.len() > 1 && args[1] == "--self-test" {
        run_self_test()
    } else {
        run_audit()
    };
    match result {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
