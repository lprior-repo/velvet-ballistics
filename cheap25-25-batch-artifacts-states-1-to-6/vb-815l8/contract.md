# Contract — vb-815l8

- bead_id: vb-815l8
- title: Tests: replace tautological recovery fault-tolerance assertion (P1 bug)
- controller: femdation
- state: 3 (rust-contract)
- authored_at: 2026-07-01
- bead_scope: TEST-ONLY; one-line assertion replacement + one-line import + comment cleanup

## 1. Contract Clauses

### C-1 — Runtime frame hydration rejects every seed

- **Statement:** For any value `seed: RecoveryFrameSeed`, `DurableFrameRecoveryBoundary::from_seed(seed).hydrate_run_frame()` returns `Err(RuntimeError::InvalidRecoveryHydration)`.
- **Why:** `RecoveryCannotResumeState::from_seed` (`crates/vb_storage/src/recovery/types.rs:949-957`) unconditionally applies `mark_missing_components(MissingRunStateComponents::ALL)`, so `is_resumable()` is always false; `reject_unsupported_live_frame_state` (`crates/vb_runtime/src/recovery.rs:109-115`) translates that into `Err(RuntimeError::InvalidRecoveryHydration)`.
- **Tested at:** `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` (8 unit-test sites).
- **Locked in by replacement at:** `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79`.

### C-2 — Boundary seed validation is invariant, not permissive

- **Statement:** The runtime boundary is not permissive on empty seeds, on seeds with `unsupported == SUPPORTED`, or on seeds with `step_count == 0`. The two false claims in the comments at lines 75-78 of the target file contradict `RecoveryResumeStatus::CannotResume` (`crates/vb_runtime/src/recovery.rs:41-50`) and must be removed.
- **Why:** The `RecoveryResumeStatus` enum has no `Resumable` variant by design (lines 41-50); a frame seed alone never carries the full `RunState`. The boundary must reflect this.
- **Locked in by comment cleanup at:** `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:75-78`.

### C-3 — Test uses typed assertion, not tautological `is_ok() || is_err()`

- **Statement:** The replacement assertion must be `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")` (Pattern A in `codebase-map.md` §1.3) or `assert!(matches!(result, Err(RuntimeError::InvalidRecoveryHydration)))` (Pattern B). Pattern A is preferred and matches the canonical style at `crates/vb_runtime/src/recovery/tests.rs:55-57`.
- **Why:** `PartialEq for RuntimeError` is exact by unit tag (`crates/vb_runtime/src/error/equality.rs:3-28`). A typed assertion discriminates `Ok(_)` from `Err(InvalidRecoveryHydration)` and from other `Err(_)` variants.

### C-4 — Import is added at lines 7-13

- **Statement:** `use vb_runtime::RuntimeError;` is added to the import block at lines 7-13 of the target file, matching the precedent at `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13`.
- **Why:** `vb_runtime` is already a dev-dependency at `crates/workspace_tests/Cargo.toml:43`.

## 2. Exact Required Edits

### 2.1 Lines 7-13 (import block)

Add to the import block:

```rust
use vb_runtime::RuntimeError;
```

Place after the existing `use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};` (line 8) and before `use vb_storage::recovery::{...};` (line 9-13). Style matches `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13`.

### 2.2 Lines 75-78 (comment block; replace the two false claims)

Replace:

```rust
    // Hydration should succeed because the seed itself is valid (corrupt snapshot
    // is a storage-layer concern; the boundary only validates the seed shape).
```

with:

```rust
    // Hydration must reject: a frame seed alone never carries the full RunState
    // (see RecoveryResumeStatus::CannotResume invariant at
    // crates/vb_runtime/src/recovery.rs:41-50).
```

Replace:

```rust
    // A seed with step_count=0 and no workflow may still be a valid empty-run seed.
```

with:

```rust
    // RecoveryCannotResumeState::from_seed unconditionally marks every
    // MissingRunStateComponents::ALL flag true
    // (crates/vb_storage/src/recovery/types.rs:949-957), so is_resumable() is
    // always false and the boundary always rejects the seed.
```

### 2.3 Line 79 (assertion)

Replace:

```rust
    assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed
```

with:

```rust
    assert_eq!(
        result,
        Err(RuntimeError::InvalidRecoveryHydration),
        "durable frame hydration must reject any frame seed: a frame seed alone \
         never carries the full RunState, so cannot_resume_state().is_resumable() \
         is never empty",
    );
```

## 3. Out Of Scope (Explicit)

| Excluded | Reason |
|----------|--------|
| Renaming the test `recovery_from_corrupt_snapshot_sequence_is_detected`. | Test rename is a behavior-test rewrite, not a contract assertion fix; flagged for `test-writer` follow-up. |
| Replacing the test body with a true storage-corrupt-snapshot test. | Different contract surface; separate bead. |
| Adding proptest, Kani, fuzz, or auxiliary test cases. | Out of scope per `codebase-map.md` §5 Q3. |
| Splitting the 359-line test file. | Source-length exception unchanged; one-line change does not affect the exception. |
| Modifying production code in `crates/vb_runtime/`, `crates/vb_storage/`, `crates/vb_core/`. | This is a test-only fix. |
| Modifying `Cargo.toml`, build wiring, source-length exceptions. | No build-impact. |
| The five out-of-scope tautological assertions listed in `codebase-map.md` §7. | Covered by other beads. |

## 4. Acceptance Criteria

- [ ] `use vb_runtime::RuntimeError;` is present at lines 7-13.
- [ ] The two false comments at lines 75-76 and 78 are replaced with the invariant-referencing comments in §2.2.
- [ ] Line 79 contains the typed `assert_eq!` from §2.3 (Pattern A).
- [ ] No other lines are modified.
- [ ] The file compiles under `holzman-rust` source lint.
- [ ] `cargo test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected` passes (test-writer to confirm).

## 5. Open Contract Questions

1. **Test-intent clarification (flagged in `domain-model.md` §8 Q1).** The test name implies storage-corrupt-snapshot detection; the body asserts boundary rejection. The contract is correct for the body. If the author intended corrupt-snapshot storage detection, a body rewrite is needed — out of scope for this bead.
2. **No source-length impact.** Adding one `use` and replacing one assertion adds ~2 net lines; the file remains on the over-300-line exception list (`split-or-retire-before-release`). No split required.

## 6. Downstream Routing

- **test-writer:** Implement the exact edits in §2. Add `use vb_runtime::RuntimeError;` to imports. Replace comments at lines 75-78. Replace the assertion at line 79.
- **test-reviewer:** Confirm the replacement matches Pattern A exactly, the comments reference the right invariants, and no other lines are touched.
- **black-hat-reviewer:** Confirm no new tautological assertion is introduced, no production code is mutated, and the assertion is typed (not boolean).
- **proof-planner:** Mark Rust-local lane only; no Verus/Kani/Flux/proptest/fuzz obligations for this bead (the contract is locked in by the existing 8 unit-test sites at `vb_runtime/src/recovery/tests.rs:55-57, 119-122, ...`).