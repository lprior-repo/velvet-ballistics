# Wave 4 / Agent 11 — Rust Contract (Type/Domain Auditor) Review

Scope: 6 bug IDs from `/tmp/wave4-chunk-11.txt` — `vb-maupz`, `vb-mx7qt`,
`vb-nx1b2`, `vb-nyw4m`, `vb-p528k`, `vb-q7d5c`. Wave 4 covers CI / formal /
evidence gating. This audit applies the rust-contract lens: ubiquitous-language
correctness, typed-error discipline, post-commit/post-write failure windows,
canonical-naming, dependency/wiring, and CI gate fail-closed-ness, against
master Section 40 (CI gate), Section 43 (AI acceptance), and the
`Cargo.toml`/`.moon/tasks/all.yml` policy.

All commands run from the absolute workspace root
`/home/lewis/src/velvet-ballistics`. Git root, `jj root`, and command
working directory all agreed. No source modified. No beads created.

## Per-bug audit table

| bug-id    | pri | source-fix                                                                    | test                                                                | fail-closed           | coverage                      | targeted-cmd                                                                                                                       | result                                                                                  | verdict     | evidence                                                                                                                                            |
|-----------|-----|-------------------------------------------------------------------------------|---------------------------------------------------------------------|-----------------------|-------------------------------|-------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------|-------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| vb-maupz  | P3  | NONE — `crates/vb_storage/src/admission.rs:387-414` still runs `put_compiled_ir` → `persist_strict()` → post-write `journal.compiled_ir(...)` read check, mapping any transient read failure to `JournalError::ArtifactMalformed`. The `verify_persisted_artifact_present` function and the suggested `JournalError::AcceptedArtifactVerifyFailed` variant do not exist in the tree; `admission/flow.rs` referenced by the close reason does not exist either. The "post-commit failure window" the bead claims to have removed is still present — only the function name was changed, not the behavior. | `cargo test -p vb_storage --lib submit_artifact` — 17 passed, 0 failed. Tests cover happy paths and the strict/journalled/relaxed policies, but no regression test exercises a transient read failure post-`persist_strict`. | fail-closed (`set -euo pipefail` in coverage, test, and `ci` tasks) | master §40 has no explicit min %; moon `coverage` task is a smoke single-test gate, no `--fail-under-lines/--fail-under-regions` | `cargo test -p vb_storage --lib submit_artifact_strict_is_durable` → 2 passed; `cargo check -p vb_storage --lib` → clean. The post-write read at `crates/vb_storage/src/admission.rs:409-414` is still the SA-011 window. | NOT-PATCHED | `crates/vb_storage/src/admission.rs:387-414`; no `JournalError::AcceptedArtifactVerifyFailed`; no `admission/flow.rs` file.                                              |
| vb-mx7qt  | P2  | Stale bead: the cited `vb_a0t1_source_length_gate_tests.rs:679` and `vb_8ma2_workspace_assertions.rs:323` line numbers do not match the current tree (`vb_a0t1_source_length_gate_tests.rs` no longer exists; `vb_8ma2_workspace_assertions.rs` is 253 lines, not 323). The cited `crates/vb_runtime/src/runtime/mod.rs:13-14` does not exist — `vb_runtime/src/runtime.rs` is a flat file with no `runtime_recovery`/`runtime_sharding` modules. The "missing `[legacy-tests]` feature in `crates/vb_core/Cargo.toml`" claim is true, but there is no `legacy-tests` feature referenced anywhere in the workspace. | `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions valid_workspace_passes_sharpened_assertions` → 1 passed; `--test vb_test_runtime_ipc_resource_behavior edge_submit_after_shutdown` → 1 passed. The third cited test (`out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence`) cannot be located. | fail-closed (all moon tasks `set -euo pipefail`)                  | smoke (single-test llvm-cov)  | `cargo check -p velvet-ballistics-workspace-tests --tests` → clean; `cargo check -p vb_runtime --lib` → clean.                       | PARTIAL (cannot confirm against stale line numbers, two of three cited tests now pass, third is gone) | PARTIAL     | `crates/vb_core/Cargo.toml:25-30` (no `legacy-tests`); `crates/vb_runtime/src/runtime.rs` (single file, no `runtime_recovery`/`runtime_sharding`); `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:174` (test exists at new line, not 323). |
| vb-nx1b2  | P3  | Bead is a duplicate of `vb-qp6qh` (RS-214, runtime shard introspection epoch saturation). The underlying fix is in `crates/vb_runtime/src/shard/types.rs:400-404,425-429,440-444`: all three `next_epoch` increments use `checked_add(1).ok_or(RuntimeError::IntrospectionEpochExhausted)` — fail-closed, no silent `saturating_add` wrap at `u64::MAX`. The new typed error `RuntimeError::IntrospectionEpochExhausted` is wired through `error/mod.rs:192`, `error/diagnostics.rs:98,156`, `error/display.rs:61`, `error/equality.rs:33`. Drop guard at `types.rs:343-353` already checks epoch equality before unregistering, so stale handles cannot unregister current ones. | `cargo test -p vb_runtime --lib epoch` → `introspection_register_returns_typed_error_when_next_epoch_is_max` passes. | fail-closed (moon tasks `set -euo pipefail`)                       | smoke (single-test llvm-cov)  | `cargo test -p vb_runtime --lib epoch` → 2 passed including the typed-error test.                                                   | PATCHED (via parent `vb-qp6qh`)                                                        | PATCHED     | `crates/vb_runtime/src/shard/types.rs:400-404`; `crates/vb_runtime/src/error/mod.rs:192`; `IntrospectionEpochExhausted` cross-references in `error/{diagnostics,display,equality}.rs`. |
| vb-nyw4m  | P0  | `vb_validate` type-mismatches in `diag_render/mapping.rs` are fixed; forbidden `velvet-ballistics:*` colon-dirs at workspace root are gone (`find -maxdepth 1 -name 'velvet-ballistics:*'` returns 0). However, a regression has been introduced in `crates/vb_runtime/src/engine/execute/execute_tests.rs`: tests `execute_reduce_start_errors_on_uninitialized_input` (line 1081) and `execute_repeat_start_single_attempt_no_panic` (line 1212) build workflows with `body: StepIdx::new(0)` self-loops that the `CompiledWorkflow::try_from_parts` validator now rejects with `backward edge from StepIdx(0) to StepIdx(0)`. The 24 listed collect/cancel/config/reentry tests now all pass (1735 passed, 2 failed). | `cargo test -p vb_runtime --lib` → 1735 passed, **2 failed** in `engine::execute::execute_tests::{execute_reduce_start_errors_on_uninitialized_input,execute_repeat_start_single_attempt_no_panic}`. `cargo test -p vb_validate --lib type_taint` → 149 passed. | fail-closed (`set -euo pipefail` in all CI tasks)                  | smoke (single-test llvm-cov)  | `cargo test -p vb_runtime --lib` → 2 failures; `cargo check --workspace --lib --all-targets` → clean.                                 | PARTIAL (24 listed tests fixed; 2 new execute regressions) | PARTIAL     | `crates/vb_runtime/src/engine/execute/execute_tests.rs:1081,1212`; `CompiledWorkflow::try_from_parts` rejects `StepIdx(0)→StepIdx(0)`.                                  |
| vb-p528k  | P1  | NONE — `crates/vb_runtime/src/verification/kani/mod.rs` still wires only 4 modules: `kani_retry_math`, `kani_for_each_ordering`, `kani_together_ordering`, `kani_engine_signals`. The directory has 14 `.rs` files (including `mod.rs`), leaving 10 orphaned files uncompiled by the crate: `kani_admission_ordering`, `kani_ask_answer_lifecycle`, `kani_attempt_fence_harnesses`, `kani_cancel_kill_lattice`, `kani_idempotency_tracker`, `kani_resume_state_machine`, `kani_shard_lifecycle_harnesses`, `kani_sxkz6_shard_for_run`, `vb_fzgdn_timer_harnesses`, plus `kani_submit_frame_release` referenced in the original finding. No `#[trusted]`, `#[ignore]`, or `#[opaque]` attributes present — the orphaned files use only `#[kani::unwind(N)]`, which is the correct Kani pattern (not a fail-open concern). The bead was administratively closed but the master §4 wiring/deadline has not been met. | No test exercise — the orphaned harnesses are not in the build. `cargo kani` is not run in the `ci` task; `verify-kani`/`verify-kani-vb-validate` (in `kani.yml`) only cover `vb_core` and `vb_validate`. | fail-closed for the wired gates; the unwired modules are not in scope of any CI gate. | n/a (not a coverage task) | `rg '#\[(trusted|ignore|opaque)\]' crates/vb_runtime/src/verification/kani/` returns 0 matches; `kani-list.json` does not include vb_runtime harnesses. | NOT-PATCHED (10 modules still orphaned) | NOT-PATCHED | `crates/vb_runtime/src/verification/kani/mod.rs:3-6` (4 modules); directory has 14 `.rs` files; `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs:47` etc. use `#[kani::unwind(5/10)]`. |
| vb-q7d5c  | P1  | PATCHED — the three mutually-incompatible states have been resolved by consolidating the code registry into a single source of truth. `crates/vb_core/src/diagnostic.rs` now defines `pub const CODE_REGISTRY: &[CodeEntry] = &[ ... ];` at line 118 (the State A `ENTRIES`/`build_registry` pattern), and the 20 sibling data files have been merged into a single flat file. The directory `crates/vb_core/src/diagnostic/codes/` no longer exists; there is no `codes.rs`/`accessor.rs` unclosed-delimiter risk (State C). The `include!()` pattern (State B) is also gone — there is no `include!` in `diagnostic.rs` per the search. A concurrent edit-loop CI guard is not present, but the underlying ambiguity has been removed by collapse-to-one-file, which makes the loop impossible to enter. | `cargo check -p vb_core --lib` → clean; `cargo test -p vb_core --lib diagnostic` → 86 passed, 0 failed. The test that previously failed to compile (State C) is gone, and the new flat-file format is the only state in the tree. | fail-closed (moon `ci` runs `check` then `lint-src`, both `set -euo pipefail`) | smoke (single-test llvm-cov)  | `rg 'ENTRIES|include!|build_registry' crates/vb_core/src/diagnostic.rs` → 0 matches; `ls crates/vb_core/src/diagnostic/codes/` → no such directory. | PATCHED                                                            | PATCHED     | `crates/vb_core/src/diagnostic.rs:118` (`pub const CODE_REGISTRY`); directory layout collapsed to single file; `diagnostic/tests_and_verification.rs` is the only sibling.          |

## Summary

- bugs-checked: 6
- pass: 2 (`vb-nx1b2`, `vb-q7d5c`)
- partial: 2 (`vb-mx7qt`, `vb-nyw4m`)
- fail: 2 (`vb-maupz`, `vb-p528k`)
- unknown: 0

## Fail-open gates

Count: 0.

The only `set +e` in `.moon/tasks/all.yml` is at line 379 inside the advisory
`supply-chain` task's `run_geiger` helper. It is bounded: the geiger exit code
is captured into `geiger_status`, `set -e` is restored immediately, and the
task fails closed if the markdown report is missing or the status is not in
{0, 1}. Per master §40 and the 2026-05-23 owner waiver, advisory supply-chain
reports are non-blocking. No `|| true`, no swallowed exit, no `exit 0` after
a failed step. The mandatory gate set (`check`, `test`, `lint-src`, `fmt`,
`banned-token-gates`, `source-length`) all use `set -euo pipefail`.

CI-vs-master gap (not a fail-open, but worth flagging): the `ci` task in
`.moon/tasks/all.yml:793-803` runs only `fmt`, `banned-token-gates`,
`source-length`, `check`, `lint-src`, `test`. Master §40 additionally requires
`feature-powerset`, `miri`, `coverage`, `mutants-smoke`, `bench-build`,
`fuzz-smoke`, and the kani/verus tasks from `kani.yml`/`verus.yml`. Those
tasks are present and fail-closed individually (`runInCI: true`), but they
are not chained from `ci`, so a single `moon ci` invocation does not run the
full master §40 pipeline. This is a contract-parity gap with the master, not
a fail-open defect.

## Coverage threshold mismatches

Count: 0 explicit threshold mismatches.

Master §40 invokes `cargo llvm-cov --workspace --all-features` and the moon
`coverage` task implements it at `all.yml:451-471`. Master §40 does **not**
specify a minimum line/region percentage. The moon `coverage` task is a smoke
single-test gate (`action::tests::validate_action_outcome_failed_always_succeeds`),
with the comment "Smoke policy: full confidence comes from the test lane".
There is no `--fail-under-lines`, no `--fail-under-regions`, no `cargo-tarpaulin`
threshold, and no coverage gate in any other task. A check of the supplied
`tarpaulin-report.json` shows it is 3 bytes (empty placeholder), so there is
no actual coverage artifact to compare against a threshold. This is consistent
with master, so there is no mismatch — but the master is silent on the
minimum, so any future tightening will need a new bead.

## Top-3 NOT-PATCHED with reason

1. **vb-maupz (SA-011 storage admission partial failure)** — The bead's
   close reason cites `admission/flow.rs:99-112` for the removal of the
   post-commit `verify_persisted_artifact_present` call, but that file does
   not exist. The current `submit_artifact_with_contracts` in
   `crates/vb_storage/src/admission.rs:387-414` still does
   `put_compiled_ir` → `persist_strict()` → `journal.compiled_ir(...)` read
   check, and a transient read failure still maps to
   `JournalError::ArtifactMalformed` rather than a dedicated
   `JournalError::AcceptedArtifactVerifyFailed`. The post-write failure
   window the bug asked to remove is still present; only the function name
   changed. No regression test exercises the failure mode.

2. **vb-p528k (ARCH-W0-02: 10 Kani modules orphaned)** — `mod.rs` at
   `crates/vb_runtime/src/verification/kani/mod.rs:3-6` still wires only 4
   of the 14 `.rs` files in the directory. The other 10 are dead code from
   the perspective of the crate compiler and the kani gate
   (`cargo kani -p vb_runtime` is not in `ci`; `kani-list.json` and the
   `verify-kani*` tasks only cover `vb_core`/`vb_validate`). No
   `#[trusted]`/`#[ignore]`/`#[opaque]` abuse (the only Kani attributes in
   the directory are correct `#[kani::unwind(N)]` annotations), so the
   orphaned files are not a fail-open, but they violate master §4 (wire
   or delete) and the bead was closed without action.

3. **vb-nyw4m (Wave 6 regressions)** — The 24 collect/cancel/config tests
   are fixed, the `vb_validate` type-mismatches are gone, and the four
   forbidden `velvet-ballistics:*` colon-dirs at the workspace root have
   been removed. However, `cargo test -p vb_runtime --lib` now fails two
   tests that were not in the original bug list:
   `engine::execute::execute_tests::execute_reduce_start_errors_on_uninitialized_input`
   (`execute_tests.rs:1081`) and `engine::execute::execute_tests::execute_repeat_start_single_attempt_no_panic`
   (`execute_tests.rs:1212`). Both build a workflow with a self-loop
   (`body: StepIdx::new(0)`) that the tightened `CompiledWorkflow::try_from_parts`
   validator rejects with `backward edge from StepIdx(0) to StepIdx(0)`.
   Either the validator should permit test-mode degenerate bodies, or the
   test fixtures need a non-zero `body`. Until then, the CI test gate is
   red on this regression.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-11-rust-contract.md`
