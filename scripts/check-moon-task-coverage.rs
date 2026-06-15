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
use std::path::Path;
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
    // Model-only stand-in smoke lanes are intentionally excluded here so they
    // cannot satisfy production-bound Kani verification accounting. Each
    // production-bound Kani lane is mandatory on its own.
    ("Kani verify (production-bound, vb_core)", &["verify-kani"]),
    (
        "Kani verify (production-bound, vb_validate)",
        &["verify-kani-vb-validate"],
    ),
    (
        "Kani verify (production-bound, vb_compile)",
        &["verify-kani-vb-compile"],
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

const PRODUCTION_BOUND_KANI_TOOLS: &[(&str, &[&str])] = &[
    ("Kani verify (production-bound, vb_core)", &["verify-kani"]),
    (
        "Kani verify (production-bound, vb_validate)",
        &["verify-kani-vb-validate"],
    ),
    (
        "Kani verify (production-bound, vb_compile)",
        &["verify-kani-vb-compile"],
    ),
];

const MODEL_ONLY_KANI_TASK: &str = "kani-model-smoke-shard-command-queue-standin";

struct AuditSummary {
    defined: BTreeSet<String>,
    yml_file_count: usize,
    gaps: Vec<String>,
}

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
        let mut active_top_level_key: Option<String> = None;
        for line in content.lines() {
            if let Some(top_level_key) = top_level_key_name(line) {
                active_top_level_key = Some(top_level_key.to_owned());
                continue;
            }
            if active_top_level_key.as_deref() != Some("tasks") {
                continue;
            }
            let Some(name) = task_name(line) else {
                continue;
            };
            defined.insert(name.to_owned());
        }
    }
    Ok(defined)
}

fn top_level_key_name(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let colon_pos = line.find(':')?;
    let name = &line[..colon_pos];
    if name.is_empty() || name.contains(' ') || name.contains('\t') {
        return None;
    }
    Some(name)
}

fn task_name(line: &str) -> Option<&str> {
    let trimmed = line.strip_prefix("  ")?;
    if trimmed.starts_with("  ") {
        return None;
    }
    let colon_pos = trimmed.find(':')?;
    let name = &trimmed[..colon_pos];
    if name.is_empty() || name.contains(' ') || name.contains('\t') {
        return None;
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some(name)
}

fn collect_gaps_for_tools(
    defined: &BTreeSet<String>,
    required_tools: &[(&str, &[&str])],
) -> Vec<String> {
    let mut gaps: Vec<String> = Vec::new();
    for (tool, candidates) in required_tools {
        if !candidates
            .iter()
            .any(|candidate| defined.contains(*candidate))
        {
            gaps.push(format!("GAP: {tool}  expected_one_of={candidates:?}"));
        }
    }
    gaps
}

fn collect_gaps(defined: &BTreeSet<String>) -> Vec<String> {
    collect_gaps_for_tools(defined, MANDATORY_TOOLS)
}

fn audit_moon_dir(moon_dir: &Path) -> Result<AuditSummary, String> {
    let defined = collect_defined_tasks(moon_dir)?;
    let yml_file_count = count_yml_files(moon_dir)?;
    let gaps = collect_gaps(&defined);
    Ok(AuditSummary {
        defined,
        yml_file_count,
        gaps,
    })
}

fn print_audit_summary(summary: &AuditSummary) {
    println!(
        "moon-task-coverage-audit: parsed {} tasks from {} files",
        summary.defined.len(),
        summary.yml_file_count
    );
    println!("defined: {:?}", summary.defined);
    println!();
    if !summary.gaps.is_empty() {
        println!("=== GAPS ===");
        for g in &summary.gaps {
            println!("{g}");
        }
        println!(
            "FAIL: {} mandatory tools have no Moon task",
            summary.gaps.len()
        );
    } else {
        println!(
            "PASS: all {} mandatory tool categories have at least one Moon task",
            MANDATORY_TOOLS.len()
        );
    }
}

fn run_audit() -> Result<u8, String> {
    let summary = audit_moon_dir(Path::new(".moon"))?;
    print_audit_summary(&summary);
    if !summary.gaps.is_empty() {
        Ok(1)
    } else {
        Ok(0)
    }
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

fn write_fixture_tasks(moon_tasks_dir: &Path, task_names: &BTreeSet<String>) -> Result<(), String> {
    let mut content = String::from(
        "fileGroups:\n  should-not-count:\n    - 'crates/demo/src/**/*'\notherTopLevel:\n  also-not-a-task:\n    command: 'ignore me'\ntasks:\n",
    );
    for task_name in task_names {
        content.push_str("  ");
        content.push_str(task_name);
        content.push_str(":\n    command: 'true'\n");
    }
    fs::write(moon_tasks_dir.join("all.yml"), content).map_err(|e| format!("write all.yml: {e}"))
}

fn required_task_names_for_tools(
    required_tools: &[(&str, &[&str])],
) -> Result<BTreeSet<String>, String> {
    required_task_names_with_alias_overrides(required_tools, &[])
}

fn required_task_names_with_alias_overrides(
    required_tools: &[(&str, &[&str])],
    alias_overrides: &[(&str, usize)],
) -> Result<BTreeSet<String>, String> {
    let mut task_names = BTreeSet::new();
    for (tool, candidates) in required_tools {
        let mut candidate_index = 0_usize;
        for (override_tool, override_index) in alias_overrides {
            if tool == override_tool {
                candidate_index = *override_index;
                break;
            }
        }
        let Some(candidate) = candidates.get(candidate_index) else {
            return Err(format!(
                "tool {tool} is missing candidate task ID at index {candidate_index}"
            ));
        };
        task_names.insert((*candidate).to_string());
    }
    Ok(task_names)
}

fn required_task_names() -> Result<BTreeSet<String>, String> {
    required_task_names_for_tools(MANDATORY_TOOLS)
}

fn assert_required_tool_candidates_present(
    actual_tools: &[(&str, &[&str])],
    expected_tools: &[(&str, &[&str])],
) -> Result<(), String> {
    for (expected_tool, expected_candidates) in expected_tools {
        let Some((_, actual_candidates)) = actual_tools
            .iter()
            .find(|(actual_tool, _)| actual_tool == expected_tool)
        else {
            return Err(format!("missing mandatory tool entry: {expected_tool}"));
        };
        if actual_candidates != expected_candidates {
            return Err(format!(
                "mandatory tool entry drifted for {expected_tool}: expected {:?}, got {:?}",
                expected_candidates, actual_candidates
            ));
        }
    }
    Ok(())
}

fn run_self_test() -> Result<u8, String> {
    let temp = std::env::temp_dir().join(format!("moon-coverage-selftest-{}", std::process::id()));
    fs::create_dir_all(&temp).map_err(|e| format!("mkdir: {e}"))?;
    let moon_tasks_dir = temp.join(".moon").join("tasks");
    fs::create_dir_all(&moon_tasks_dir).map_err(|e| format!("mkdir: {e}"))?;

    let cleanup = || {
        let _ = fs::remove_dir_all(&temp);
    };

    assert_required_tool_candidates_present(MANDATORY_TOOLS, PRODUCTION_BOUND_KANI_TOOLS)?;

    let expected = required_task_names()?;
    write_fixture_tasks(&moon_tasks_dir, &expected)?;

    let defined = collect_defined_tasks(&temp.join(".moon"))?;
    if defined != expected {
        println!("FixtureFail: parsed {:?}, expected {:?}", defined, expected);
        cleanup();
        return Ok(1);
    }

    let full_audit = audit_moon_dir(&temp.join(".moon"))?;
    if !full_audit.gaps.is_empty() {
        println!("AuditHappyPathFail: unexpected gaps {:?}", full_audit.gaps);
        cleanup();
        return Ok(1);
    }

    let alternate_aliases = required_task_names_with_alias_overrides(
        MANDATORY_TOOLS,
        &[
            ("Loom smoke", 1),
            ("Verus verify", 1),
            ("moon ci (orchestration)", 2),
        ],
    )?;
    write_fixture_tasks(&moon_tasks_dir, &alternate_aliases)?;

    let alternate_alias_audit = audit_moon_dir(&temp.join(".moon"))?;
    if !alternate_alias_audit.gaps.is_empty() {
        println!(
            "AliasAuditFail: alternate candidate fixture unexpectedly gapped {:?}",
            alternate_alias_audit.gaps
        );
        cleanup();
        return Ok(1);
    }
    for required_alias in ["loom-list-smoke", "verify-verus-all", "quick"] {
        if !alternate_alias_audit.defined.contains(required_alias) {
            println!(
                "AliasAuditFail: missing alternate candidate task {required_alias} in {:?}",
                alternate_alias_audit.defined
            );
            cleanup();
            return Ok(1);
        }
    }

    let mut missing_fmt = expected.clone();
    missing_fmt.remove("fmt");
    write_fixture_tasks(&moon_tasks_dir, &missing_fmt)?;

    let missing_audit = audit_moon_dir(&temp.join(".moon"))?;
    if missing_audit.gaps.len() != 1 {
        println!(
            "AuditGapFail: expected 1 gap, got {} -> {:?}",
            missing_audit.gaps.len(),
            missing_audit.gaps
        );
        cleanup();
        return Ok(1);
    }
    if !missing_audit.gaps[0].contains("GAP: rustfmt") {
        println!(
            "AuditGapFail: expected rustfmt gap, got {:?}",
            missing_audit.gaps
        );
        cleanup();
        return Ok(1);
    }

    let expected_production_kani = required_task_names_for_tools(PRODUCTION_BOUND_KANI_TOOLS)?;
    write_fixture_tasks(&moon_tasks_dir, &expected_production_kani)?;

    let production_kani_defined = collect_defined_tasks(&temp.join(".moon"))?;
    let production_kani_gaps =
        collect_gaps_for_tools(&production_kani_defined, PRODUCTION_BOUND_KANI_TOOLS);
    if !production_kani_gaps.is_empty() {
        println!(
            "KaniFixtureFail: unexpected production-bound Kani gaps {:?}",
            production_kani_gaps
        );
        cleanup();
        return Ok(1);
    }

    let mut model_only_tasks = BTreeSet::new();
    model_only_tasks.insert(MODEL_ONLY_KANI_TASK.to_string());
    write_fixture_tasks(&moon_tasks_dir, &model_only_tasks)?;

    let model_only_defined = collect_defined_tasks(&temp.join(".moon"))?;
    let model_only_gaps = collect_gaps_for_tools(&model_only_defined, PRODUCTION_BOUND_KANI_TOOLS);
    if model_only_gaps.len() != PRODUCTION_BOUND_KANI_TOOLS.len() {
        println!(
            "KaniModelOnlyFail: expected {} production-bound Kani gaps, got {} -> {:?}",
            PRODUCTION_BOUND_KANI_TOOLS.len(),
            model_only_gaps.len(),
            model_only_gaps
        );
        cleanup();
        return Ok(1);
    }
    for (tool, _) in PRODUCTION_BOUND_KANI_TOOLS {
        if !model_only_gaps.iter().any(|gap| gap.contains(tool)) {
            println!(
                "KaniModelOnlyFail: missing required production-bound gap for {tool}: {:?}",
                model_only_gaps
            );
            cleanup();
            return Ok(1);
        }
    }

    cleanup();
    println!("FixturePass: moon-task-coverage parser and gap audit");
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
