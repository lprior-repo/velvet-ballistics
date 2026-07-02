# Codebase Map: vb-qi37.1.1

Bead: `vb-qi37.1.1`
Title: `runtime/recovery: Journal deterministic step lifecycle`
State: 2 artifact retry
Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`

## Scope Summary

The active MASTER contract is DRIFT-2 in `velvet-ballistics-MASTER.md:3384-3401`. The intended recovery evidence chain is:

- Every deterministic step emits `StepStarted`.
- Every deterministic slot output emits `SlotWritten` before PC advances.
- `SlotWritten` must carry enough data to reconstruct both slot value and taint.
- Every deterministic successful step emits `StepSucceeded` after its slot writes.
- Recovery must fail with a typed error when `UnsupportedRecoveryState` says required state cannot be hydrated.
- Shard journal errors must propagate, not be swallowed.

Current implementation is partially ahead of the original drift text: runtime evidence flushing and storage value capture exist, but durable taint capture remains suspect because `JournalEvent::SlotWrittenEvent` has no taint field and recovery marks event-only taint unsupported.

## Relevant Files

- `crates/vb_runtime/src/journal.rs`
  - Defines `RuntimeJournalEvent::{StepStarted, SlotWritten, StepSucceeded}` at lines 103-129.
  - `RuntimeJournalConfig` selects volatile, journaled queue, or strict storage adapters at lines 222-255.
  - `StorageRuntimeJournal::storage_event` maps runtime lifecycle events to `vb_storage::JournalEvent` at lines 318-455.
  - `RuntimeJournalEvent::SlotWritten` maps to `JournalEvent::SlotWrittenEvent { value: Some(value), extra }` at lines 416-427.
  - There is no runtime event taint payload in `RuntimeJournalEvent::SlotWritten`; it carries `run`, `slot`, encoded `value`, and `extra` only.
  - `QueuedStorageRuntimeJournal::append` rejects `DurabilityProfile::Strict` with `UnsupportedAsyncStrictAck` at lines 538-542; DRIFT-2 notes this as an explicit remaining marker.

- `crates/vb_runtime/src/shard/impl_.rs`
  - `Shard::flush_evidence` drains `EvidenceCollector` and preserves event order at lines 228-241.
  - `flush_step_started` appends `RuntimeJournalEvent::StepStarted` and propagates `RuntimeResult` at lines 255-259.
  - `flush_step_succeeded` appends `RuntimeJournalEvent::StepSucceeded`, mapping `None` output to `SlotIdx::ZERO`, at lines 261-275.
  - `flush_slot_written` postcard-encodes `SlotValue`, optional collect pagination extra, pushes trace, and appends `RuntimeJournalEvent::SlotWritten` at lines 277-300.
  - Suspect gap: `flush_slot_written` receives `SlotValue` but not `Taint`, so deterministic taint cannot be persisted through the journal path.

- `crates/vb_runtime/src/shard/lifecycle.rs`
  - Submit path journals `RunSubmitted`/`RunAdmission`, inserts `RunState`, and calls `drive_run` at lines 89-131.
  - Action completion writes slot with taint into the live frame at lines 172-175, then journals `SlotWritten`, `StepSucceeded`, and `ActionCompleted` at lines 181-207.
  - Action completion also drops taint before journaling: `RuntimeJournalEvent::SlotWritten` carries encoded value only at lines 192-197.
  - `drive_run` call sites use `EvidenceCollector` and then `flush_evidence`; grep hits around lines 370-421 are important touchpoints for deterministic lifecycle order.

- `crates/vb_runtime/src/recovery.rs`
  - Runtime recovery boundary hydrates `RunFrame` from `RecoveryFrameSeed` at lines 63-69.
  - `reject_unsupported_live_frame_state` rejects hydration when `slot_values`, `slot_taint`, or unsupported pending actions are set at lines 73-82.
  - `apply_recovered_slots` writes recovered slot value and taint into `RunFrame` at lines 100-105.
  - Summary-only recovery intentionally returns `UnsupportedFullRecoveryHydration` at lines 144-151.

- `crates/vb_storage/src/events.rs`
  - Durable `JournalEvent` variants include `StepStarted`, `StepSucceeded`, and `SlotWrittenEvent` at lines 34-100.
  - `SlotWrittenEvent` stores `value: Option<Vec<u8>>` and `extra: Option<Vec<u8>>`; there is no durable taint field.
  - `JournalEvent::slot_value` decodes postcard value bytes at lines 228-237.
  - `record_kind` maps both `StepSucceeded` and `SlotWrittenEvent` to `RecordKind::SlotWritten` at lines 207-225; this may be intentional legacy encoding, but rust-contract should decide whether it is acceptable for deterministic lifecycle semantics.

- `crates/vb_storage/src/journal.rs`
  - `append_journaled`, `append_strict`, and `append_strict_batch` are the durability APIs at lines 189-209.
  - `append_unpersisted` serializes each `JournalEvent` under `(RunId, EventSeq)`, rejects duplicates, and propagates `JournalError` at lines 211-232.
  - This is the storage pattern to reuse for any new event payload shape: immutable event history, duplicate sequence rejection, postcard record encoding.

- `crates/vb_storage/src/recovery/replay/summary.rs`
  - Summary replay counts `StepStarted`, `StepSucceeded`, and `SlotWrittenEvent` at lines 22-61.
  - Frame seed recovery builds `RecoveryFrameSeed` from ordered events at lines 157-205.
  - `seed_unsupported_state` sets `slot_values` or `slot_taint` unsupported based on missing/corrupt values and event-only taint gaps at lines 245-275.
  - `FrameSeedAccumulator` currently has `slot_values` and `slot_taint` maps, but `record_slot_write` inserts value, removes taint, and marks `event_slot_taint_unsupported = true` at lines 385-402.
  - `recovered_event_slots` defaults missing taint to `Taint::Clean` at lines 493-507, but the unsupported flag should prevent runtime hydration if event taint is required.
  - With a compiled workflow, `recover_slots_through_step` can replay deterministic slots and recover taint from `RunFrame::initialized_slots` at lines 509-533; without workflow, durable event taint remains unsupported.

- `crates/vb_storage/src/recovery/recover.rs`
  - `recover_runtime_summary` reads events and returns summary hydration at lines 76-86.
  - `recover_runtime_frame_seed` reads events and delegates to `recover_runtime_frame_seed_from_events` at lines 88-98.
  - `recover_all_incomplete_runs` filters durable run headers by absence of terminal events at lines 112-134.

- `crates/vb_storage/src/recovery/types.rs`
  - `RecoveredSlotEntry` already carries `value` and `taint` at lines 200-209.
  - `UnsupportedRecoveryState` has explicit `slot_values`, `slot_taint`, `action_payloads`, and `pending_actions` flags at lines 220-231.
  - `event_slot_taint_unsupported` documents that event-only slot values have no durable taint payload at lines 242-249.
  - `slot_values_unsupported` sets both `slot_values` and `slot_taint` at lines 251-259.

- `crates/vb_storage/src/recovery/tests.rs`
  - Unit tests define deterministic workflow fixtures and helper journal events at lines 34-134.
  - Existing recovery tests cover summary hydration, frame dimensions, workflow replay, unsupported slot values/taint, and `SlotWrittenEvent` dimension tracking.
  - Good local place for contract tests around event-only value+taint recovery and unsupported hydration flags.

- `crates/vb_storage/tests/recovery_integration.rs`
  - Integration tests build full event chains with `RunAccepted`, `StepStarted`, `SlotWrittenEvent`, `StepSucceeded`, and `RunFinished` at lines 29-109.
  - Existing fixtures still write `SlotWrittenEvent { value: None, extra: None }`; tests verify ordering/counting but not value or taint recovery.
  - Good end-to-end storage place for strict append/reopen/recover assertions.

- `docs/storage-journal.md`
  - Documents append-only Fjall journal, big-endian run event keys, postcard encoding, duplicate rejection, and durability modes.
  - Recovery section still says future recovery will hydrate `RunFrame`, so docs are stale relative to current `RecoveryFrameSeed` support.

## Patterns To Reuse

- Use `EvidenceCollector` as the single deterministic lifecycle source, then `Shard::flush_evidence` for ordered trace/journal emission.
- Preserve `RuntimeResult` propagation with `try_for_each`; do not introduce swallowed journal errors.
- Continue postcard encoding for compact durable event payloads.
- Keep per-run monotonic `EventSeq` assignment inside runtime journal adapters.
- Keep `UnsupportedRecoveryState` as the explicit gate between storage recovery and runtime hydration.
- Use existing `RecoveryFrameSeed`/`RecoveredSlotEntry` shapes rather than adding a parallel recovery product.
- Prefer direct enum payload extension if taint must become durable; avoid sidecar events unless rust-contract requires backward compatibility for already persisted journals.

## Suspected Touchpoints For Implementation State

- Add durable taint to runtime evidence path:
  - `EvidenceEvent::SlotWritten` in `crates/vb_runtime/src/engine/*` or wherever `EvidenceCollector` is defined.
  - `Shard::flush_slot_written` in `crates/vb_runtime/src/shard/impl_.rs`.
  - `RuntimeJournalEvent::SlotWritten` in `crates/vb_runtime/src/journal.rs`.
  - Action and ask answer paths in `crates/vb_runtime/src/shard/lifecycle.rs` that currently know taint but journal only value.

- Add durable taint to storage event schema:
  - `JournalEvent::SlotWrittenEvent` in `crates/vb_storage/src/events.rs`.
  - All construction sites in tests and runtime adapters.
  - `record_kind` likely unchanged unless contract wants a distinct kind for `StepSucceeded`.

- Teach recovery to consume event taint:
  - `FrameSeedAccumulator::record_slot_write` in `crates/vb_storage/src/recovery/replay/summary.rs`.
  - `seed_unsupported_state` should leave `slot_taint` false only when taint is present or workflow replay reconstructs it.
  - `recovered_event_slots` should stop defaulting event-only recovered taint to clean unless explicitly justified by contract.

- Verify runtime hydration remains gated:
  - `reject_unsupported_live_frame_state` in `crates/vb_runtime/src/recovery.rs` already rejects `slot_values`/`slot_taint`; tests should pin this behavior.

## Test Locations

- Runtime unit tests:
  - `crates/vb_runtime/src/journal.rs` tests for runtime-to-storage event mapping, queued journal behavior, strict rejection, and sequence behavior.
  - `crates/vb_runtime/src/shard/impl_.rs` and `crates/vb_runtime/src/shard/tests.rs` for `flush_evidence` ordering and deterministic `StepStarted -> SlotWritten -> StepSucceeded` emission.
  - `crates/vb_runtime/src/recovery.rs` tests for hydration rejecting unsupported `slot_values`/`slot_taint` and applying recovered slot taint.

- Storage unit tests:
  - `crates/vb_storage/src/recovery/tests.rs` for summary counts, frame seed reconstruction, unsupported-state flags, digest mismatch, and value/taint event replay.
  - `crates/vb_storage/src/recovery/replay/summary.rs` inline tests for accumulator behavior if existing style keeps replay tests near implementation.

- Storage integration tests:
  - `crates/vb_storage/tests/recovery_integration.rs` for strict append, reopen, ordered event recovery, partial chain recovery, and full value+taint recovery once the durable event schema supports taint.

- Documentation acceptance:
  - `docs/storage-journal.md` should be updated after implementation to describe value+taint slot events and full-frame seed recovery status.
  - `velvet-ballistics-MASTER.md` DRIFT-2/Phase 44 status should be updated only when implementation and gates close the gap.

## Risks And Dependencies

- Durable taint schema is the key unresolved risk. `RecoveredSlotEntry` supports taint, but `JournalEvent::SlotWrittenEvent` does not store it.
- Adding a field to a serde/postcard enum variant can affect old persisted records. The next state should decide whether this repository requires backward compatibility for existing journals; if not, keep the change minimal.
- Event-only recovery currently marks taint unsupported but can still build a seed with default clean taint. Runtime hydration gate should block that seed; contract tests must prove it.
- Workflow replay can reconstruct deterministic value+taint only when compiled workflow is available and replayable. Event-only recovery still needs durable taint if hydration is expected without workflow replay.
- `StepSucceeded` with `None` output is encoded as `SlotIdx::ZERO` in runtime; rust-contract should clarify whether a no-output deterministic step may emit `StepSucceeded` without implying slot zero was written.
- Existing integration tests use `SlotWrittenEvent { value: None }`, which intentionally exercises unsupported value recovery. New tests should distinguish legacy/missing-value cases from full evidence cases.
- `QueuedStorageRuntimeJournal` rejects strict mode. Contract should decide whether this bead must preserve that behavior or only document it as an unsupported async strict ack path.
- There are prior bead artifacts under `.beads/vb-7gs9` with evidence-chain contracts and tests. They can be mined for wording, but this bead should not rely on them as authoritative over the MASTER contract.

## Next-State Notes For `rust-contract`

- Define the exact invariant for deterministic lifecycle order: for each deterministic step, `StepStarted(step)` must precede all `SlotWritten` emitted by that step, and `StepSucceeded(step)` must follow those slot writes before PC advancement.
- Decide whether `SlotWritten` must include `step` as well as `slot`, `value`, and `taint`. Current durable event has no step field, making per-step association indirect via order and `StepSucceeded.output`.
- Specify taint persistence contract explicitly: durable recovery from journal events alone either must reconstruct taint from a taint payload or must report `UnsupportedRecoveryState.slot_taint = true` and fail runtime hydration.
- Specify behavior for deterministic steps that do not write an output slot. Current `StepSucceeded` forces `SlotIdx::ZERO`; this may create false slot evidence.
- Specify unsupported hydration error mapping: storage marks unsupported state; runtime `hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when `slot_values`/`slot_taint` are true.
- Specify action/ask answer slot writes separately from deterministic internal step writes because lifecycle paths write tainted values but currently journal value only.
- Include negative contracts for missing value, corrupt value bytes, missing taint, mismatched run events, duplicate sequence, and digest mismatch.
- Keep implementation constraints from MASTER: no async, no channels, synchronous append in the shard drive loop, bounded writer queue backpressure, no swallowed journal write errors.

STATUS: COMPLETE
