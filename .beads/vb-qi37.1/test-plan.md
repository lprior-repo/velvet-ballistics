# Test Plan: vb-qi37.1 Recovery Hydration

STATUS: APPROVED

## Summary

- Behaviors identified: 12 contract clauses plus 12 typed error scenarios.
- Trophy allocation: unit and integration recovery tests, property tests for event-only recovery, static source gates, Verus/TLA proof gates.
- Proptest invariants: event-only slot recovery preserves values/taint, no-output success creates no slot zero, valid slot events remain hydrateable.

## Behavior Inventory

- Recovery rejects empty durable input with `RecoveryError::NoRecoveryData`.
- Recovery rejects mixed-run or corrupt ordering with `RecoveryError::ReplayDivergence`.
- Snapshot-plus-tail recovery accepts only matching run and later tail events.
- Summary recovery reports exact sequence bounds and counts.
- Frame-seed recovery reconstructs pc, dimensions, steps, slots, taint, pending action facts, terminal state, and unsupported flags from durable data.
- Runtime hydration rejects summary-only recovery with `RuntimeError::UnsupportedFullRecoveryHydration`.
- Runtime hydration rejects unsupported frame seed state with `RuntimeError::InvalidRecoveryHydration`.
- Runtime hydration applies recovered step states, slot values, taint, and pc exactly.
- Digest verification returns exact workflow-source and compiled-IR mismatch variants.
- Action ABI and policy digest checks are waived downstream until production exposes those surfaces.
- Recovery never consumes YAML/JSON/HTTP runtime artifacts.
- Source gates reject silent fallible-result discard, unwrap, expect, panic, todo, unimplemented, unchecked indexing, and unsafe in scoped recovery/runtime source.

## BDD Scenarios

- `given_event_only_slot_write_when_frame_seed_recovers_then_exact_slot_value_and_taint_are_returned` -> `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`.
- `given_no_output_step_when_frame_seed_recovers_then_slot_count_is_zero` -> workspace contract test.
- `given_corrupt_slot_value_when_frame_seed_recovers_then_unsupported_slot_values_is_set` -> workspace contract test.
- `given_supported_seed_when_runtime_boundary_hydrates_then_frame_contains_exact_taint` -> workspace/runtime tests.
- `given_summary_only_recovery_when_runtime_boundary_hydrates_then_unsupported_full_recovery_hydration_is_returned` -> `vb_runtime` recovery tests.
- `given_dimension_overflow_when_frame_seed_recovers_then_frame_dimension_overflow_is_returned` -> `vb_storage` recovery tests.
- `given_workflow_source_digest_mismatch_when_verify_digests_runs_then_workflow_source_digest_mismatch_is_returned` -> `vb_storage` recovery tests.
- `given_compiled_ir_digest_mismatch_when_verify_digests_runs_then_compiled_ir_digest_mismatch_is_returned` -> `vb_storage` recovery tests.
- `given_empty_journal_when_recovery_runs_then_no_recovery_data_is_returned` -> `vb_storage` recovery tests.
- `given_mixed_run_or_corrupt_sequence_when_recovery_runs_then_replay_divergence_is_returned` -> `vb_storage` recovery tests.

## Proptest Invariants

- Any valid `SlotValue::I64` event-only slot write in the planned range recovers the exact value and `Taint::Secret`.
- Any no-output success event in the planned step range keeps `slot_count == 0`.
- Any valid slot-write event in the planned slot/value range keeps `UnsupportedRecoveryState::SUPPORTED`.

## Fuzz Targets

- No new required fuzz target for this bead. Slot byte postcard decoding is covered by explicit corrupt-byte tests and fuzz/theorem/dependency lanes are waived in the approved contract for this bead.

## Mutation Checkpoints

- Deleting digest mismatch branches is caught by exact mismatch-variant tests.
- Changing dimension overflow bounds is caught by exact `FrameDimensionOverflow` tests and Verus dimension proof.
- Defaulting missing output to slot zero is caught by no-output slot-count tests.
- Ignoring unsupported flags is caught by runtime invalid-hydration tests and Verus unsupported-state proofs.

## Open Questions

- None blocking for vb-qi37.1. Action ABI and policy digest mismatch detection are downstream when production surfaces exist.
