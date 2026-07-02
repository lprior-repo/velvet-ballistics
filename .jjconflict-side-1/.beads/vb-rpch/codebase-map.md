# CODEBASE MAP — vb-rpch

## Bead
- **id**: vb-rpch
- **title**: bdd: Durability and recovery acceptance scenarios
- **parent**: vb-hjvq (EPIC release: Full E2E BDD acceptance suite)
- **blocks**: vb-oewy (bdd: Full suite runner and evidence artifact contract)
- **depends_on**: vb-hxm0 (bdd: Executable behavior catalog), vb-ypnk (quality: Add evidence bundle)

## Research Sources Read
- `velvet-ballistics-MASTER.md` — durability profiles, Fjall persistence, journal replay semantics
- `crates/workspace_tests/tests/` — existing BDD test patterns
- `crates/vb_compile/tests/` — compile-time artifact tests
- `crates/vb_runtime/src/` — runtime admission and recovery boundary
- `crates/vb_storage/src/` — Fjall journal, recovery, replay modules

## Key Crates Touched

### vb_storage (PRIMARY)
- `crates/vb_storage/src/recovery/mod.rs` — Recovery module re-exports
- `crates/vb_storage/src/recovery/types.rs` — RecoveryError enum (16 variants)
- `crates/vb_storage/src/recovery/recover.rs` — High-level recovery orchestration
- `crates/vb_storage/src/recovery/replay/mod.rs` — Core replay logic
- `crates/vb_storage/src/recovery/replay/core.rs` — Event processing
- `crates/vb_storage/src/recovery/replay/summary.rs` — RecoveryFrameSeed building
- `crates/vb_storage/src/recovery/hydrate.rs` — Live-frame hydration
- `crates/vb_storage/src/recovery/hydrate_support.rs` — Hydration helpers
- `crates/vb_storage/src/journal/` — Fjall-backed journal (append, batch, core, parse, replay, source)
- `crates/vb_storage/src/admission.rs` — Durability gate admission (submit_artifact)
- `crates/vb_storage/tests/recovery_bdd_tests.rs` — 1918-line BDD test suite (B-001..B-020)
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` — Durability gate unit tests
- `crates/vb_storage/src/recovery/vb_h6ix_tests.rs` — Recovery integration tests
- `crates/vb_storage/src/recovery/tests.rs` — Unit tests for recovery types

### vb_runtime (SECONDARY)
- `crates/vb_runtime/src/admission.rs` — Runtime admission, load_accepted_artifact
- `crates/vb_runtime/src/recovery/` — DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary

### vb_core (SUPPORT)
- `crates/vb_core/src/action.rs` — ActionId, ActionTicket, verify_idempotency
- `crates/vb_core/src/run.rs` — RunId, RunFrame basics
- `crates/vb_core/src/workflow.rs` — CompiledWorkflow, WorkflowDigest

### vb_compile (TERTIARY)
- `crates/vb_compile/tests/` — Compile artifact tests

## Existing Coverage Inventory

### Recovery BDD Tests (recovery_bdd_tests.rs)
| Test | Scenario | Status |
|------|----------|--------|
| B-001a | header_binds_target_run_when_digests_match | PASS |
| B-001b | header_rejects_workflow_source_digest_mismatch | PASS |
| B-001c | header_rejects_compiled_ir_digest_mismatch | PASS |
| B-002 | full_journal_reconstructs_exact_pc_steps_slots_taint_terminal | PASS |
| B-003 | snapshot_plus_tail_reconstructs_frame_with_tail_overwrite | PASS |
| B-004 | empty_journal_returns_no_recovery_data | PASS |
| B-005 | corrupt_journal_record_returns_typed_storage_error | PASS |
| B-006 | action_aborts_are_replayed_with_exact_step_state | PASS |
| B-007 | non_idempotent_action_blocked_during_recovery | PASS |
| B-008 | journal_sequence_gap_returns_replay_divergence | PASS |
| B-009 | slot_value_recovery_hydrates_exact_tainted_frame | PASS |
| B-010 | verify_digests_detects_ir_digest_mismatch | PASS |
| B-011 | frame_dimension_overflow_returns_typed_error | PASS |
| B-012 | run_snapshot_persists_and_restores_frame | PASS |
| B-013 | snapshot_plus_tail_tail_event_ordering_preserved | PASS |
| B-014 | snapshot_tail_fact_erased_when_no_tail_event | PASS |
| B-015 | (see action_abi_mismatch_returns_typed_error) | IGNORED vb-ty9 |
| B-016 | (see policy_digest_mismatch_returns_typed_error) | IGNORED vb-ty9 |
| B-017 | terminal_state_mismatch_not_reachable_via_public_api | DEFERRED_GLOBAL |

### Durability Gate Tests (vb_2bok_durability_gate_tests.rs)
- `submit_artifact_relaxed_skips_gate_validation` — gate_count=0, durable=false
- `submit_artifact_journaled_enforces_both_gates` — gate_count=15, durable=false
- `submit_artifact_strict_enforces_gates_plus_syncall` — gate_count=15, durable=true
- Many more unit tests for policy tier behavior

### Recovery Unit Tests (vb_h6ix_tests.rs)
- `recovery_error_replay_divergence_carries_fields`
- `replay_divergence_on_out_of_order_steps`
- `replay_error_step_state_invariant`
- `recovered_step_entry_equality`

### vb_qi37_1_1_red_recovery_contract_test.rs
- Frame seed hydration tests
- Taint propagation in recovery
- Unsupported state detection

## Recovery Error Variants (from types.rs)
1. `Journal` — journal error during recovery
2. `WorkflowSourceDigestMismatch` — expected/found digests
3. `CompiledIrDigestMismatch` — expected/found digests
4. `ActionAbiMismatch` — action_id field (IGNORED vb-ty9)
5. `PolicyDigestMismatch` — step field (IGNORED vb-ty9)
6. `NonIdempotentActionBlocked` — action/step fields
7. `ReplayDivergence` — step/detail fields
8. `NoRecoveryData` — run field
9. `CorruptSnapshot` — run/seq fields
10. `TerminalStateMismatch` — expected/found strings
11. `FrameDimensionOverflow` — run field

## Durability Profiles (from MASTER.md)
- **Relaxed**: no gate validation, gate_count=0, durable=false
- **Journaled**: 15 gates enforced, bounded Fjall writer queue, group commit, durable=false
- **Strict**: 15 gates + synchronous SyncAll before acknowledgement, durable=true

## Gap Analysis
1. **ActionAbiMismatch** — defined in types.rs, IGNORED test (vb-ty9) — recovery lacks action ABI digest lookup/input
2. **PolicyDigestMismatch** — defined in types.rs, IGNORED test (vb-ty9) — recovery lacks expected policy digest lookup/input
3. **TerminalStateMismatch** — DEFERRED_GLOBAL — public API has no expected-terminal parameter

## Acceptance Tests Required
Per bead description:
- `test_strict_run_persists_run_accepted_before_ack` — strict durability profile persists RunAccepted before ack
- `test_recovery_hydrates_slots_taint_step_states_from_journal` — frame seed recovery with exact taint/step state
- `test_recovery_rejects_missing_slot_values_or_pending_action_state_when_unsupported` — unsupported recovery state
- `test_corrupt_record_digest_mismatch_and_non_idempotent_replay_fail_typed` — corrupt record + digest mismatch + non-idempotent replay

## Files to Create/Modify
1. `crates/workspace_tests/tests/vb_rpch_durability_recovery_bdd.rs` — new BDD scenario file
2. `crates/vb_storage/src/recovery/vb_rpch_tests.rs` — supplementary storage tests
3. Update `crates/vb_storage/src/recovery/mod.rs` — add vb_rpch_tests module