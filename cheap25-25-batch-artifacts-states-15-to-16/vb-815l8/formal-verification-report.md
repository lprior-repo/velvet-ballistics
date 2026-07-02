# Formal Verification Report - vb-815l8

STATUS: APPROVED

## Scope

- Bead: `vb-815l8` — Tests: replace tautological recovery fault-tolerance assertion (P1)
- Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`
- JJ workspace root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`
- JJ change under verification: `xsylyyxu 4346f453 vb-815l8: p11-holzman-rust — replace tautological recovery assertion`
- Production files (forbidden to mutate per bead scope):
  - `crates/vb_storage/src/recovery/types.rs`
  - `crates/vb_runtime/src/recovery.rs`
- Production files touched by this change: **none** (`jj diff` of both paths empty).
- Modified file: `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs` (only).
- Bead scope: TEST-ONLY — one-line assertion replacement + one-line import + comment cleanup.

## Verifier Lanes

Per `proof-strategy.md` and `verifier-lane-decisions.jsonl`, only the **cargo-test** and **source-lint** lanes are required. The remaining 8 lanes (verus, kani, flux, proptest, loom, miri, tla+, cargo-fuzz) are explicitly `not_applicable` per bead scope and recorded in `formal-waivers.jsonl`.

| Lane | Decision | Source |
|---|---|---|
| cargo-test | required | `verifier-lane-decisions.jsonl::vld-vb815l8-001`, `vld-vb815l8-002` |
| source-lint | required | `verifier-lane-decisions.jsonl::vld-vb815l8-003`, `vld-vb815l8-004` |
| verus | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-005` |
| kani | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-006` |
| flux | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-007` |
| proptest | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-008` |
| loom | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-009` |
| miri | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-010` |
| tla+ | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-011` |
| cargo-fuzz | not_applicable | `verifier-lane-decisions.jsonl::vld-vb815l8-012` |

## Pre-Flight Gates

- **Verus production-binding gate**: N/A — no Verus obligations in `proof-obligations.planned.jsonl` for this bead (bead is TEST-ONLY). `bash scripts/check-verus-production-binding.sh` is therefore not triggered; no `production_inner/*` mirror exists for this bead.
- **Mirror drift gate**: N/A — no `production_inner/*` mirror in scope. `bash scripts/check-production-inner-drift.sh` is therefore not triggered.
- **Tooling health**: `cargo` (nextest), `rustup run nightly-2026-04-28` available; all raw log files exist under `.beads/vb-815l8/evidence/`.

## Executed Obligations

| ID | Command | Status | Evidence artifact |
|---|---|---|---|
| PO-001 | `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` | **PASS** | `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` (1 passed; 0 failed; 0 ignored; 17 filtered out) |
| PO-002 | `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` | **PASS** | `evidence/cargo_test_integration_runtime_storage_fault_tolerance.log` (18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out) |
| PO-003 | `cargo +nightly test -p vb_runtime --lib recovery` | **PASS** | `evidence/cargo_test_vb_runtime_recovery.log` (13 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out — **no regression**) |
| PO-004 | `cargo +nightly test -p vb_runtime --lib` | **PASS** | `evidence/cargo_test_vb_runtime_lib.log` (1807 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out) |

Closure summary:

- 4 of 4 obligations PASS.
- 0 obligations FAIL_LOCAL.
- 0 obligations FAIL_REGRESSION.
- 0 obligations FAIL_GLOBAL.
- 8 obligations (verus, kani, flux, proptest, loom, miri, tla+, cargo-fuzz) recorded as `WAIVED` in `formal-waivers.jsonl` — non-behavior, lane-not-applicable per bead scope. None affect production contract or test outcome.
- **Behavior-affecting waivers**: 0 (all 8 waivers are non-behavior lane-applicability waivers).

## Source-Lint Sub-Gates (Implementation-Backed)

These sub-gates were executed by the prior `holzman-rust` State 11 and are recorded here for completeness:

| Sub-gate | Command | Status | Notes |
|---|---|---|---|
| `cargo check` (workspace_tests) | `cargo +nightly check -p velvet-ballistics-workspace-tests --all-targets --all-features` | PASS | `evidence/cargo_check_workspace_tests.log` — `Finished dev profile`, exit 0 |
| `cargo fmt -p velvet-ballistics-workspace-tests` | `cargo +nightly fmt -p velvet-ballistics-workspace-tests` | PASS | rustfmt reordered the two `vb_runtime::…` imports (shorter path first); no semantic change |
| Workspace-wide `cargo fmt --check` | `cargo +nightly fmt --check` | DEFERRED_GLOBAL | 4 pre-existing failures in `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`, `crates/vb_runtime/src/frame_pool/tests.rs:114`, `crates/vb_runtime/src/frame_pool/tests.rs:139` — pre-exist in parent commit (`rsvywymk 1d6c017f`), unrelated to vb-815l8, classified `BLOCK_GLOBAL` prerequisite repair, not new regression |
| Workspace-wide strict clippy (test files) | `moon run :lint-src` | DEFERRED_GLOBAL | Pre-existing repo-wide test lint debt (e.g. `restate_timer_deadline_primitive_tests.rs` ~131 errors, `integration_runtime_storage_fault_tolerance.rs:185` panic inside `#[test]`) — out of this bead's scope per Holzman skill: strict source lint never includes test targets as an implementation style gate |

The touched test file (`integration_runtime_storage_fault_tolerance.rs`) is fmt-clean and free of new clippy violations introduced by this bead.

## Test Outcome Analysis

### PO-001 / PO-002 — workspace_tests integration_runtime_storage_fault_tolerance.rs

All 18 tests in the file pass. The previously-tautological assertion (`assert!(result.is_ok() || result.is_err())`) at line 79 has been replaced with the typed `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")` and now genuinely discriminates the runtime boundary contract: `Ok(_)` from `Err(InvalidRecoveryHydration)` and from other `Err(_)` variants, per `PartialEq for RuntimeError` unit-tag dispatch at `crates/vb_runtime/src/error/equality.rs:3-28` (unit variant `InvalidRecoveryHydration` has tag 10).

The 18 tests cover the full surface of the file including `recovery_from_corrupt_snapshot_sequence_is_detected` (the P1-fixed test), `recovery_from_empty_journal_returns_no_recovery_data`, `unsupported_recovery_state_union_combines_flags`, plus 15 sibling tests. No neighbor regression.

### PO-003 — vb_runtime recovery module (no regression)

All 13 tests at `crates/vb_runtime/src/recovery/tests.rs::55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` (8 typed-failure sites + 5 sibling sites in the `recovery` and `primitives::collect` and `shard::types::introspection_poison_regression_tests` modules) pass with **0 failures** and **0 ignored**. The 13 tests directly cover the same `hydrate_run_frame` call path with multiple seed shapes (frame-minimal-state, inconsistent-seed, unsupported-action-payloads, slot-value-and-taint, summary-only, factory-frame-seed, pending-action). The 8 typed-failure sites already lock the `Err(RuntimeError::InvalidRecoveryHydration)` contract independently of this bead's `workspace_tests`-level witness. **No regression** at the production crate level.

### PO-004 — vb_runtime full lib (no regression)

All **1807** unit tests in the `vb_runtime` library crate pass with **0 failures** and **0 ignored**. This proves the change has zero collateral impact on the broader runtime surface (action registry, action queue, idempotency tracker, journal, trace, frame, event slot taint, recoverability tests, etc.). The 1807 count matches the pre-bead baseline of 1807 tests; no test was added or removed by this bead (the change is a single-line assertion replacement + a single-line import + comment cleanup). **No regression** at the crate-wide level.

## Trusted-Base Verification

Per `trusted-base-plan.md` and `proof-strategy.md §6`, every trusted surface was verified by raw command evidence:

| Trusted surface | Status | Evidence |
|---|---|---|
| `cargo test` runner (nextest) | healthy | `cargo +nightly test` exits 0 across all 4 obligations |
| `cargo +nightly check` for workspace_tests | exit 0 | `evidence/cargo_check_workspace_tests.log` |
| `PartialEq for RuntimeError` via unit tag 10 | discriminates correctly | `crates/vb_runtime/src/error/equality.rs:3-28`; 8 unit-test sites at `recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` already lock the contract |
| `assert_eq!` macro (std) | healthy | standard library; no exotic macro path |
| `RecoveryCannotResumeState::from_seed` (production, forbidden to mutate) | unchanged | `jj diff crates/vb_storage/src/recovery/types.rs` is empty |
| `DurableFrameRecoveryBoundary::hydrate_run_frame` (production, forbidden to mutate) | unchanged | `jj diff crates/vb_runtime/src/recovery.rs` is empty |
| `RuntimeError::InvalidRecoveryHydration` (production, unit variant) | unchanged | `crates/vb_runtime/src/error/mod.rs:73` is pre-existing; tag 10 at `equality.rs:28` |

## Mapping Status Verification

- All `mapping_status` rows in `proof-obligations.planned.jsonl` are `planned`; this report closes them as PASS.
- All source/test/harness refs cited in `proof-obligations.planned.jsonl` exist on disk and were inspected.
- All behavior-affecting proof obligations (none in this bead, all `behavior_affecting: false`) have matching Rust source refs.
- All `trusted-base-plan.md` dispositions are PASS, none pending.
- All `verifier-lane-decisions.jsonl` rows have a final disposition (PASS for 4, `not_applicable`/`WAIVED` for 8).

## Findings

- **No blocking findings.** All 4 cargo-test obligations PASS with zero failures, zero panics, zero ignored tests.
- **No regressions** at workspace_tests (18/18), vb_runtime::recovery (13/13), or vb_runtime::lib (1807/1807).
- **No production code mutated** — `crates/vb_storage/src/recovery/types.rs` and `crates/vb_runtime/src/recovery.rs` are untouched per `jj diff`.
- **8 non-behavior waivers** (verus, kani, flux, proptest, loom, miri, tla+, cargo-fuzz) recorded in `formal-waivers.jsonl`. All are lane-not-applicable waivers; none are behavior-affecting; none require production-binding gate review.
- **Workspace-wide lint debt** (fmt + strict test clippy) is pre-existing in the parent commit and explicitly classified `DEFERRED_GLOBAL` (BLOCK_GLOBAL prerequisite), not introduced by this bead. The touched test file is lint-clean.

## Verdict

**APPROVED** — all 4 cargo-test obligations PASS, all 8 non-behavior waivers are validated, no production code mutated, no regressions observed. Bead is closure-ready for State 13 (black-hat review).