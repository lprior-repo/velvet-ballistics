# Contract: vb-qi37.1.1 - runtime/recovery: Journal deterministic step lifecycle

## Scope

This bead specifies the durable evidence chain required for deterministic runtime step recovery. It covers:

- Runtime deterministic evidence emission from `EvidenceCollector` through `Shard::flush_evidence`.
- Runtime-to-storage mapping in `RuntimeJournalEvent` and `JournalEvent`.
- Durable `SlotWritten` recovery of both `SlotValue` and `Taint`.
- Recovery gating via `UnsupportedRecoveryState` and `DurableFrameRecoveryBoundary::hydrate_run_frame`.
- Journal write error propagation and duplicate sequence handling.

The active authority is DRIFT-2 in `velvet-ballistics-MASTER.md`: deterministic steps must emit `StepStarted`, `SlotWritten(value + taint)`, and `StepSucceeded`; recovery must reconstruct slot values and taint or fail with typed unsupported hydration.

## Domain Terms

- Deterministic step: a runtime step driven inside `drive_deterministic_full` without external action, wait, ask, YAML, JSON, or HTTP behavior in the runtime core.
- Evidence chain: the ordered lifecycle event sequence for a step: `StepStarted(step)` before all slot writes caused by that step, then `StepSucceeded(step, output)` after those writes and before PC advancement is considered recoverable.
- Slot write: a write of a `SlotValue` and its exact `Taint` to a `SlotIdx`, including deterministic writes and boundary writes from action completion or ask answer paths.
- Event-only recovery: recovery from durable journal events without replaying a compiled workflow.
- Workflow replay recovery: recovery with a matching `CompiledWorkflow`, where deterministic state may be recomputed through the last succeeded step.

## Preconditions

1. Every fallible journal, recovery, and hydration operation returns `Result<T, Error>`; no fallible path may panic, unwrap, expect, todo, unimplemented, dbg, or use unsafe.
2. `SlotWritten` evidence has access to the exact value and taint that were written to the live `RunFrame`.
3. Runtime journal adapters assign monotonic per-run `EventSeq` values and never reuse a sequence for a different event.
4. Storage append receives events for exactly one `(RunId, EventSeq)` key per durable record.
5. Recovery frame seed construction receives events in durable sequence order for one run only.
6. Runtime hydration receives a `RecoveryFrameSeed` whose dimensions, PC, steps, slots, and unsupported flags were derived from durable events or verified replay.
7. `moon ci` remains the canonical acceptance gate; ad-hoc cargo commands are not sufficient for final acceptance.

## Postconditions

1. For each deterministic successful step, durable storage contains `StepStarted(step)`, zero or more `SlotWritten(slot, value, taint, extra)` events emitted by that step, and `StepSucceeded(step, output)` in that order.
2. Any deterministic slot output is journaled before the PC advances past the producing step for recoverability purposes.
3. Every durable slot write that is considered hydrateable carries enough data to reconstruct both `SlotValue` and `Taint` without defaulting missing taint to `Taint::Clean`.
4. Event-only recovery with complete value and taint payloads produces `RecoveredSlotEntry { slot, value, taint }` and leaves `UnsupportedRecoveryState.slot_values == false` and `slot_taint == false`.
5. Event-only recovery with missing/corrupt value or missing taint marks unsupported state and runtime hydration returns `RuntimeError::InvalidRecoveryHydration`.
6. Workflow replay recovery rejects digest mismatch with `RecoveryError::CompiledIrDigestMismatch` and never hydrates state from the wrong artifact.
7. Journal write failures propagate as `RuntimeError` or storage `JournalError`; they are never swallowed or converted into success.
8. Duplicate durable `(RunId, EventSeq)` writes fail with a typed duplicate error unless an existing queued retry is byte-for-byte idempotent under the established queued writer contract.
9. Volatile mode may retain in-memory events only; journaled and strict modes must preserve the same event semantics.

## Invariants

1. Ordering invariant: for a given run and step, `StepSucceeded(step)` must not appear before `StepStarted(step)`.
2. Slot-before-success invariant: any `SlotWritten` caused by a step must appear after that step's `StepStarted` and before that step's `StepSucceeded`.
3. Taint fidelity invariant: recovered taint must equal the taint written to the live frame; missing taint is unsupported, not clean.
4. Value fidelity invariant: recovered slot value bytes must decode to the exact `SlotValue` written to the live frame; corrupt bytes make slot values unsupported.
5. Hydration gate invariant: if `UnsupportedRecoveryState.slot_values`, `slot_taint`, or unsupported pending actions are true, `hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration`.
6. Single-run replay invariant: recovery over mixed run ids fails with `RecoveryError::ReplayDivergence`.
7. Bounded-resource invariant: evidence collection and journal writing remain bounded; overflow or backpressure must be explicit and recoverability-affecting drops must be observable through typed error or diagnostic state, not silent success.
8. Core serialization invariant: runtime core uses compact binary/postcard journal payloads only; no runtime JSON, YAML, or HTTP is introduced.
9. No-output step invariant: a deterministic step that writes no output must not create false slot-zero recovery evidence. If a durable `StepSucceeded` cannot represent `None`, hydration must not infer that slot zero was written from `StepSucceeded` alone.

## Contract Signatures

These signatures are contractual shapes, not implementation instructions:

```rust
pub enum RuntimeJournalEvent {
    StepStarted { run: RunId, step: StepIdx },
    SlotWritten {
        run: RunId,
        step: Option<StepIdx>,
        slot: SlotIdx,
        value: Vec<u8>,
        taint: Taint,
        extra: Option<Vec<u8>>,
    },
    StepSucceeded { run: RunId, step: StepIdx, output: Option<SlotIdx> },
}

pub enum JournalEvent {
    SlotWrittenEvent {
        run: RunId,
        seq: EventSeq,
        step: Option<StepIdx>,
        slot: SlotIdx,
        value: Option<Vec<u8>>,
        taint: Option<Taint>,
        extra: Option<Vec<u8>>,
    },
}

pub trait RuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>;
    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport>;
}

pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed>;

pub fn recover_runtime_frame_seed_from_events_with_workflow(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<RecoveryFrameSeed>;

pub trait RuntimeRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary;
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
}
```

`Option<StepIdx>` and `Option<SlotIdx>` preserve compatibility for boundary writes and no-output steps. If implementation chooses a different representation, it must still preserve the same semantics: no false slot-zero evidence and deterministic writes attributable by order or explicit step.

## Typed Error Taxonomy

- `RuntimeError::EncodeFailed`: postcard encoding of `SlotValue`, taint payload, or frame extra fails.
- `RuntimeError::JournalPoisoned`: runtime journal sequence or volatile event storage lock is poisoned.
- `RuntimeError::InvalidRecoveryHydration`: recovery seed reports unsupported slot values, unsupported taint, unsupported pending action hydration, invalid dimensions, invalid PC, or invalid recovered step/slot application.
- `RuntimeError::UnsupportedFullRecoveryHydration`: caller asks a summary-only boundary for a live frame.
- `RuntimeError::UnsupportedAsyncStrictAck`: queued async adapter is asked to provide strict per-event acknowledgement.
- `JournalError::DuplicateEvent { run, seq }`: storage already contains a non-idempotent event for the same per-run sequence.
- `JournalError::WriteLockPoisoned`: storage writer lock cannot be acquired.
- `JournalError::EncodeRecord` or equivalent: durable record serialization exceeds bounded payload rules or fails encoding.
- `RecoveryError::NoRecoveryData { run }`: recovery is requested with no durable events.
- `RecoveryError::ReplayDivergence { step, detail }`: recovery sees mixed run ids, impossible lifecycle ordering, invalid replay evidence, or contradictory recovered state.
- `RecoveryError::CompiledIrDigestMismatch { expected, found }`: replay workflow digest does not match the durable `RunAccepted` workflow digest.
- `RecoveryError::FrameDimensionOverflow { run }`: recovered max step or slot index cannot fit the frame dimension type.

## Acceptance Criteria

1. Deterministic drive evidence contains taint in `EvidenceEvent::SlotWritten` or an equivalent type-safe payload before flushing.
2. `Shard::flush_slot_written`, action completion, and ask answer journaling persist the taint they wrote to `RunFrame`.
3. `JournalEvent::SlotWrittenEvent` can durably represent value and taint; old/missing taint records remain unsupported unless replay can prove taint.
4. Recovery from complete event-only slot writes reconstructs `RecoveredSlotEntry.value` and `.taint` without defaulting missing taint to clean.
5. Recovery from missing value, corrupt value, missing taint, mixed run ids, duplicate sequence, and digest mismatch returns typed errors or unsupported flags as specified.
6. `StepSucceeded` no-output semantics do not imply slot zero was written.
7. Journal append errors from storage and queued writer paths propagate to the shard caller.
8. Final implementation updates `docs/storage-journal.md` and DRIFT-2 status only after the evidence is implemented and `moon ci` passes.

## Martin Fowler Given/When/Then Scenarios

### Scenario 1: deterministic step emits ordered lifecycle evidence
Given a run executes a deterministic step that writes one output slot with taint
When the shard flushes collected evidence
Then the durable journal records `StepStarted`, `SlotWritten(value, taint)`, and `StepSucceeded` in that order
And the slot write appears before the step is recoverably advanced.

### Scenario 2: event-only recovery hydrates complete slot state
Given ordered events include a complete `SlotWrittenEvent` with decodable value and taint
When `recover_runtime_frame_seed_from_events` builds a seed
Then the seed contains `RecoveredSlotEntry` with the same value and taint
And `UnsupportedRecoveryState.slot_values` and `slot_taint` are false.

### Scenario 3: missing taint blocks hydration
Given ordered events include a slot value but no durable taint and no workflow replay proof
When runtime hydration is requested
Then recovery marks `slot_taint` unsupported
And `hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration`.

### Scenario 4: corrupt slot value blocks hydration
Given a `SlotWrittenEvent` contains bytes that do not decode as `SlotValue`
When frame seed recovery runs
Then `slot_values` and `slot_taint` are unsupported
And runtime hydration fails with `RuntimeError::InvalidRecoveryHydration`.

### Scenario 5: workflow digest mismatch rejects replay
Given durable events name workflow digest A
And replay is attempted with compiled workflow digest B
When workflow recovery runs
Then it returns `RecoveryError::CompiledIrDigestMismatch`
And no frame seed is hydrated from the wrong workflow.

### Scenario 6: journal append failure propagates
Given a strict storage journal append fails
When a shard flushes `StepStarted`, `SlotWritten`, or `StepSucceeded`
Then the shard method returns a `RuntimeError`
And later events are not reported as successfully persisted.

### Scenario 7: no-output step does not fabricate slot zero
Given a deterministic step succeeds without an output slot
When lifecycle events are recovered
Then `StepSucceeded` marks the step succeeded
And recovery does not infer that `SlotIdx::ZERO` was written unless a distinct `SlotWritten` event exists.

### Scenario 8: mixed run events are rejected
Given recovery receives events for two run ids in one ordered slice
When frame seed recovery applies the events
Then it returns `RecoveryError::ReplayDivergence`
And no partial frame is exposed.

## Proof Obligations

- Prove with unit tests that every deterministic successful write path carries taint from live frame write through runtime event to storage event.
- Prove with recovery tests that missing taint is unsupported and cannot silently become `Taint::Clean`.
- Prove with integration tests that strict append, reopen, and recover preserve value and taint across storage boundaries.
- Prove duplicate event sequence rejection remains intact for non-idempotent duplicate writes.
- Prove no-output steps do not inflate slot dimensions or fabricate slot zero evidence.
- Prove `moon ci` passes after implementation.

## Out of Scope

- Implementing code or tests in this State 3 contract step.
- Adding JSON, YAML, HTTP, or alternate runtime-core serialization.
- Redesigning the full journal storage engine, Fjall partitioning, or queue architecture.
- Making summary-only recovery hydrate a live frame.
- Resolving `UnsupportedAsyncStrictAck` for queued strict acknowledgements beyond preserving explicit typed rejection.
- Updating product performance claims without real baseline/result benchmarks.

## Risk Notes

- Extending serde/postcard enum variants can affect already persisted records. If backward compatibility is required, old records without taint must be treated as unsupported unless workflow replay reconstructs taint.
- Current `EvidenceCollector` silently drops events at capacity; DRIFT-2 recoverability requires any dropped evidence to be observable and not treated as a complete durable chain.
- Current `StepSucceeded { output: SlotIdx }` forces `SlotIdx::ZERO` for no-output steps; this is a recovery correctness risk.
- Current storage maps `StepSucceeded` and `SlotWrittenEvent` to `RecordKind::SlotWritten`; this may be acceptable only if decode remains unambiguous and lifecycle summaries remain correct.
- Action completion and ask answer paths already know taint but currently journal only value; these boundary paths must not regress while fixing deterministic evidence.
