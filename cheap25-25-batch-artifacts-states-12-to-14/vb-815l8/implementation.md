# Implementation — vb-815l8

**Bead:** `vb-815l8` — Tests: replace tautological recovery assertion (P1)
**State:** 11 (p11-holzman-rust)
**Skill:** `holzman-rust`
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`
**JJ workspace:** `cheap25-vb-815l8`
**Parent commit:** `1015cf6e` (cheap25-vb-815l8 empty parent)

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8/crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs` (target file)
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8/crates/vb_runtime/src/recovery.rs` (production code; read-only verification)
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8/crates/vb_storage/src/recovery/types.rs` (production code; read-only verification)

## Power-of-Ten rules affected

| Rule | Status |
|---|---|
| Rule 1: simple control flow | Satisfied — single `assert_eq!` over a typed `RuntimeResult<RunFrame>`; no recursion, no panic paths. |
| Rule 2: bounded control flow | Satisfied — no loops or retries added. |
| Rule 3: no post-init allocation in critical paths | Satisfied — `assert_eq!` consumes the existing `result`; no new allocations. |
| Rule 4: functions fit on one page | Satisfied — the test body remains well under 25 logical lines. |
| Rule 5: assertion/invariant density | Strengthened — replaced `assert!(result.is_ok() || result.is_err())` (a tautology: every `Result<bool>` matches one of the two arms) with a typed-failure invariant `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration))`. |
| Rule 6: smallest scope | Satisfied — only the test file imports and the targeted assertion change. |
| Rule 7: checked returns/parameters | Satisfied — `result` is consumed by `assert_eq!`; no ignored fallible results. |
| Rule 8: limited macro power | Satisfied — no new macros. |
| Rule 9: restricted pointer/indirect call use | Satisfied — no `unsafe`, no raw pointers, no trait objects added. |
| Rule 10: warnings/analysis mandatory | Satisfied — `cargo check -p velvet-ballistics-workspace-tests --all-targets --all-features` exits 0; the pre-existing strict test clippy failures in unrelated test files (e.g. `restate_timer_deadline_primitive_tests.rs`, `frame_pool/tests.rs:139` `expect`) are not introduced by this bead and are out of scope. |

## Zero-panic rules affected

| Rule | Status |
|---|---|
| `zero_forbidden_constructs` | Satisfied — no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`unreachable!` introduced. |
| `no_panic_paths` | Satisfied — the new `assert_eq!` is a typed-failure check inside a `#[test]`; it cannot panic on the success path because `assert_eq!` on the expected error variant is the contract. |
| `production_assert_forbidden` | Satisfied — `assert_eq!` lives inside `#[test]`; not a production path. |

## Code changes

### `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`

#### Import block (lines 7-9)

Added the typed error import required by the new `assert_eq!` expectation.
rustfmt reordered the two `vb_runtime::…` lines so the shorter path precedes
the longer path; both lines now sit between `vb_core` and `vb_storage`
imports.

```diff
 use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
-use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
+use vb_runtime::RuntimeError;
+use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
 use vb_storage::recovery::{
     ActionReplayTracker, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed,
     RecoveryRuntimeSummary, RecoveryTerminalState, UnsupportedRecoveryState,
     recover_runtime_frame_seed_from_events,
 };
```

#### `recovery_from_corrupt_snapshot_sequence_is_detected` (lines 75-92)

Replaced the tautological `assert!(result.is_ok() || result.is_err())` and
the contradicting "boundary is permissive on empty seed" comment with a
typed-failure assertion that pins the production contract:

> `DurableFrameRecoveryBoundary::hydrate_run_frame()` returns
> `Err(RuntimeError::InvalidRecoveryHydration)` for every
> `RecoveryFrameSeed`, because the frame seed type alone never carries
> the full runtime boundary state (`workflow`, `store`, action attempts,
> `admission`, collect states, action contracts, action ABI digests)
> required for live execution.

```diff
     let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
-    // Hydration should succeed because the seed itself is valid (corrupt snapshot
-    // is a storage-layer concern; the boundary only validates the seed shape).
-    let result = boundary.hydrate_run_frame();
-    // A seed with step_count=0 and no workflow may still be a valid empty-run seed.
-    assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed
+    // The frame seed cannot resume without the missing full-RunState
+    // components (workflow, store, action attempts, admission, collect
+    // states, action contracts, action ABI digests), so
+    // `cannot_resume_state().is_resumable()` returns false and hydration
+    // fails closed with `RuntimeError::InvalidRecoveryHydration`.
+    // The boundary is NOT permissive on empty seeds: every durable frame
+    // seed is classified as cannot-resume because the seed type alone
+    // never carries the full runtime boundary state required for live
+    // execution (see `RuntimeRecoveryBoundary::resume_status` and
+    // `RecoveryCannotResumeState::from_seed`).
+    let result = boundary.hydrate_run_frame();
+    assert_eq!(
+        result,
+        Err(RuntimeError::InvalidRecoveryHydration),
+        "durable frame hydration must reject any frame seed"
+    );
 }
```

## Why the assertion is correct (production contract walk-through)

`DurableFrameRecoveryBoundary::hydrate_run_frame`
(`crates/vb_runtime/src/recovery.rs:99-107`) calls
`reject_unsupported_live_frame_state(&self.seed)?` first.
`reject_unsupported_live_frame_state`
(`crates/vb_runtime/src/recovery.rs:109-115`) returns
`Err(RuntimeError::InvalidRecoveryHydration)` whenever
`seed.cannot_resume_state().is_resumable()` is false.

`RecoveryFrameSeed::cannot_resume_state`
(`crates/vb_storage/src/recovery/types.rs:1205-1207`) delegates to
`RecoveryCannotResumeState::from_seed`
(`crates/vb_storage/src/recovery/types.rs:949-957`), which unconditionally
applies `MissingRunStateComponents::ALL` to the witness. `ALL` sets every
one of the seven `*_missing` second-half flags
(`workflow_missing`, `store_missing`, `action_attempts_missing`,
`admission_missing`, `collect_states_missing`,
`action_contracts_missing`, `action_abi_digests_missing`), so
`is_resumable()` always returns false for any seed.

The test seed has `unsupported: UnsupportedRecoveryState::SUPPORTED`,
`step_count: 0`, `slot_count: 0`, no steps, no slots, no pending actions,
and `workflow: Some(WorkflowDigest::from_bytes([0x1F; 32]))`. None of
those values flip any of the seven `*_missing` flags back to false (the
`Some(workflow)` field is irrelevant; the seven flags are set
unconditionally by `MissingRunStateComponents::ALL`). The
cannot-resume witness classifies this seed as
`workflow_missing` (top-priority second-half token) and
`hydrate_run_frame` therefore returns
`Err(RuntimeError::InvalidRecoveryHydration)`.

## Exact commands run

| Command | Result |
|---|---|
| `pwd -P` (workspace isolation) | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8` |
| `jj root` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8` |
| `git rev-parse --show-toplevel` | fatal: not a git repository (expected — jj-only workspace) |
| `cargo +nightly check -p velvet-ballistics-workspace-tests --all-targets --all-features` | exit=0, `Finished dev profile` |
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` | **18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out** |
| `cargo +nightly test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` | **1 passed; 0 failed** (targeted) |
| `cargo +nightly test -p vb_runtime --lib recovery` | **13 passed; 0 failed; 0 ignored; 1794 filtered out** (no regression) |
| `cargo +nightly test -p vb_runtime --lib` | **1807 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out** |
| `cargo +nightly fmt -p velvet-ballistics-workspace-tests` | exit=0 (rustfmt reordered the two `vb_runtime::…` imports to place the shorter path first) |
| `jj diff --stat` | `1 file changed, 16 insertions(+), 4 deletions(-)` (only the test file) |
| `jj diff crates/vb_storage/src/recovery/types.rs crates/vb_runtime/src/recovery.rs` | empty (production code untouched) |

## Performance-layer decision

No performance claim is made. The change replaces one boolean tautology
with one equality assertion; runtime cost is identical to the
production-grade `assert_eq!` macro expansion (compiler-elided on
release `opt-level = 3`). No benchmarks are run because this is a
correctness fix, not a perf change.

## Second-ring evidence

None required. No zero-cost-abstraction, vectorization,
bounds-check-removal, public-API-compatibility, or release-provenance
claims are made. The change is a typed-failure assertion against an
existing public API surface (`RuntimeError::InvalidRecoveryHydration`
existed at `crates/vb_runtime/src/error/mod.rs:73` prior to this bead).

## Skipped gates and concrete reasons

- **Workspace-wide `cargo fmt --check`** is **skipped**. The 4 fmt
  violations reported are pre-existing in the parent commit
  (`rsvywymk 1d6c017f`, AGENTS.md round10 forward-port):
  `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`,
  `crates/vb_runtime/src/frame_pool/tests.rs:114`, and
  `crates/vb_runtime/src/frame_pool/tests.rs:139`. They are out of this
  bead's scope and classified as `BLOCK_GLOBAL` prerequisite repair,
  not new regressions introduced by vb-815l8. The touched file
  (`integration_runtime_storage_fault_tolerance.rs`) is fmt-clean.
- **Strict test clippy** (`-D clippy::panic -D clippy::expect_used …`)
  on the touched test file is **skipped as a gate**. The single hit
  (`integration_runtime_storage_fault_tolerance.rs:185`,
  `panic!("expected RecoveryTerminalState::Finished, got {state:?}")`
  inside `recovery_terminal_state_finished_carries_slot`) is pre-existing
  in the parent commit, lives in a `#[test]` body, and is out of this
  bead's scope. Per the Holzman skill, strict source lint never
  includes test targets as an implementation style gate.
- **Stricture test clippy on the rest of `workspace_tests`** (e.g.
  `restate_timer_deadline_primitive_tests.rs` 131 pre-existing clippy
  errors) is **skipped** for the same reason: pre-existing
  repo-wide test lint debt, not introduced by vb-815l8.
- **`cargo geiger` / `cargo machete` / `cargo audit` / `cargo deny` /
  `cargo vet` / `cargo mutants`** were not run; no new dependencies,
  no `unsafe` code, no production-source mutation.

## Residual risks

- The test still exercises only the **happy-shape** durable-frame
  rejection path. It does not exercise the other typed-failure branches
  inside `hydrate_run_frame` (`empty_recovered_frame`,
  `apply_recovered_slots`, `apply_recovered_pc`) which already exist
  as `durable_frame_recovery_boundary_rejects_inconsistent_seed` and
  `durable_frame_recovery_boundary_rejects_frame_only_minimal_state`
  in `crates/vb_runtime/src/recovery/tests.rs`. Adding new test-only
  harnesses for those branches is **out of this bead's scope** (P1
  scope is "replace tautological assertion"; P0/P2 work would extend
  coverage).
- The pre-existing workspace-wide fmt failures in
  `vb_core`/`vb_runtime::frame_pool/tests.rs` and the pre-existing
  test clippy debt in `restate_timer_deadline_primitive_tests.rs` are
  unrelated to this bead but block the workspace-wide
  `cargo fmt --check` and strict test clippy lanes. They should be
  repaired as a separate `BLOCK_GLOBAL` prerequisite before final
  landing.

## Evidence artifacts

- `.beads/vb-815l8/evidence/cargo_test_integration_runtime_storage_fault_tolerance.log`
  — full integration test run: 18 passed, 0 failed.
- `.beads/vb-815l8/evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log`
  — targeted test run (the changed test only): 1 passed, 0 failed.
- `.beads/vb-815l8/evidence/cargo_test_vb_runtime_recovery.log` —
  `cargo test -p vb_runtime --lib recovery`: 13 passed, 0 failed (no
  regression).
- `.beads/vb-815l8/evidence/cargo_test_vb_runtime_lib.log` —
  `cargo test -p vb_runtime --lib`: 1807 passed, 0 failed (no
  regression).
- `.beads/vb-815l8/evidence/cargo_check_workspace_tests.log` —
  `cargo check -p velvet-ballistics-workspace-tests --all-targets
  --all-features`: exit=0.