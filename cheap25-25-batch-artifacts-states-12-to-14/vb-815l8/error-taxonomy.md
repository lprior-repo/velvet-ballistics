# Error Taxonomy — vb-815l8

- bead_id: vb-815l8
- scope: TEST-ONLY; one-line assertion replacement + one-line import + comment cleanup
- authored_at: 2026-07-01

## 1. Error Class Hierarchy (read-only; not constructed by this bead)

The target test asserts against a single typed error variant. The taxonomy below is the full recovery-error surface that bounds the assertion's reach.

### 1.1 `vb_runtime::RuntimeError` (`crates/vb_runtime/src/error/mod.rs:7`)

`#[non_exhaustive]` enum with `Debug + Clone`. The replacement assertion uses the unit variant `RuntimeError::InvalidRecoveryHydration` (line 73). Equality is implemented via unit tags in `crates/vb_runtime/src/error/equality.rs:3-28`; tag 10 corresponds to `InvalidRecoveryHydration`.

| Tag | Variant | Used by this test? |
|-----|---------|--------------------|
| 10 | `RuntimeError::InvalidRecoveryHydration` | YES — the asserted variant. |
| 12 | `RuntimeError::UnsupportedFullRecoveryHydration` | NO — used by `SummaryRecoveryBoundary::hydrate_run_frame` (`recovery.rs:189`), not exercised here. |
| others | `QueueFull`, `RunNotFound`, `RunAlreadyExists`, … | NO — out of scope for the recovery boundary. |

### 1.2 `vb_storage::recovery::RecoveryError` (not used by the replacement)

`RecoveryError::NoRecoveryData`, `RecoveryError::CorruptSnapshot`, `RecoveryError::FrameDimensionOverflow`, `RecoveryError::ReplayDivergence`, `RecoveryError::NonIdempotentActionBlocked`, `RecoveryError::WorkflowSourceDigestMismatch`, `RecoveryError::ActionAbiMismatch`, `RecoveryError::PolicyDigestMismatch`, `RecoveryError::TerminalStateMismatch` — all surveyed in `delivery-scope.jsonl` but unused by the replacement assertion. The target test's name (`recovery_from_corrupt_snapshot_sequence_is_detected`) is misleading; the body does NOT exercise `RecoveryError::CorruptSnapshot`.

### 1.3 `vb_core::errors::CoreError` (secondary; not used directly)

`CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }` is the secondary gate at `crates/vb_core/src/frame/parts/impl_001_construct.rs:10-14`. This error is translated by `empty_recovered_frame` (`crates/vb_runtime/src/recovery.rs:124`) into `RuntimeError::InvalidRecoveryHydration` before propagating to the boundary consumer. The replacement assertion therefore does not directly assert on `CoreError`.

## 2. The Replacement Assertion (typed error contract)

```
assert_eq!(
    boundary.hydrate_run_frame(),
    Err(RuntimeError::InvalidRecoveryHydration),
    "durable frame hydration must reject any frame seed: a frame seed alone never \
     carries the full RunState, so cannot_resume_state().is_resumable() is never empty",
);
```

- LHS: `RuntimeResult<RunFrame>`.
- RHS: `Err(RuntimeError::InvalidRecoveryHydration)`.
- Discrimination: `PartialEq for RuntimeError` is exact by unit tag (`equality.rs:3-28`); the assertion will fail with a `Debug` payload that names the actual error variant on mismatch.

## 3. Typed-Error Discrimination (positive vs. negative tests)

| Outcome | Discriminated by `assert_eq!(... , Err(InvalidRecoveryHydration))`? |
|---------|-----------------------------------------------------------------------|
| `Ok(RunFrame)` | NO — fails because `PartialEq<Result<_, _>>` is false. |
| `Err(RuntimeError::InvalidRecoveryHydration)` | YES — passes. |
| `Err(RuntimeError::UnsupportedFullRecoveryHydration)` | NO — fails because unit tags differ. |
| `Err(RuntimeError::Core { source: ... })` | NO — fails because structural fields differ. |
| `Err(RuntimeError::StorageJournalAppend { source: ... })` | NO — fails because structural fields differ. |
| `Err(vb_storage::recovery::RecoveryError::CorruptSnapshot { .. })` | NO — type mismatch (`RuntimeError` vs. `RecoveryError`); would fail to compile if a future maintainer attempted to insert it. |
| `panic!` | NO — different control-flow path; test fails by panic rather than assertion. |

## 4. Error-Message Contract

The replacement assertion includes a `&'static str` message that names the invariant: "durable frame hydration must reject any frame seed: a frame seed alone never carries the full RunState, so cannot_resume_state().is_resumable() is never empty". The message must:

- Reference the `RecoveryResumeStatus::CannotResume` invariant (`crates/vb_runtime/src/recovery.rs:41-50`).
- Reference the storage-layer `from_seed` marking (`crates/vb_storage/src/recovery/types.rs:949-957`).

## 5. Forbidden Error Patterns

The replacement assertion removes the only forbidden pattern in this test: `assert!(result.is_ok() || result.is_err())`. This pattern is a type-erased tautology and is forbidden in `crates/workspace_tests/tests/` per `AGENTS.md` source-lint expectations (the assertion itself does not fail `holzman-rust` because it is `assert!(bool)`, but it is a P1 testing defect because it masks regressions).

## 6. Error-Path Surface (read-only survey)

The recovery boundary surfaces errors at exactly four sites in `crates/vb_runtime/src/recovery.rs`:

| Site | Lines | Returns | Translation |
|------|-------|---------|-------------|
| `reject_unsupported_live_frame_state` | 109-115 | `Err(RuntimeError::InvalidRecoveryHydration)` | Direct variant. |
| `empty_recovered_frame` | 117-125 | `Err(RuntimeError::InvalidRecoveryHydration)` | `RunFrame::new` error → mapped. |
| `apply_recovered_slots` | 133-139 | `Err(RuntimeError::InvalidRecoveryHydration)` | `write_slot_with_taint` error → mapped. |
| `apply_recovered_pc` | 141-148 | `Err(RuntimeError::InvalidRecoveryHydration)` | Bounds check or `set_pc` error → mapped. |

For the target test seed, only site 1 is reached. Sites 2-4 are dead code for this seed but would each independently produce the same typed outcome if reached.

## 7. Open Taxonomy Questions

None. The replacement uses a single typed variant that is exact and discrimination-safe. There is no stringly-typed error, no `Box<dyn Error>`, no `unwrap`/`expect`/`panic` surface.