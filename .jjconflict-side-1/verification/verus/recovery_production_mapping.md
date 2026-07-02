# Recovery Production Mapping

Obligation: `VERUS-REC-001` / `PO-002`.

This refinement note binds `verification/verus/recovery_hydration_contracts.rs` to the production-shaped recovery boundary discovered in the isolated workspace. It is verification evidence only; it does not edit production code.

## Spec To Production Shape

- `SpecRecoveryInput.has_header` maps to `recover_runtime_summary`, `recover_runtime_frame_seed`, and `recover_all_incomplete_runs` receiving at least one durable journal event for the target `RunId`; empty event sets return `RecoveryError::NoRecoveryData` in `crates/vb_storage/src/recovery/recover.rs`.
- `SpecRecoveryInput.has_required_slot` maps to `RecoveryFrameSeed.slots` and `RecoveredSlotEntry` carrying the slot value required to hydrate a live `RunFrame` in `crates/vb_storage/src/recovery/types.rs`.
- `SpecRecoveryInput.has_taint`, `secret_required`, and `recovered_secret` map to `RecoveredSlotEntry.taint: Taint` and `RunFrame::write_slot_with_taint` use in `crates/vb_storage/src/recovery/hydrate.rs`.
- `SpecRecoveryInput.snapshot_valid` maps to `RunSnapshot` run id, sequence, slot, and taint decoding checks in `hydrate_run_frame`; production errors are `RecoveryError::ReplayDivergence`, `RecoveryError::CorruptSnapshot`, or `RecoveryError::NoRecoveryData` depending on the rejected fact.
- `SpecRecoveryInput.ordered`, `tail_after_watermark`, and `fact_erased` map to durable journal ordering, tail-after-snapshot checks, and monotonic snapshot-plus-tail application in `hydrate_run_frame` and replay summary code.
- `SpecRecoveryInput.workflow_source_digest_match` maps to `check_workflow_source_digest`, whose mismatch branch returns `RecoveryError::WorkflowSourceDigestMismatch`.
- `SpecRecoveryInput.compiled_ir_digest_match` maps to `check_compiled_ir_digest`, whose mismatch branch returns `RecoveryError::CompiledIrDigestMismatch`.
- `SpecRecoveryInput.pending_action` maps to unresolved `RecoveredPendingAction` / pending action lifecycle facts; unsupported pending action restart maps to `RecoveryError::NonIdempotentActionBlocked` or runtime-boundary rejection.
- `SpecRecoveryInput.collect_extra_valid` maps to `CollectStates::hydrate_journal_events` / `hydrate_extra_with_context`; corrupt, empty, wrong-identity, or wrong-page extra maps to `EngineError::CollectExtraHydrationFailed`.
- `SpecRecoveryInput.runtime_boundary_supported` maps to `RecoveryHydration::FrameSeed` being consumable by the runtime live-frame hydration boundary; unsupported or internally inconsistent frame seed maps to `RuntimeError::InvalidRecoveryHydration` or `RuntimeError::UnsupportedFullRecoveryHydration`.
- `SpecRecoveryInput.dimensions` and `max_dimensions` map to derived `step_count`, `slot_count`, and `RunFrame::new`/counter bounds; overflow maps to `RecoveryError::FrameDimensionOverflow`.

## Result Lattice

- `Ok(SpecRecoverySuccess)` maps to a production recovery product that is runnable or summary-complete only after all durable facts required by the chosen boundary are present.
- `SpecRecoveryError::NoRecoveryData` maps to `RecoveryError::NoRecoveryData`.
- `SpecRecoveryError::CorruptSnapshot` maps to `RecoveryError::CorruptSnapshot` or snapshot decode rejection.
- `SpecRecoveryError::ReplayDivergence` maps to `RecoveryError::ReplayDivergence` and journal sequence failures.
- `SpecRecoveryError::WorkflowSourceDigestMismatch` maps to `RecoveryError::WorkflowSourceDigestMismatch`.
- `SpecRecoveryError::CompiledIrDigestMismatch` maps to `RecoveryError::CompiledIrDigestMismatch`.
- `SpecRecoveryError::NonIdempotentActionBlocked` maps to `RecoveryError::NonIdempotentActionBlocked`.
- `SpecRecoveryError::FrameDimensionOverflow` maps to `RecoveryError::FrameDimensionOverflow`.
- `SpecRecoveryError::InvalidRecoveryHydration` maps to `RuntimeError::InvalidRecoveryHydration` at the caller/runtime boundary required by `PRE-006`.
- `SpecRecoveryError::CollectExtraHydrationFailed` maps to `EngineError::CollectExtraHydrationFailed`.

## Trusted Boundaries

- Fjall I/O durability and OS crash behavior remain outside Verus and are owned by integration evidence.
- Ordered journal replay is supplied by TLA+ and integration evidence before this Verus abstraction is treated as production-bound.
- Byte decoding is trusted after production decode functions return typed values; corrupt encodings must be exercised by integration/proptest/mutation lanes.
- This note binds the abstraction to named production types and errors, but it is not a substitute for later formal-verifier execution of production tests and gates.
