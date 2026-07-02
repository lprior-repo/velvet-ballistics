# Workflow Model — vb-815l8

- bead_id: vb-815l8
- scope: TEST-ONLY; one-line assertion + one-line import + comment cleanup at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:7-13, 75-79`
- authored_at: 2026-07-01

## 1. Workflow State Machine

This bead is test-only. The only workflow is the test execution flow for the single test `recovery_from_corrupt_snapshot_sequence_is_detected`. There is no production workflow mutated.

### 1.1 Test-execution states (in scope)

| State | Pre-condition | Action | Post-condition | Terminal? |
|-------|---------------|--------|----------------|-----------|
| SEED-BUILT | `use vb_runtime::RuntimeError;` imported; `RecoveryFrameSeed` constructed at lines 50-72 | `DurableFrameRecoveryBoundary::from_seed(seed)` (line 74) | `boundary: DurableFrameRecoveryBoundary` | no |
| HYDRATE-INVOKED | `boundary` constructed | `boundary.hydrate_run_frame()` (line 77) | `result: RuntimeResult<RunFrame>` | no |
| ASSERT-CHECKED | `result` returned | `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")` (new line 79) | test passes iff `result == Err(InvalidRecoveryHydration)` | YES |

### 1.2 Guard invariants

- Between SEED-BUILT and HYDRATE-INVOKED: `seed.unsupported == UnsupportedRecoveryState::SUPPORTED` (all four flags false). Verified by the seed construction at lines 50-72; not asserted but is a precondition of the replacement assertion.
- Between HYDRATE-INVOKED and ASSERT-CHECKED: `boundary` is the only handle to the seed; the seed's `cannot_resume_state()` has all 13 `*_missing` flags true because `RecoveryCannotResumeState::from_seed` at `crates/vb_storage/src/recovery/types.rs:949-957` always applies `mark_missing_components(MissingRunStateComponents::ALL)`. Therefore `boundary.hydrate_run_frame()` deterministically returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### 1.3 Outcomes

The single outcome is:

- `PASS`: `boundary.hydrate_run_frame() == Err(RuntimeError::InvalidRecoveryHydration)` and the assertion message is unused.

There are no `Ok(_)` outcomes and no other typed `Err(_)` outcomes reachable from this seed. Any `Ok(_)` would constitute a production-code regression at `crates/vb_runtime/src/recovery.rs:99-115`; any other `Err(_)` variant would constitute a regression at `crates/vb_runtime/src/recovery.rs:113-148`.

### 1.4 Terminal states

`recovery_from_corrupt_snapshot_sequence_is_detected` is terminal upon assertion completion. There is no cleanup phase. The test does not own any resource beyond stack-local values.

## 2. Production-Workflow Survey (read-only; for context)

The target test exercises the runtime-recovery production workflow indirectly. The workflow being asserted is:

```
RecoveryFrameSeed ──► DurableFrameRecoveryBoundary::from_seed ──► boundary
boundary ──► boundary.hydrate_run_frame() ──► reject_unsupported_live_frame_state(seed)?
                                                  ├── Ok(()) if is_resumable() else Err(InvalidRecoveryHydration)
                                                  ▼
                                            empty_recovered_frame(seed)?
                                                  ├── Ok(frame) if step_count > 0 else Err(InvalidRecoveryHydration)
                                                  ▼
                                            apply_recovered_steps
                                                  ▼
                                            apply_recovered_slots
                                                  ▼
                                            apply_recovered_pc
                                                  ▼
                                            Ok(frame)
```

For the target seed, the first gate (`reject_unsupported_live_frame_state`) returns `Err(InvalidRecoveryHydration)` because `is_resumable()` is false. Therefore the workflow terminates at gate 1.

The second gate (`empty_recovered_frame`) would independently terminate with `Err(InvalidRecoveryHydration)` if the first gate were relaxed, because `RunFrame::new` rejects `step_count == 0` (`crates/vb_core/src/frame/parts/impl_001_construct.rs:10-14`). Both gates agree on the typed outcome.

## 3. Workflow Hazards (temporal / sequencing)

| Hazard | Source | Mitigation in this bead |
|--------|--------|--------------------------|
| Test author adds a third assertion that re-introduces `result.is_ok() \|\| result.is_err()`. | Future regression. | Out of scope for this bead; flagged for `test-reviewer` and `black-hat-reviewer` follow-up. |
| `RuntimeError` gains a new variant with unit tag 10 by accident. | Refactor in `crates/vb_runtime/src/error/mod.rs` or `equality.rs`. | Out of scope; `equality.rs` uses unit tags, so any tag duplication would be caught by the existing `equality.rs` test suite. |
| `RecoveryCannotResumeState::from_seed` is changed to apply only a subset of `MissingRunStateComponents::ALL`. | Future storage-layer refactor. | Would invalidate the test; that refactor is a separate behavior change and must update this test. |

## 4. Workflow Diagram (textual)

```
[Test start]
   │
   ▼
[line 49] let run = RunId::new(9002);
   │
   ▼
[lines 50-72] let seed = RecoveryFrameSeed { ... unsupported: SUPPORTED };
   │
   ▼
[line 74] let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
   │
   ▼
[line 77] let result = boundary.hydrate_run_frame();
   │              │
   │              ▼
   │       ┌──────────────────────────────────────┐
   │       │ reject_unsupported_live_frame_state: │
   │       │   seed.cannot_resume_state().is_resumable() │
   │       │   = false (all 13 *_missing = true) │
   │       │   ⇒ Err(RuntimeError::InvalidRecoveryHydration) │
   │       └──────────────────────────────────────┘
   │              │
   ▼              ▼
[NEW line 79] assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...");
   │
   ▼
[Test PASS / FAIL]
```

## 5. Open Workflow Questions

None. The workflow is single-shot, deterministic, and the test has exactly one typed outcome. No out-of-band ordering, scheduling, or sequencing hazards exist for this bead.