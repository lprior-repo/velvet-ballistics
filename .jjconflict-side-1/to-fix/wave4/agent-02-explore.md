# Wave 4 — Agent 02 (Explore) — Bug Chunk 02

**Working dir:** `/home/lewis/src/velvet-ballistics` (git root verified)
**Branch:** `main` @ `4a0b93d1a` (8 commits ahead of `origin/main`)
**Date:** 2026-06-24
**Scope:** Read-only mapping of 6 bug IDs in `/tmp/wave4-chunk-02.txt` (note: chunk file contained 6 IDs, not 5 as stated in prompt)

## Bug Mapping Table

| bug-id | pri | files-touched | verification-artifact | targeted-cmd | result | verdict | evidence |
|--------|-----|---------------|----------------------|--------------|--------|---------|----------|
| vb-4hei8 | P0 | `crates/vb_core/benches/aggregate_resource_budget.rs` (20 lines, working tree) | n/a (criterion bench, not formal) | `cargo check -p vb_core --benches` → PASS; `cargo bench -p vb_core --bench aggregate_resource_budget --no-run` → PASS; bench name on main is `aggregate_resource_budget_contract_surface` (substring check on `vb_core::budget` source) | File compiles & runs but content does **not** match close reason | NOT-PATCHED | Historical commit `7214690cc` (on `origin/cleanup-30r` only) held the real 1000-runs bench (imports `AggregateResourceBudget, validate_step_ceilings`; name `aggregate_resource_budget/1000_runs`). Current `main` (`4a0b93d1a`) reverted to trivial substring check; `.evidence/vb-p4kca/` directory absent in main; `git log -- crates/vb_core/benches/aggregate_resource_budget.rs` on main shows only `0975204a6`. |
| vb-5dfez | P2 | `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs` (line 121 area) | kani harness file under `crates/vb_runtime/src/verification/kani/` | `rg -n "kani::," crates/` → 0 matches; `cargo fmt --all --check` → PASS; `cargo check -p vb_runtime --all-targets` → PASS | Parse error gone, file valid | PATCHED | Line 109 now uses `kani::assert!(` (valid macro form). The PO-vb282my-AA-KANI-003 harness at line 125-163 (`kani_ask_answer_pending_timer_guard`) is intact and uses `kani::any::<PendingTimerKind>()` with bounded `assume` on u16 step. No kani syntax errors detected. |
| vb-5e4xm | P0 | `xtask/src/evidence/release_rendering.rs` (lines 236-253), `xtask/src/ai_profile.rs`, `xtask/src/evidence/profile_runner.rs` | ai-release is `cargo xtask ai-release --bead vb-nf2u`, NOT a moon task | `grep -n "Command::new\|cargo\|kani\|verus" xtask/src/evidence/profile_runner.rs` → 0 matches; profile still emits `core_runtime_parity_claim: unsupported` | Pure YAML rendering from hardcoded templates; no real tool invocation | NOT-PATCHED | `render_ai_release_report` (release_rendering.rs:236-253) still hardcodes `profile: ai-release ... core_runtime_parity_claim: unsupported ... command: cargo xtask ai-release --bead vb-nf2u`. No `Command::new` in `profile_runner.rs`. No rename to `ai-release-fixture`. The bead's "either add real cargo test+clippy invocation OR rename + set runInCI: false" was satisfied for neither branch. Close reason was just "Closed" with no actual code change. |
| vb-5j3we | P0 | workspace-wide; `crates/vb_runtime/src/engine/execute/execute_tests.rs:69` (2 failing tests) | moon task `:quick` covers `cargo check --workspace` and crate test lanes | `cargo check --workspace --lib --all-targets` → PASS; `cargo test -p vb_storage --lib` → 1273/1273 PASS; `cargo test -p vb_validate --lib` → 836/836 PASS; `cargo test -p vb_runtime --lib` → 1735/1737, **2 FAIL** | Workspace restored; 2 vb_runtime tests still panic | PARTIAL | Two `engine::execute::execute_tests` tests panic at `execute_tests.rs:69:13` with `workflow validation failed: backward edge from StepIdx(0) to StepIdx(0)`: (1) `execute_reduce_start_errors_on_uninitialized_input`, (2) `execute_repeat_start_single_attempt_no_panic`. Close reason claimed "1686 pass, 24 FAIL" — current is 1735 pass, 2 FAIL. Most prior failures resolved; 2 remain (likely regressed or never fixed). |
| vb-720pw | P1 | `crates/vb_runtime/src/shard/lru_ring.rs:251` (the file itself) | n/a — `lru_ring` was replaced by `crates/vb_runtime/src/shard/completion_watermark.rs` | `grep -rn "force_insert" crates/` → 0 matches; `grep -rn "LruRing" crates/` → 0 matches; `find crates -name "lru_ring.rs"` → 0 hits | Surface gone; ring module refactored | PATCHED (REFACTORED/PHANTOM) | `lru_ring` module deleted (wave-3 refactor, see wave-2 prior reports). Replacement `completion_watermark.rs` enforces capacity at every mutation point via `max_pending` / `max_waiters` checks and returns typed `CompletionWatermarkError::QueueFull`. There is no `force_insert` path to violate the contract. |
| vb-7m2pd | P1 | `crates/vb_core/src/kani_workflow_arbitrary.rs:667` (the kani attribute); `crates/vb_storage/src/journal/append/mod.rs:38-43` (re-export). Both source paths **no longer exist** on main (refactored) | moon task `:fuzz-smoke` (builds fuzz binary) | `grep -n "proof_for(parse)" crates/` → 0 matches; `grep -rn "use self::decision::\*" crates/vb_storage/src/` → 0 matches; `find crates/vb_storage/src/journal/append` → dir absent (replaced by `append.rs` flat file); `cargo check -p vb_storage` → PASS | Both root causes gone | PATCHED | `crates/vb_storage/src/journal/` is flat (no `append/mod.rs` submodule). No `use self::decision::*` re-exports. No `#[kani::proof_for(parse)]` attribute in any vb_core file. Bead close reason correctly identified that root cause was elsewhere (kani attribute + module re-export), not the FrameSeedAccumulator. |

## Summary

- **bugs-checked:** 6
- **PATCHED:** 3 (vb-5dfez, vb-720pw, vb-7m2pd)
- **PARTIAL:** 1 (vb-5j3we)
- **NOT-PATCHED:** 2 (vb-4hei8, vb-5e4xm)
- **UNKNOWN:** 0

## Top-3 NOT-PATCHED / PARTIAL (with file:line + reason)

1. **`vb-5e4xm` — NOT-PATCHED**
   - **File:** `xtask/src/evidence/release_rendering.rs:236-253` (`render_ai_release_report`); `xtask/src/evidence/profile_runner.rs` (entire file)
   - **Reason:** ai-release profile remains purely synthetic. Hardcoded YAML still emits `core_runtime_parity_claim: unsupported`. No `Command::new("cargo"...)` / `Command::new("kani"...)` / `Command::new("verus"...)` invocation anywhere in the profile path. Bead recommended two fixes; neither was applied. Status CLOSED but evidence absent.

2. **`vb-4hei8` — NOT-PATCHED**
   - **File:** `crates/vb_core/benches/aggregate_resource_budget.rs:1-20` (working tree)
   - **Reason:** Current `main` (commit `4a0b93d1a`) has a 20-line trivial bench (substring check on `vb_core::budget` source). The real 1000-runs bench (importing `AggregateResourceBudget, validate_step_ceilings`; name `aggregate_resource_budget/1000_runs`) exists only on `origin/cleanup-30r` (commit `7214690cc`) and never landed on `main`. `.evidence/vb-p4kca/` directory absent on `main` (`git show 4a0b93d1a:.evidence/vb-p4kca/metadata.yaml` → fatal). Bead close reason is misleading.

3. **`vb-5j3we` — PARTIAL**
   - **File:** `crates/vb_runtime/src/engine/execute/execute_tests.rs:69`
   - **Reason:** Workspace check + `vb_storage` (1273/1273) + `vb_validate` (836/836) are all green per close claim, but `vb_runtime --lib` has 2 actual panics: `engine::execute::execute_tests::execute_reduce_start_errors_on_uninitialized_input` and `engine::execute::execute_tests::execute_repeat_start_single_attempt_no_panic`. Both panic at line 69 with `workflow validation failed: backward edge from StepIdx(0) to StepIdx(0)`. Close claim of "24 cancel/collect failures" reduced to 2, but 2 remain — likely the same or new engine::execute workflow-validation failures.

## Output

- **file-path written:** `/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-02-explore.md`
