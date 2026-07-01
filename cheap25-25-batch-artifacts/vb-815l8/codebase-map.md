# Codebase Map — vb-815l8

- bead_id: vb-815l8
- title: Tests: replace tautological recovery fault-tolerance assertion (P1 bug)
- description: A recovery test contains a tautological assertion (`assert!(result.is_ok() || result.is_err())` or similar). Replace it with an exact contract assertion that distinguishes success from a specific typed error.
- controller: femdation
- current_state: 2 (explore scout)
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8
- authored_at: 2026-07-01

## 1. The P1 Bug (single site, fully scoped)

### 1.1 The tautological assertion

File: `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`
Line: 79 (within test `recovery_from_corrupt_snapshot_sequence_is_detected`, line 46)

```rust
let result = boundary.hydrate_run_frame();
// A seed with step_count=0 and no workflow may still be a valid empty-run seed.
assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed
```

This is the ONLY tautological `is_ok() || is_err()` assertion inside any test directly related to recovery fault tolerance. Other matches at
`crates/workspace_tests/tests/integration_compile_error_message_quality.rs:{238,263,286}`,
`crates/vb_core/src/verification/kani/kani_choose_replay.rs:{337}`,
`crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs:{176}`,
`crates/vb_core/src/budget/tests.rs:{1306}`,
`crates/vb_compile/src/kani_foreach_parity.rs:{471}` are *out of scope* for this bead (covered by other beads).

### 1.2 Why the comment is also wrong

The seed under test (lines 50-72) has `UnsupportedRecoveryState::SUPPORTED` (all
four flags false), but `RecoveryCannotResumeState::from_seed` (storage layer)
**unconditionally** applies `mark_missing_components(MissingRunStateComponents::ALL)`,
so every `*_missing` flag is set true and `is_resumable()` returns `false`.
That means `Reject_unsupported_live_frame_state` (runtime) returns
`Err(RuntimeError::InvalidRecoveryHydration)` at line 113 of
`crates/vb_runtime/src/recovery.rs` BEFORE the empty-seed `RunFrame::new`
path runs. The boundary is NOT permissive on this seed; the comment lies and
the tautological assertion masks that lie.

A second, independent invariant triggers even if the resumability check were
relaxed: `RunFrame::new` (in
`crates/vb_core/src/frame/parts/impl_001_construct.rs`, line 10-14) rejects
`step_count == 0` with `CoreError::InvalidCompiledWorkflow { reason:
"step_count_zero" }` which `empty_recovered_frame` would translate into
`Err(RuntimeError::InvalidRecoveryHydration)` (line 124 of
`crates/vb_runtime/src/recovery.rs`).

Either gate independently forces the same typed outcome; both contracts
agree. The precise replacement assertion is therefore stable regardless of
which gate the test author picks as the "primary" contract surface.

### 1.3 Exact required fix (no scope expansion)

Replace:
```rust
assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed
```
with either (pattern A — preferred, matches `crates/vb_runtime/src/recovery/tests.rs:{121,172,...}` and `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:{268-272}`):
```rust
assert_eq!(
    result,
    Err(RuntimeError::InvalidRecoveryHydration),
    "durable frame hydration must reject any frame seed: a frame seed alone never carries the full RunState, so cannot_resume_state() is never empty"
);
```
or (pattern B — required if test is renamed to keep test author's intent that
this is a corrupt-snapshot detection test):
```rust
assert!(
    matches!(result, Err(RuntimeError::InvalidRecoveryHydration)),
    "durable frame hydration must reject a frame seed with no full-RunState evidence"
);
```

The replacement requires adding one import to the test file at line 7-13:
```rust
use vb_runtime::RuntimeError;
```
(matches the import line used in `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13`).

No other imports, no other lines, no new helpers, no new test cases.

## 2. Production Code Touched By The Contract Being Asserted

### 2.1 Runtime recovery boundary (the unit under test)

- `crates/vb_runtime/src/recovery.rs:30-39` — `trait RuntimeRecoveryBoundary`
  with `summary()`, `resume_status()`, `hydrate_run_frame()`.
- `crates/vb_runtime/src/recovery.rs:51-57` — `enum RecoveryResumeStatus`
  (no `Resumable` variant by design; see comments at lines 41-50).
- `crates/vb_runtime/src/recovery.rs:60-107` — `struct DurableFrameRecoveryBoundary` impl.
- `crates/vb_runtime/src/recovery.rs:99-106` — `hydrate_run_frame` calls
  `reject_unsupported_live_frame_state`, `empty_recovered_frame`,
  `apply_recovered_steps`, `apply_recovered_slots`, `apply_recovered_pc`.
- `crates/vb_runtime/src/recovery.rs:109-115` —
  `reject_unsupported_live_frame_state` returns
  `Err(RuntimeError::InvalidRecoveryHydration)` when `cannot_resume_state().is_resumable()` is false.
- `crates/vb_runtime/src/recovery.rs:117-125` — `empty_recovered_frame`
  maps `RunFrame::new` failure to `Err(RuntimeError::InvalidRecoveryHydration)`.
- `crates/vb_runtime/src/recovery.rs:127-148` — step/slot/pc appliers, all
  return `Err(RuntimeError::InvalidRecoveryHydration)` on invalid state.

### 2.2 `RuntimeError` definition

- `crates/vb_runtime/src/error/mod.rs:72-73` — `RuntimeError::InvalidRecoveryHydration`
  variant definition.
- `crates/vb_runtime/src/error/mod.rs:7` — `#[non_exhaustive] enum RuntimeError`.
- `crates/vb_runtime/src/error/equality.rs:3-7,28` —
  `impl PartialEq` using `runtime_error_unit_tag`; tag 10 is
  `InvalidRecoveryHydration`, so `assert_eq!` against
  `Err(RuntimeError::InvalidRecoveryHydration)` IS a typed assertion.
- `crates/vb_runtime/src/error/equality.rs:212` — `impl Eq`.
- `crates/vb_runtime/src/error/diagnostics.rs:75-78` — diagnostic code
  wiring (test should not be affected).
- `crates/vb_runtime/src/error/display.rs:29` — display string
  "invalid recovery frame hydration".
- `crates/vb_runtime/src/lib.rs:92` — re-exports
  `pub use error::{RuntimeError, RuntimeResult}`.

### 2.3 Storage layer cannot-resume projection (drives the test outcome)

- `crates/vb_storage/src/recovery/types.rs:541-1098` — module covering all
  recovery-state flags and the `from_seed` projection.
- `crates/vb_storage/src/recovery/types.rs:610-633` — `enum RecoveredStepState`.
- `crates/vb_storage/src/recovery/types.rs:634-643` — `struct RecoveredSlotEntry`.
- `crates/vb_storage/src/recovery/types.rs:645-655` — `struct RecoveredPendingAction`.
- `crates/vb_storage/src/recovery/types.rs:660-720` — `UnsupportedRecoveryState`
  with const `SUPPORTED` (line 667) — every flag false.
- `crates/vb_storage/src/recovery/types.rs:730-810` — `struct RecoveryFrameSeed`
  with fields `summary`, `first_step`, `step_count`, `slot_count`, `pc`,
  `steps`, `slots`, `pending_actions`, `unsupported`.
- `crates/vb_storage/src/recovery/types.rs:785-810` —
  `struct MissingRunStateComponents` with const `ALL` (line 809) used by
  `from_seed`.
- `crates/vb_storage/src/recovery/types.rs:949-957` —
  `RecoveryCannotResumeState::from_seed` UNCONDITIONALLY marks all
  `*_missing` flags true via
  `state.mark_missing_components(MissingRunStateComponents::ALL)`.
- `crates/vb_storage/src/recovery/types.rs:969-992` —
  `mark_missing_components` selectively sets each `*_missing` flag.
- `crates/vb_storage/src/recovery/types.rs:1025-1039` —
  `RecoveryCannotResumeState::is_resumable()` returns
  `false` for the test seed (all 13 flags are `true`).
- `crates/vb_storage/src/recovery/types.rs:1202-1214` —
  `RecoveryFrameSeed::cannot_resume_state` and `is_resumable` delegators.

### 2.4 Frame construction (secondary contract surface)

- `crates/vb_core/src/frame/parts/impl_001_construct.rs:3-31` —
  `RunFrame::new` rejects `step_count == 0` with
  `CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }`
  (line 10-14). This is the "second gate" that would still produce
  `Err(RuntimeError::InvalidRecoveryHydration)` via the
  `empty_recovered_frame` mapper (recovery.rs:117-125) even if a future
  refactor removes the resumability gate.

## 3. Reference Test Patterns Already In The Repo

These show the canonical typed-assertion style for the same contract surface.
The replacement in §1.3 should follow pattern A so it matches these references.

- `crates/vb_runtime/src/recovery/tests.rs:55-57` —
  `assert_eq!(boundary.hydrate_run_frame(), Err(RuntimeError::UnsupportedFullRecoveryHydration))`
- `crates/vb_runtime/src/recovery/tests.rs:119-122` — frame_minimal_state rejects as
  `Err(RuntimeError::InvalidRecoveryHydration)`.
- `crates/vb_runtime/src/recovery/tests.rs:170-173` — inconsistent-seed reject.
- `crates/vb_runtime/src/recovery/tests.rs:212-215` — unsupported-action-payloads reject.
- `crates/vb_runtime/src/recovery/tests.rs:269-272` — slot_value_and_taint reject.
- `crates/vb_runtime/src/recovery/tests.rs:294-297` — summary-only reject.
- `crates/vb_runtime/src/recovery/tests.rs:359-362` — factory frame-seed reject.
- `crates/vb_runtime/src/recovery/tests.rs:489-492` — pending-action reject.
- `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13` —
  `use vb_runtime::RuntimeError;` (existing import pattern).
- `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:267-272` —
  `let result = boundary.hydrate_run_frame(); assert!(matches!(result, Err(RuntimeError::InvalidRecoveryHydration)));`
  (existing matching-pattern assertion in the same crate).
- `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:245-273` —
  full surrounding "minimum seed, expect typed reject" test.

## 4. Cargo and Build Wiring

- `crates/workspace_tests/Cargo.toml:1-46` — package
  `velvet-ballistics-workspace-tests` with `vb_runtime` as a `dev-dependency`
  (line 43), so `use vb_runtime::RuntimeError;` is already authorized.
- `crates/workspace_tests/Cargo.toml:48-287` — `[[test]]` entries. The
  `integration_runtime_storage_fault_tolerance.rs` file is NOT explicitly
  listed but `autotests` is not set to `false`, so Cargo auto-discovers
  all `tests/*.rs` and this file IS compiled into the test binary.
- `Cargo.toml:1-11` — workspace members; the relevant crates are
  `vb_runtime`, `vb_storage`, `vb_core`, `workspace_tests`.
- The `@/home/lewis/.agents/skills/holzman-rust/SKILL.md` source lint
  pipeline does NOT consider the tautological assertion an error on its own
  (it is a `assert!(...)` of a `bool`, not a panic surface) but the bead
  is flagged as "P1 bug" because the test silently passes for any
  hydration outcome — including regressing the boundary to silently
  returning `Err(...)` for valid seeds or `Ok(frame)` for invalid seeds.

## 5. Open Questions For Downstream Owners

1. **Test-intent clarification.** The test name
   `recovery_from_corrupt_snapshot_sequence_is_detected` implies the author
   wanted to verify storage-layer corrupt-snapshot detection (which would
   produce `Err(RecoveryError::CorruptSnapshot { run, seq })` from
   `recover_runtime_frame_seed_from_events` /
   `crates/vb_storage/src/recovery/replay/summary/derive.rs:69`). The
   current test never calls that entry point; it constructs a
   `RecoveryFrameSeed` manually and feeds it to the boundary. If the author
   intended corrupt-snapshot storage detection, the test needs a body
   rewrite, not just an assertion replacement — flag this in `rust-contract`
   review. Otherwise, treat the test as "boundary rejects frame seed with
   SUPPORTED unsupported state and zero-step seed" and apply the
   §1.3 replacement only.

2. **Comment cleanup.** The two lines of comment at
   `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:75-79`
   ("boundary only validates the seed shape", "boundary is permissive on
   empty seed") are both incorrect per the production code in
   `crates/vb_runtime/src/recovery.rs:109-115`. The replacement assertion
   should be paired with a comment that references the
   `RecoveryResumeStatus::CannotResume` invariant from
   `crates/vb_runtime/src/recovery.rs:41-50`.

3. **No new dependencies, no new features, no new tests.** The bead is
   scoped to one line replacement plus optionally an import line. Do not
   add proptests, Kani harnesses, or auxiliary test cases — those belong
   to separate beads.

4. **Source-length exception unchanged.** The file is 12.3K / 359 lines (source-length-exceptions.txt baseline 346 predates current growth)
   and is on the over-300-line exception list
   (`.config/source-length-exceptions.txt:200`,
   `vb-jpq7.47|split-or-retire-before-release`). One additional `use` and
   one additional `assert_eq!` does not change this status. No split is
   required for this bead.

## 6. Risk Tags

- `parser/codec` — the recovery seed JSON/postcard decode/serialize path
  shapes what values may appear in a manually-constructed seed (the test
  exercises that surface area).
- `persistence` — frame hydration is the recovery-time rebuilder of a
  prior run's runtime state from journaled events; the contract being
  asserted is a persistence boundary.
- `public API` — the test exercises the public
  `DurableFrameRecoveryBoundary::hydrate_run_frame` method and the
  public `vb_runtime::RuntimeError` enum.
- `user-visible behavior` — the bead is classified P1 because a tautological
  test passes regardless of the boundary's actual behavior, masking
  regressions that an end user would see as run-time hydration failures.

## 7. Excluded Paths (Out Of Scope)

- `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:{238,263,286}` — different domain (compile error taxonomy, not recovery); covered by other beads.
- `crates/vb_core/src/verification/kani/kani_choose_replay.rs:{337}` — Kani harness `cover!` statement, not a recovery assertion; out of scope.
- `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs:{176}` — proptest for compile source, not recovery; out of scope.
- `crates/vb_core/src/budget/tests.rs:{1306}` — budget-error proptest; out of scope.
- `crates/vb_compile/src/kani_foreach_parity.rs:{471}` — Kani harness parity check; out of scope.

## 8. Unmapped / Unknown

- The `bd` dolt server is offline in this isolated workspace
  (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-815l8`) so
  `bd show vb-815l8` could not pull bead metadata. The bead title and
  description used here come from the controller prompt, not from the
  authoritative Dolt row.
- The exact test count of `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`
  under the current `autotests` behavior is UNKNOWN because the crate
  contains explicit `[[test]]` entries that override auto-discovery
  selectively; whether Cargo runs this file depends on the
  `cargo nextest` invocation pattern used by the CI pipeline (see
  `xtask/src/proof.rs:146`). Treat as MISSING and re-verify after the
  GoToTestPlan run.
