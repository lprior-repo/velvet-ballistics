# Domain Model — vb-815l8

- bead_id: vb-815l8
- title: Tests: replace tautological recovery fault-tolerance assertion (P1 bug)
- controller: femdation
- state: 3 (rust-contract)
- bead_scope: TEST-ONLY, Rust-local, no production-code mutation, no Verus/Kani/Flux/proptest/fuzz lanes
- authored_at: 2026-07-01

## 1. Ubiquitous Language

| Term | Definition | Source of truth |
|------|------------|-----------------|
| Recovery frame seed | A `RecoveryFrameSeed` value assembled by storage from journaled events that describes a prior run's frame-level state without guaranteeing it can resume live execution. | `crates/vb_storage/src/recovery/types.rs:730-810` |
| Cannot-resume state | A typed witness `RecoveryCannotResumeState` enumerating the precise runtime-boundary components that are not recoverable from the seed (workflow, store, action attempts, admission, collect states, action contracts, action ABI digests, …). | `crates/vb_storage/src/recovery/types.rs:949-1039` |
| Resume status | The runtime-visible decision projected from a seed, with two variants `CannotResume(RecoveryCannotResumeState)` and `SummaryOnly`. `Resumable` is intentionally not a variant because a frame seed never carries the full `RunState`. | `crates/vb_runtime/src/recovery.rs:41-57, 90-97` |
| Live-frame hydration | The act of rebuilding a live `RunFrame` from a seed via `DurableFrameRecoveryBoundary::hydrate_run_frame()`. Always returns a typed `RuntimeError` on any gate failure. | `crates/vb_runtime/src/recovery.rs:99-106` |
| Hydration gate | One of the typed reject sites (`reject_unsupported_live_frame_state`, `empty_recovered_frame`, `apply_recovered_*`) that translates a structural storage or core error into `RuntimeError::InvalidRecoveryHydration`. | `crates/vb_runtime/src/recovery.rs:109-148` |
| Tautological assertion | An assertion whose boolean outcome is invariant across the entire `Result<_, _>` codomain (e.g., `assert!(result.is_ok() || result.is_err())`). Passes for every possible runtime behavior, including regressions. | Convention; the test at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79` is the single P1 instance in scope. |

## 2. Entities (not constructed in this bead; surveyed for contract completeness)

- `RuntimeError` (`#[non_exhaustive] enum`) — defined at `crates/vb_runtime/src/error/mod.rs:7`. The variant `RuntimeError::InvalidRecoveryHydration` is the unit-variant tag 10 per `crates/vb_runtime/src/error/equality.rs:3-7,28`.
- `RuntimeResult<T>` — re-export at `crates/vb_runtime/src/lib.rs:92`.
- `RecoveryFrameSeed` — owned struct at `crates/vb_storage/src/recovery/types.rs:730-810`.
- `UnsupportedRecoveryState` — bit-flag struct; const `SUPPORTED` has every flag false (`crates/vb_storage/src/recovery/types.rs:660-720`, line 667).
- `RecoveryCannotResumeState` — bit-mask struct; const `ALL` applies every missing flag (`crates/vb_storage/src/recovery/types.rs:785-810`, line 809).

## 3. Value Objects (test surfaces, not owned by this bead)

- `RunId::new(9002)` — the only `RunId` literal that exercises the target test (`integration_runtime_storage_fault_tolerance.rs:49`).
- `EventSeq::ZERO` — placeholder sequence value used twice in the test seed (lines 53, 54).
- `WorkflowDigest::from_bytes([0x1F; 32])` — synthetic 32-byte digest carried by `summary.workflow` (`integration_runtime_storage_fault_tolerance.rs:55`).

## 4. Commands and Events (not exercised; surveyed for completeness)

The target test does not call `recover_runtime_frame_seed_from_events`; it constructs a `RecoveryFrameSeed` manually and hands it to `DurableFrameRecoveryBoundary::from_seed`. The storage-layer `JournalEvent` replayer (`RecoveryError::CorruptSnapshot { run, seq }`) is therefore not in the call graph of this test — its name `recovery_from_corrupt_snapshot_sequence_is_detected` is misleading; the test asserts boundary behavior, not storage-corrupt-snapshot detection.

## 5. Policies (boundary-level)

| Policy | Where enforced | Why |
|--------|---------------|-----|
| Boundary never produces `Ok(RunFrame)` for any `RecoveryFrameSeed`. | `reject_unsupported_live_frame_state` at `crates/vb_runtime/src/recovery.rs:109-115`. | `from_seed` unconditionally marks every `*_missing` flag true, so `is_resumable()` is always false; the boundary must reject every seed. |
| Empty seed (zero-step, all-zero summary) is rejected, not "permissive". | `empty_recovered_frame` at `crates/vb_runtime/src/recovery.rs:117-125` and `RunFrame::new` step-count-zero reject at `crates/vb_core/src/frame/parts/impl_001_construct.rs:10-14`. | Either gate alone forces `Err(RuntimeError::InvalidRecoveryHydration)`. |
| Boundary reflects the `RecoveryResumeStatus` invariant (no `Resumable` variant). | `crates/vb_runtime/src/recovery.rs:41-57, 90-97`. | A `RecoveryFrameSeed` alone never carries the full `RunState`. |

## 6. Invariants (test-asserted)

| ID | Invariant | Where locked in by the replacement assertion |
|----|-----------|---------------------------------------------|
| INV-RT-HYDRATE-001 | `DurableFrameRecoveryBoundary::hydrate_run_frame()` returns `Err(RuntimeError::InvalidRecoveryHydration)` for any `RecoveryFrameSeed`. | New `assert_eq!` at `integration_runtime_storage_fault_tolerance.rs:79`. |

## 7. Forbidden States (after the test fix)

| Forbidden state | Why unrepresentable after the fix |
|-----------------|-----------------------------------|
| The test passing under `Ok(frame)` for an invalid seed. | `assert_eq!(Err(InvalidRecoveryHydration))` cannot match `Ok(_)`. |
| The test passing under a different `RuntimeError` variant (e.g., `UnsupportedFullRecoveryHydration`). | `PartialEq` discriminates by unit tag 10 vs. 12 (`crates/vb_runtime/src/error/equality.rs:3-28`). |
| The test author re-introducing `assert!(result.is_ok() || result.is_err())` patterns. | The replacement is a typed `assert_eq!` and any re-introduction requires deliberate mutation. |

## 8. Open Domain Questions Flagged For Downstream Owners

1. **Test intent mismatch.** The test name `recovery_from_corrupt_snapshot_sequence_is_detected` implies storage-corrupt-snapshot detection, but the body constructs a `RecoveryFrameSeed` manually and exercises the runtime boundary. The replacement assertion is correct for the actual code path; if the test author intended storage-corrupt-snapshot detection, the body must be rewritten — but that is OUT OF SCOPE for this bead (only line 79 mutates). Flag for `test-writer` review.
2. **Misleading comments at lines 75-78.** The current comments claim "boundary only validates the seed shape" and "boundary is permissive on empty seed". Both contradict the production code at `crates/vb_runtime/src/recovery.rs:109-115` and the `RecoveryResumeStatus` invariant at `crates/vb_runtime/src/recovery.rs:41-50`. Comment cleanup is in scope for this bead (lines 75-78) and must reference the `RecoveryResumeStatus::CannotResume` invariant.
3. **No source-length impact.** Adding `use vb_runtime::RuntimeError;` and replacing one assertion adds ~2 lines; the file remains on the over-300-line exception list (`split-or-retire-before-release`). No split required.

## 9. Scope Boundary

This bead touches only:

- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs` lines 7-13 (one import added), lines 75-79 (comment cleanup + assertion replacement).

This bead does NOT touch:

- Production code in `crates/vb_runtime/`, `crates/vb_storage/`, `crates/vb_core/`.
- Other test files (the five out-of-scope files listed in `codebase-map.md` §7).
- `Cargo.toml`, build wiring, source-length exceptions.
- Any verifier artifact, behavior-test rewrite, Kani harness, proptest, fuzz target, or proof plan.