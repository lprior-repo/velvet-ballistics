# Proof Writer Report — vb-rqmw

## Session Summary

**Date:** 2026-05-23
**Bead:** vb-rqmw
**State:** 5 (Proof Execution)
**Verifier:** Verus 0.2026.05.05.d03e906

## Obligations Completed

| ID | File | Action | Status |
|----|------|--------|--------|
| PO-001 | `step_state_machine.rs` | FIX | ✅ 16 verified |
| PO-002 | `signals_invariant.rs` | NO_CHANGE | ✅ 19 verified |
| PO-003 | `signals_try_take.rs` | NO_CHANGE | ✅ 6 verified |
| PO-004 | `run_loop_termination.rs` | NO_CHANGE | ✅ 7 verified |
| PO-005 | `vb_cli_commands_journal_trace.rs` | FIX | ✅ 4 verified |
| PO-006 | `budget_bounded.rs` | FIX | ✅ 15 verified |
| PO-007 | `idempotency_replay_tracker.rs` | FIX | ✅ 8 verified |
| PO-008-01 | `accepted_artifact_admission_decision.rs` | BIND | ✅ 10 verified |
| PO-008-02 | `accepted_envelope_model.rs` | BIND | ✅ 8 verified |
| PO-008-03 | `accepted_run_atomic_admission.rs` | BIND | ✅ 6 verified |
| PO-008-04 | `admission_artifact_model.rs` | BIND | ✅ 6 verified |
| PO-008-05 | `capability_artifact_model.rs` | BIND | ✅ 8 verified |

**Total:** 12 obligations, 12 completed, 0 failed

## Fixes Applied

### PO-001: step_state_machine.rs — 39 by(compute) removed

**Problem:** Proof functions used `by(compute)` which reduces proofs to computational reduction, making them vacuous.

**Fix:** Replaced all `by(compute)` usages with explicit proof reasoning:
- `proof_idempotent_remark_allowed`: Direct assertion of `current == current`
- `proof_terminal_blocks_outward`: Match on terminal states showing `non_idempotent_transition` returns false
- `proof_suspended_resumes_only_to_running`: Match on Waiting/Asking showing only Running is valid transition
- `proof_all_pairs`: 30 transition pair assertions proven directly from spec definitions
- EngineSignal lemmas: Assertions proven from `spec_mark_step_after_signal` definition

### PO-005: vb_cli_commands_journal_trace.rs — Unknown variant added

**Problem:** Rust `trace_one` has `#[non_exhaustive]` JournalEvent with catch-all `_ => Unknown` case, but Verus spec claimed exhaustive 18-variant matching.

**Fix:**
- Added `Unknown` variant to `SpecJournalEvent`
- Added `Unknown` case to `spec_trace_one` producing `SpecTraceEntry { event_type: "Unknown", ... }`
- Updated `proof_trace_one_variant_coverage` to include `Unknown` case
- Fixed `by(compute)` in `proof_trace_one_deterministic`

### PO-006: budget_bounded.rs — Overflow model corrected

**Problem:** Spec used `Option<int>` with `None` for overflow, but Rust returns `Err(WorkflowError::StepCountOverflow)`.

**Fix:**
- Added `SpecWorkflowError::StepCountOverflow` enum variant
- Changed `checked_add`, `checked_mul`, `checked_compose`, `checked_repeat` to return `Result<int, SpecWorkflowError>`
- Updated `spec_count_total_steps_result` to use `Result`
- Updated proof lemmas: `proof_overflow_add_returns_error`, `proof_overflow_mul_returns_error`, `proof_unknown_factor_rejects`, `proof_nested_overflow_rejects`

### PO-007: idempotency_replay_tracker.rs — HashSet model

**Problem:** Spec used abstract boolean `resolved`/`scheduled` flags instead of actual `HashSet<(ActionId, StepIdx)>` tracking.

**Fix:**
- Replaced abstract booleans with `Set<(int, int)>` for `completed` and `failed`
- Added `spec_is_resolved`, `spec_mark_completed`, `spec_mark_failed`, `spec_retry_allowed` using set operations
- Rewrote all proof lemmas to use set membership (`contains`) instead of boolean flags
- Added `proof_mark_completed_preserves_other_entries` showing insert doesn't affect other pairs

## Binding Sections Added (PO-008)

For each orphaned spec, added a BINDING comment block documenting the Rust type binding:

| Spec | Rust Type | Location |
|------|----------|----------|
| `accepted_artifact_admission_decision.rs` | `ArtifactEnvelopeError` | `crates/vb_runtime/src/admission.rs:24` |
| `accepted_envelope_model.rs` | `RecordEnvelope` | `crates/vb_storage/src/types.rs:183` |
| `accepted_run_atomic_admission.rs` | `RunAdmission` | `crates/vb_runtime/src/admission.rs:78` |
| `admission_artifact_model.rs` | `AcceptedArtifact` | `crates/vb_storage/src/admission.rs:133` |
| `capability_artifact_model.rs` | `Capability` | `crates/vb_core/src/capability.rs:10` |

## Findings

### No Change Required (PO-002, PO-003, PO-004)

Upon inspection, `signals_invariant.rs`, `signals_try_take.rs`, and `run_loop_termination.rs` do NOT contain `by(compute)` usages. The proof functions have proper `requires`/`ensures` contracts. All three files verify successfully without modification.

This discrepancy between the obligation description and actual file contents suggests either:
1. The files were already fixed prior to this session, or
2. The obligation descriptions were misaligned

## Commands Run

```bash
cargo verus verification/verus/step_state_machine.rs        # 16 verified
cargo verus verification/verus/signals_invariant.rs        # 19 verified
cargo verus verification/verus/signals_try_take.rs        # 6 verified
cargo verus verification/verus/run_loop_termination.rs    # 7 verified
cargo verus verification/verus/vb_cli_commands_journal_trace.rs  # 4 verified
cargo verus verification/verus/budget_bounded.rs          # 15 verified
cargo verus verification/verus/idempotency_replay_tracker.rs  # 8 verified
cargo verus verification/verus/accepted_artifact_admission_decision.rs  # 10 verified
cargo verus verification/verus/accepted_envelope_model.rs  # 8 verified
cargo verus verification/verus/accepted_run_atomic_admission.rs  # 6 verified
cargo verus verification/verus/admission_artifact_model.rs  # 6 verified
cargo verus verification/verus/capability_artifact_model.rs  # 8 verified
```

## Blockers

None. All obligations completed successfully.

## Trusted Boundaries

| Bound | Description |
|-------|-------------|
| SpecStepState | 9 finite variants (Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled) |
| validate_transition | Total function covering all state pairs |
| JournalEvent | Storage layer validates known variants; Unknown is forward-compatibility fallback |
| WorkflowError::StepCountOverflow | Only overflow error modeled; u64 MAX bound enforced |
| ActionReplayTracker | HashSet semantics (completed ∪ failed) = resolved set |
| RecordEnvelope | Schema version 1 required; sequence provides total ordering |
| RunAdmission | Artifact digest binding to admitted artifact |
| AcceptedArtifact | Policy digest binding to resource contract |
| Capability | Name length bounded [0, 128]; exact match required |

## Artifacts Produced

1. `trusted-base-ledger.jsonl` — One row per obligation with verification results
2. `proof-writer-report.md` — This report

## Next Steps

- State 6: Black-hat review
- State 7: Evidence packaging
- State 8: Landing
