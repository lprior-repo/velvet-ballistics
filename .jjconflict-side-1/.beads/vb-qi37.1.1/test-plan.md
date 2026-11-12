# Test Plan: vb-qi37.1.1 — runtime/recovery: Journal deterministic step lifecycle

## Summary

This repaired plan explicitly addresses every rejection in `.beads/vb-qi37.1.1/test-plan-review.md`: shutdown drain coverage, recovery summary coverage, >=5x unit-test density, exact typed errors, exact replay diagnostic fields, drain mutations, hydration boundary failures, EventSeq overflow, resource cleanup, and concrete static/panic checks.

- Counted public contract surface: 6 functions/methods from the bead contract: `RuntimeJournal::append`, `RuntimeJournal::drain_for_shutdown`, `recover_runtime_frame_seed_from_events`, `recover_runtime_frame_seed_from_events_with_workflow`, `RuntimeRecoveryBoundary::summary`, `RuntimeRecoveryBoundary::hydrate_run_frame`.
- Required unit-test density: 6 public APIs × 5 = **30 minimum unit scenarios**.
- Planned unit scenarios: **34** named unit scenarios, plus 16 integration, 2 E2E/acceptance, and 5 static gates.
- Behaviors identified: 34 runtime/storage/recovery behaviors + 5 static resource/policy behaviors.
- Proptest invariants: 10.
- Fuzz targets: 4.
- Kani harnesses: 7.
- Mutation threshold: `cargo-mutants` kill rate **>= 90%** for touched runtime/storage/recovery files, with the drain-specific mutants listed below required to die.
- Final acceptance gate: `moon ci` from workspace root.

All assertions must compare exact values, exact structs, exact flags, exact event sequences, or exact enum variants. No test may assert only `is_ok()` or `is_err()`.

## 1. Behavior Inventory

1. Runtime evidence records `StepStarted` before slot writes when a deterministic step starts.
2. Runtime evidence records `SlotWritten` with exact `SlotValue` bytes and exact `Taint` when a deterministic step writes a slot.
3. Runtime evidence records `StepSucceeded` after all slot writes when a deterministic step succeeds.
4. Runtime-to-storage mapping preserves run, sequence, optional step, slot, value, taint, and extra when a slot-write event is durably encoded.
5. Runtime slot encoding returns `RuntimeError::EncodeFailed` when postcard encoding fails before event creation.
6. Action completion journaling preserves the exact taint when an action writes an output slot.
7. Ask-answer journaling preserves the exact taint when an ask answer writes an output slot.
8. Runtime journal append assigns consecutive per-run `EventSeq` values when events are appended for one run.
9. Runtime journal append returns `RuntimeError::JournalPoisoned` when its sequence/event lock is poisoned.
10. Runtime journal append returns `RuntimeError::UnsupportedAsyncStrictAck` when queued strict mode is asked for per-event strict acknowledgement.
11. Runtime journal append returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::WriteLockPoisoned) }` when the storage append layer reports a poisoned write lock.
12. `RuntimeJournal::drain_for_shutdown` returns `JournalWriterFlushReport { drained: 0, written: 0 }` for the default/noop/empty journal path.
13. `RuntimeJournal::drain_for_shutdown` drains all queued journaled events and returns exact `drained`/`written` counts when storage writes succeed.
14. `RuntimeJournal::drain_for_shutdown` returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::WriteLockPoisoned) }` and leaves exactly two queued events observable when storage drain fails after two events were queued.
15. `RuntimeJournal::drain_for_shutdown` is idempotent after a successful full drain when duplicate drain support is expected.
16. Volatile journal mode retains ordered in-memory lifecycle events when configured for volatile evidence.
17. Journaled and strict storage modes preserve lifecycle semantics across append, reopen, and recovery.
18. Shard evidence flushing returns the exact propagated runtime error when a journal append fails.
19. Shard evidence flushing does not report later lifecycle events as persisted after an earlier append failure.
20. Storage append rejects non-idempotent duplicate `(RunId, EventSeq)` writes with `JournalError::DuplicateEvent { run, seq }`.
21. Storage append returns `JournalError::WriteLockPoisoned` when the storage writer lock is poisoned.
22. Storage append returns `JournalError::Encode(_)` when durable record postcard encoding fails.
23. Event sequence allocation returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::SequenceOverflow) }` when incrementing `EventSeq(u64::MAX)` would overflow.
24. Event-only recovery reconstructs `RecoveredSlotEntry { slot, value, taint }` when a slot event has decodable value and durable taint.
25. Event-only recovery returns a valid summary/seed with zero recovered slots when ordered lifecycle events contain no slot writes.
26. Event-only recovery marks `slot_values` and `slot_taint` unsupported when a slot event has corrupt value bytes.
27. Event-only recovery marks `slot_values` unsupported when a slot event has `value: None`.
28. Event-only recovery marks `slot_taint` unsupported when a slot event has value bytes but no durable taint and no replay proof.
29. Event-only recovery returns `RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "frame seed recovery received events for multiple runs" }` when frame seed events mix run ids.
30. Summary recovery returns `RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "recovery summary received events for multiple runs" }` when summary events mix run ids.
31. Core replay returns `RecoveryError::ReplayDivergence { step: StepIdx(1), detail: "step 1 executed before previous step 2" }` when a lower step starts after a higher step.
32. Recovery returns `RecoveryError::NoRecoveryData { run: RunId(0) }` when direct event-slice recovery receives no events.
33. Workflow replay recovery returns `RecoveryError::CompiledIrDigestMismatch { expected: digest_b, found: digest_a }` when durable and compiled workflow digests differ.
34. Recovery returns `RecoveryError::FrameDimensionOverflow { run }` when recovered max step or slot index cannot fit frame dimensions.
35. `RuntimeRecoveryBoundary::summary` returns exact run, first/last seq, workflow, step counts, slot count, terminal state, and no-output state for summary-only hydration.
36. `RuntimeRecoveryBoundary::summary` returns the embedded seed summary exactly for durable frame hydration.
37. `RuntimeRecoveryBoundary::hydrate_run_frame` returns a live frame with exact recovered step states, slot values, taints, and PC when seed state is supported and dimensions are valid.
38. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when `unsupported.slot_values` is true.
39. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when `unsupported.slot_taint` is true.
40. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when pending actions exist and `unsupported.pending_actions` is true.
41. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when frame dimensions are invalid.
42. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when recovered PC is outside step bounds.
43. `RuntimeRecoveryBoundary::hydrate_run_frame` returns `RuntimeError::InvalidRecoveryHydration` when recovered step/slot application targets outside frame bounds.
44. Summary-only boundary hydration returns `RuntimeError::UnsupportedFullRecoveryHydration` when a caller asks it for a live frame.
45. No-output deterministic step recovery marks the step succeeded without fabricating `SlotIdx::ZERO` when no distinct slot write exists.
46. Temporary storage integration tests remove/drop their temp database directory when test scope ends.
47. Static policy rejects newly introduced `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` in changed production files.
48. Static policy rejects runtime-core JSON, YAML, or HTTP additions.
49. `moon ci` passes after implementation and tests.

## 2. Trophy Allocation

| Public surface / behavior group | Unit scenarios | Integration scenarios | E2E/static | Rationale |
|---|---:|---:|---:|---|
| `RuntimeJournal::append` sequencing/errors | 7 | 4 | 0 | Pure sequence/error branches are unit-testable; real storage append requires integration. |
| `RuntimeJournal::drain_for_shutdown` | 6 | 3 | 0 | Review-required public boundary; count fields and failure branches locally, prove real queue/storage drain with integration. |
| `recover_runtime_frame_seed_from_events` | 8 | 3 | 0 | Most recovery diagnostics and flags are deterministic; real reopen/recover is integration. |
| `recover_runtime_frame_seed_from_events_with_workflow` | 4 | 2 | 0 | Digest mismatch and replay boundary require workflow fixtures. |
| `RuntimeRecoveryBoundary::summary` | 4 | 1 | 0 | Exact summary is public observable state and can lie independently of hydration. |
| `RuntimeRecoveryBoundary::hydrate_run_frame` | 5 | 3 | 0 | Hydration gate truth table is unit; end-to-end recovered frame is integration. |
| Static/resource/acceptance | 0 | 0 | 7 | Source-policy, resource ownership, mutation, fuzz, Kani, and `moon ci` are gates. |

Planned unit density is 34/6 = 5.67x, satisfying the review mandate of >=5x.

## 3. BDD Scenarios

### Behavior: deterministic step emits ordered lifecycle evidence
Test function: `fn shard_records_ordered_lifecycle_when_deterministic_step_writes_output()`
Given: run `RunId(42)` executes deterministic `StepIdx(2)` and writes `SlotIdx(3)` with `SlotValue::I64(99)` and `Taint::Tainted`.
When: the shard drives the step and flushes evidence.
Then: persisted events for the run equal `[StepStarted { run: 42, step: 2 }, SlotWritten { run: 42, step: Some(2), slot: 3, value: postcard(I64(99)), taint: Tainted, extra: None }, StepSucceeded { run: 42, step: 2, output: Some(3) }]` in that order.

### Behavior: slot encode failure is exact
Test function: `fn shard_returns_encode_failed_when_slot_value_postcard_encoding_fails()`
Given: the slot-write path receives a value fixture whose serializer returns a postcard encode failure.
When: `flush_slot_written` attempts to create durable evidence.
Then: the result equals `Err(RuntimeError::EncodeFailed)` and the recording journal contains zero `SlotWritten` events.

### Behavior: action completion persists taint
Test function: `fn action_completion_records_exact_taint_when_action_writes_output()`
Given: action `ActionId(7)` completes at `StepIdx(4)` and writes `SlotValue::Bool(true)` with `Taint::Tainted` to `SlotIdx(5)`.
When: action completion evidence is appended and recovered.
Then: the durable/recovered slot entry equals `RecoveredSlotEntry { slot: SlotIdx(5), value: SlotValue::Bool(true), taint: Taint::Tainted }`.

### Behavior: ask answer persists taint
Test function: `fn ask_answer_records_exact_taint_when_answer_writes_output()`
Given: ask answer at `StepIdx(6)` writes `SlotValue::Symbol(answer)` with `Taint::Clean` to `SlotIdx(8)`.
When: ask-answer evidence is appended and recovered.
Then: the durable/recovered slot entry equals `RecoveredSlotEntry { slot: SlotIdx(8), value: answer, taint: Taint::Clean }`.

### Behavior: append assigns monotonic sequences
Test function: `fn runtime_journal_assigns_event_seq_0_1_2_when_three_events_are_appended_for_one_run()`
Given: an empty runtime journal sequence map for `RunId(42)`.
When: `StepStarted`, `SlotWritten`, and `StepSucceeded` are appended.
Then: durable event sequences equal `[EventSeq(0), EventSeq(1), EventSeq(2)]` with no gaps.

### Behavior: append rejects sequence overflow
Test function: `fn runtime_journal_returns_sequence_overflow_when_event_seq_is_u64_max()`
Given: the runtime journal sequence map has `RunId(42) -> EventSeq(u64::MAX)`.
When: another event for `RunId(42)` is appended.
Then: append returns `Err(RuntimeError::StorageJournalAppend { source })` where `source.as_ref() == &JournalError::SequenceOverflow`, and no event is enqueued or written.

### Behavior: append lock poison is exact
Test function: `fn runtime_journal_returns_journal_poisoned_when_sequence_lock_is_poisoned()`
Given: the runtime journal sequence/event mutex is poisoned by a previous holder.
When: `append` is called with any lifecycle event.
Then: the result equals `Err(RuntimeError::JournalPoisoned)`.

### Behavior: queued strict ack is rejected
Test function: `fn queued_runtime_journal_returns_unsupported_async_strict_ack_when_strict_append_is_requested()`
Given: `QueuedStorageRuntimeJournal` is configured with `DurabilityProfile::Strict`.
When: `append(StepStarted { run: 42, step: 1 })` is called.
Then: the result equals `Err(RuntimeError::UnsupportedAsyncStrictAck)` and the queue length remains unchanged.

### Behavior: default drain report is exact
Test function: `fn runtime_journal_default_drain_returns_zero_report_when_no_queue_exists()`
Given: a `NoopRuntimeJournal` or trait implementation using the default `drain_for_shutdown`.
When: `drain_for_shutdown` is called.
Then: it returns `Ok(JournalWriterFlushReport { drained: 0, written: 0 })`.

### Behavior: queued drain success report is exact
Test function: `fn queued_runtime_journal_drain_returns_exact_report_when_three_events_are_written()`
Given: a queued journaled runtime journal has exactly three queued events for `RunId(42)` and a real temp `FjallJournal`.
When: `drain_for_shutdown` is called.
Then: it returns `Ok(JournalWriterFlushReport { drained: 3, written: 3 })` and `events_for_run(RunId(42))` returns exactly those three events in sequence order.
And: the temp database owner is dropped and the temp directory is removed at test end.

### Behavior: queued drain failure is exact
Test function: `fn queued_runtime_journal_drain_returns_write_lock_poisoned_when_storage_lock_is_poisoned()`
Given: a queued journal has two pending events and the underlying storage writer lock is poisoned.
When: `drain_for_shutdown` is called.
Then: it returns `Err(RuntimeError::StorageJournalAppend { source })` where `source.as_ref() == &JournalError::WriteLockPoisoned`, and the remaining queued count is exactly `2`.

### Behavior: queued drain is idempotent after success
Test function: `fn queued_runtime_journal_drain_returns_zero_report_when_called_again_after_success()`
Given: a queued journal has already returned `JournalWriterFlushReport { drained: 3, written: 3 }`.
When: `drain_for_shutdown` is called again.
Then: it returns `Ok(JournalWriterFlushReport { drained: 0, written: 0 })` and storage still contains exactly three events, not six.

### Behavior: volatile journal preserves order
Test function: `fn volatile_runtime_journal_retains_exact_events_when_lifecycle_events_are_appended()`
Given: volatile mode and three lifecycle events including taint.
When: the events are appended.
Then: retained events equal the appended vector byte-for-byte, including `Taint` and `extra`.

### Behavior: shard append failure propagates exactly
Test function: `fn shard_flush_returns_journal_poisoned_when_runtime_journal_append_is_poisoned()`
Given: a runtime journal adapter that returns `RuntimeError::JournalPoisoned` for `SlotWritten`.
When: the shard flushes `[StepStarted, SlotWritten, StepSucceeded]`.
Then: flush returns `Err(RuntimeError::JournalPoisoned)`.

### Behavior: shard stops after failed append
Test function: `fn shard_flush_records_no_step_succeeded_when_slot_written_append_fails()`
Given: a journal accepts `StepStarted` and returns `RuntimeError::JournalPoisoned` for `SlotWritten`.
When: evidence is flushed.
Then: the recording journal contains exactly `[StepStarted { run: 42, step: 2 }]` and contains no `StepSucceeded`.

### Behavior: duplicate durable sequence is rejected
Test function: `fn storage_returns_duplicate_event_when_same_run_seq_has_different_event()`
Given: storage contains `JournalEvent::StepStarted { run: RunId(42), seq: EventSeq(7), step: StepIdx(2) }`.
When: storage appends `JournalEvent::StepSucceeded { run: RunId(42), seq: EventSeq(7), step: StepIdx(2), output: SlotIdx(3) }`.
Then: append returns `Err(JournalError::DuplicateEvent { run: RunId(42), seq: EventSeq(7) })`.

### Behavior: storage lock poison is exact
Test function: `fn storage_append_returns_write_lock_poisoned_when_append_lock_is_poisoned()`
Given: the storage append lock is poisoned.
When: a valid `JournalEvent::StepStarted` is appended.
Then: append returns `Err(JournalError::WriteLockPoisoned)`.

### Behavior: storage encode failure is exact
Test function: `fn storage_append_returns_encode_when_record_postcard_encoding_fails()`
Given: a durable event fixture forces postcard record serialization to fail.
When: storage append serializes the record.
Then: append returns the exact outer variant `Err(JournalError::Encode(_))`.

### Behavior: complete event-only recovery hydrates exact slot
Test function: `fn event_only_recovery_returns_exact_slot_entry_when_value_and_taint_are_present()`
Given: ordered events for `RunId(42)` include `RunAccepted(seq 0)`, `StepStarted(seq 1, step 2)`, `SlotWrittenEvent(seq 2, slot 3, value: Some(postcard(I64(99))), taint: Some(Tainted))`, and `StepSucceeded(seq 3, step 2, output: Some(3))`.
When: `recover_runtime_frame_seed_from_events` runs.
Then: `seed.slots == [RecoveredSlotEntry { slot: SlotIdx(3), value: SlotValue::I64(99), taint: Taint::Tainted }]`, `seed.unsupported == UnsupportedRecoveryState::SUPPORTED`, `seed.summary.steps_started == 1`, `seed.summary.steps_succeeded == 1`, and `seed.summary.slots_written == 1`.

### Behavior: valid zero-slot recovery summary has no slots
Test function: `fn event_only_recovery_returns_empty_slots_when_lifecycle_has_zero_slot_writes()`
Given: ordered events contain `RunAccepted(seq 0)`, `StepStarted(seq 1, step 2)`, and `StepSucceeded(seq 2, step 2, output: None)` under the contract representation.
When: `recover_runtime_frame_seed_from_events` runs.
Then: `seed.summary.steps_started == 1`, `seed.summary.steps_succeeded == 1`, `seed.summary.slots_written == 0`, `seed.slots == []`, and no recovered entry has `slot == SlotIdx::ZERO`.

### Behavior: corrupt slot value blocks hydration
Test function: `fn event_only_recovery_marks_values_and_taint_unsupported_when_value_bytes_are_corrupt()`
Given: a `SlotWrittenEvent` has `value: Some([0xff, 0x00, 0x13])` and `taint: Some(Taint::Clean)`.
When: frame seed recovery and hydration run.
Then: `seed.unsupported.slot_values == true`, `seed.unsupported.slot_taint == true`, and hydration returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: missing slot value blocks hydration
Test function: `fn event_only_recovery_marks_slot_values_unsupported_when_value_is_none()`
Given: a `SlotWrittenEvent` has `value: None` and `taint: Some(Taint::Tainted)`.
When: frame seed recovery and hydration run.
Then: `seed.unsupported.slot_values == true`, `seed.unsupported.slot_taint == true`, and hydration returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: missing taint blocks hydration
Test function: `fn event_only_recovery_marks_slot_taint_unsupported_when_taint_is_none()`
Given: a `SlotWrittenEvent` has `value: Some(postcard(SlotValue::I64(99)))`, `taint: None`, and no compiled workflow replay proof.
When: frame seed recovery and hydration run.
Then: `seed.unsupported.slot_values == false`, `seed.unsupported.slot_taint == true`, recovered taint is not accepted as a clean default, and hydration returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: frame seed recovery rejects mixed runs with exact detail
Test function: `fn frame_seed_recovery_returns_exact_replay_divergence_when_events_mix_runs()`
Given: events contain `RunAccepted { run: RunId(1), seq: 0 }` followed by `StepStarted { run: RunId(2), seq: 1, step: StepIdx(3) }`.
When: `recover_runtime_frame_seed_from_events` runs.
Then: it returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "frame seed recovery received events for multiple runs".to_owned() })`.

### Behavior: summary recovery rejects mixed runs with exact detail
Test function: `fn summary_recovery_returns_exact_replay_divergence_when_events_mix_runs()`
Given: events contain `RunAccepted { run: RunId(1), seq: 0 }` followed by `StepStarted { run: RunId(2), seq: 1, step: StepIdx(3) }`.
When: summary recovery runs.
Then: it returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "recovery summary received events for multiple runs".to_owned() })`.

### Behavior: core replay rejects impossible ordering with exact detail
Test function: `fn core_replay_returns_exact_replay_divergence_when_step_order_goes_backward()`
Given: replay events contain `StepStarted { step: StepIdx(2) }` followed by `StepStarted { step: StepIdx(1) }` for the same run.
When: core replay runs.
Then: it returns `Err(RecoveryError::ReplayDivergence { step: StepIdx(1), detail: "step 1 executed before previous step 2".to_owned() })`.

### Behavior: empty recovery data is exact
Test function: `fn frame_seed_recovery_returns_no_recovery_data_run_zero_when_event_slice_is_empty()`
Given: `events == []`.
When: `recover_runtime_frame_seed_from_events(&events)` runs.
Then: it returns `Err(RecoveryError::NoRecoveryData { run: RunId(0) })`.

### Behavior: workflow digest mismatch is exact
Test function: `fn workflow_replay_returns_compiled_ir_digest_mismatch_when_digest_differs()`
Given: durable `RunAccepted` has `workflow: digest_a` and replay receives a compiled workflow with `digest_b`.
When: `recover_runtime_frame_seed_from_events_with_workflow` runs.
Then: it returns `Err(RecoveryError::CompiledIrDigestMismatch { expected: digest_b, found: digest_a })`.

### Behavior: frame dimension overflow is exact
Test function: `fn frame_seed_recovery_returns_frame_dimension_overflow_when_slot_index_overflows_count()`
Given: events for `RunId(42)` reference the maximum representable `SlotIdx` such that `index + 1` overflows the frame dimension count.
When: `recover_runtime_frame_seed_from_events` computes dimensions.
Then: it returns `Err(RecoveryError::FrameDimensionOverflow { run: RunId(42) })`.

### Behavior: summary boundary returns exact values
Test function: `fn runtime_recovery_boundary_summary_returns_exact_counts_when_summary_only_hydration_is_used()`
Given: `SummaryRecoveryBoundary` holds `RecoveryRuntimeSummary { run: RunId(42), first_seq: EventSeq(0), last_seq: EventSeq(4), workflow: Some(digest_a), steps_started: 2, steps_succeeded: 1, actions_scheduled: 0, actions_resolved: 0, suspensions: 0, slots_written: 0, terminal: None }`.
When: `summary()` is called.
Then: it returns exactly that struct, proving zero-slot/no-output state is not hidden by hydration.

### Behavior: durable boundary summary returns seed summary exactly
Test function: `fn durable_recovery_boundary_summary_returns_seed_summary_when_frame_seed_is_used()`
Given: a `DurableFrameRecoveryBoundary` built from a seed with `summary.steps_started == 1`, `summary.steps_succeeded == 1`, `summary.slots_written == 1`, and supported flags.
When: `summary()` is called.
Then: returned summary equals `seed.summary` field-for-field.

### Behavior: supported hydration rebuilds frame
Test function: `fn hydrate_run_frame_returns_frame_with_exact_slots_taint_steps_and_pc_when_seed_is_supported()`
Given: a supported seed for `RunId(42)` with `step_count: 4`, `slot_count: 4`, `pc: StepIdx(2)`, `steps: [RecoveredStepEntry { step: 2, state: Succeeded }]`, and `slots: [RecoveredSlotEntry { slot: 3, value: I64(99), taint: Tainted }]`.
When: `hydrate_run_frame` is called.
Then: the frame has run `42`, PC `2`, step `2` succeeded, slot `3` value `I64(99)`, and slot `3` taint `Tainted`.

### Behavior: hydration rejects unsupported slot values
Test function: `fn hydrate_run_frame_returns_invalid_recovery_hydration_when_slot_values_are_unsupported()`
Given: seed `unsupported.slot_values == true`.
When: `hydrate_run_frame` is called.
Then: it returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: hydration rejects unsupported slot taint
Test function: `fn hydrate_run_frame_returns_invalid_recovery_hydration_when_slot_taint_is_unsupported()`
Given: seed `unsupported.slot_taint == true`.
When: `hydrate_run_frame` is called.
Then: it returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: hydration rejects unsupported pending actions
Test function: `fn hydrate_run_frame_returns_invalid_recovery_hydration_when_pending_actions_are_unsupported()`
Given: seed has one `RecoveredPendingAction` and `unsupported.pending_actions == true`.
When: `hydrate_run_frame` is called.
Then: it returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: hydration rejects invalid dimensions
Test function: `fn hydrate_run_frame_returns_invalid_recovery_hydration_when_step_or_slot_count_is_invalid()`
Given: seed references `StepIdx(3)` but has `step_count: 3`, or references `SlotIdx(4)` but has `slot_count: 4`.
When: `hydrate_run_frame` applies recovered steps or slots.
Then: it returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: hydration rejects invalid PC
Test function: `fn hydrate_run_frame_returns_invalid_recovery_hydration_when_pc_is_out_of_bounds()`
Given: seed has `step_count: 3` and `pc: StepIdx(3)`.
When: `hydrate_run_frame` is called.
Then: it returns `Err(RuntimeError::InvalidRecoveryHydration)`.

### Behavior: summary-only hydration is unsupported exactly
Test function: `fn summary_boundary_returns_unsupported_full_recovery_hydration_when_live_frame_is_requested()`
Given: `SummaryRecoveryBoundary::from_summary(summary)`.
When: `hydrate_run_frame` is called.
Then: it returns `Err(RuntimeError::UnsupportedFullRecoveryHydration)`.

### Behavior: no-output step does not fabricate slot zero
Test function: `fn recovery_does_not_create_slot_zero_when_no_output_step_has_no_slot_written_event()`
Given: ordered events contain `StepStarted(step 2)` and `StepSucceeded { step: 2, output: None }` with no `SlotWrittenEvent` under the contract representation.
When: recovery builds a seed and summary.
Then: `summary.steps_succeeded == 1`, `summary.slots_written == 0`, `seed.slots == []`, and no dimension/count changes solely because of `SlotIdx::ZERO`.

## 4. Proptest Invariants

1. **Lifecycle order** — For any generated valid chain, `StepStarted(step)` precedes all slot writes and `StepSucceeded(step)` follows them. Invalid chains with success before start or slot after success return exact `RecoveryError::ReplayDivergence` fields.
2. **Runtime-to-storage mapping** — Any valid runtime slot write maps to storage with identical run, step, slot, value bytes, taint, and extra.
3. **Per-run EventSeq monotonicity** — 1..256 events for one run produce consecutive `EventSeq` values; seeding at `u64::MAX` returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::SequenceOverflow) }`.
4. **Drain report conservation** — For any queue length `n` within capacity and successful storage, drain returns `JournalWriterFlushReport { drained: n, written: n }`; second drain returns `{ drained: 0, written: 0 }`.
5. **Drain failure retention** — For any queue length `n > 0` and storage lock poison, drain returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::WriteLockPoisoned) }` and does not report written events.
6. **Slot value round trip** — Any valid bounded `SlotValue` encoded in a slot event recovers to the same `SlotValue`; arbitrary non-postcard bytes mark `slot_values` and `slot_taint` unsupported.
7. **Taint fidelity** — Any durable slot event with `taint: Some(t)` recovers exactly `t`; `taint: None` always sets `unsupported.slot_taint == true` unless workflow replay proves taint.
8. **Single-run recovery** — One-run chains never return mixed-run details; two-run chains return exact mixed-run `ReplayDivergence` detail for the recovery path under test.
9. **No-output semantics** — `StepSucceeded(output: None)` without a slot write never increases recovered slot count and never produces `SlotIdx::ZERO`.
10. **Hydration gate** — Any unsupported flag combination, invalid dimensions, invalid PC, or out-of-bounds recovered step/slot application returns `RuntimeError::InvalidRecoveryHydration`.

## 5. Fuzz Targets

1. **Slot value decoding** — Input: arbitrary bytes in `SlotWrittenEvent.value`. Risk: panic, OOM, invalid value accepted, clean default. Seeds: empty, one-byte, truncated postcard, valid `Null`, valid `I64(99)`, valid tainted scalar, max bounded payload, high-bit random.
2. **`recover_runtime_frame_seed_from_events`** — Input: bounded vector of serialized/deserialized `JournalEvent`. Risk: panic on impossible order, mixed run partial seed, dimension overflow, unsupported flag miss. Seeds: empty, only `RunAccepted`, success before start, complete chain, missing taint, corrupt value, mixed runs, no-output success.
3. **Durable journal record decode/replay** — Input: arbitrary stored record bytes. Risk: postcard decode panic, wrong `RecordKind`, `StepSucceeded` decoded as slot write, unbounded allocation. Seeds: valid lifecycle records, record-kind mismatch, truncated record, oversized payload marker.
4. **Workflow replay digest boundary** — Input: event chains plus digest bytes and workflow fixture selector. Risk: digest mismatch accepted or wrong artifact replay. Seeds: matching digest, mismatched digest, missing `RunAccepted`, corrupt digest length, valid event-only chain.

## 6. Kani Harnesses

1. **Lifecycle state machine completeness** — Bound: <=4 steps, <=4 writes/step. Prove success before start and write after success cannot be accepted as valid recovery.
2. **EventSeq overflow** — Bound: current seq in `{0, 1, u64::MAX - 1, u64::MAX}`. Prove next sequence is consecutive or returns `RuntimeError::StorageJournalAppend { source: Arc(JournalError::SequenceOverflow) }`.
3. **Drain report conservation** — Bound: queue length <=16. Prove `drained <= queued`, `written <= drained`, and successful full drain empties queue.
4. **Hydration unsupported truth table** — Bound: all booleans for `slot_values`, `slot_taint`, `action_payloads`, `pending_actions`. Prove any live-frame unsupported state returns `InvalidRecoveryHydration`.
5. **Hydration bounds** — Bound: step/slot counts <=8 and indices around 0/count/count+1. Prove invalid PC or recovered index returns `InvalidRecoveryHydration` without unchecked indexing.
6. **Frame dimension overflow** — Bound: max index at representable max. Prove `index + 1` overflow returns `RecoveryError::FrameDimensionOverflow { run }`.
7. **No-output no slot-zero** — Bound: <=8 no-output successes. Prove no recovered slot is synthesized without a slot-write event.

## 7. Mutation Testing Checkpoints

Minimum threshold: `cargo-mutants` kill rate >=90% for touched files in `crates/vb_runtime/src/journal.rs`, `crates/vb_runtime/src/shard/**`, `crates/vb_runtime/src/recovery.rs`, `crates/vb_storage/src/events.rs`, `crates/vb_storage/src/journal.rs`, `crates/vb_storage/src/queue.rs`, and `crates/vb_storage/src/recovery/**`.

Required killed mutants:

- Remove taint from slot-write mapping -> killed by exact slot/taint recovery scenarios.
- Change `taint: Some(t)` to missing/default clean -> killed by missing-taint and taint-fidelity scenarios.
- Reorder `StepSucceeded` before `SlotWritten` -> killed by lifecycle order BDD/proptest.
- Swallow append error -> killed by `shard_flush_returns_journal_poisoned_when_runtime_journal_append_is_poisoned`.
- Continue after failed append -> killed by `shard_flush_records_no_step_succeeded_when_slot_written_append_fails`.
- Allow duplicate `(RunId, EventSeq)` overwrite -> killed by duplicate sequence integration.
- Change `JournalError::Encode(_)` to generic storage success/error -> killed by exact encode scenario.
- Accept mixed runs -> killed by exact mixed-run frame seed and summary divergence scenarios.
- Alter `ReplayDivergence.step` or `detail` strings -> killed by exact field scenarios.
- Accept digest mismatch -> killed by exact digest mismatch scenario.
- Omit `FrameDimensionOverflow` branch -> killed by BDD + Kani dimension overflow.
- Convert no-output `None` to `SlotIdx::ZERO` -> killed by no-output BDD/proptest/Kani.
- Invert unsupported hydration checks -> killed by hydration BDD + truth-table Kani.
- **Drain-specific:** drop queued events but report success -> killed by drain report conservation and storage event count.
- **Drain-specific:** return wrong `drained` count -> killed by exact `JournalWriterFlushReport` assertions.
- **Drain-specific:** return wrong `written` count -> killed by exact `JournalWriterFlushReport` assertions.
- **Drain-specific:** swallow `WriteLockPoisoned` during drain -> killed by exact drain failure scenario.
- **Drain-specific:** duplicate writes on second drain -> killed by idempotent second-drain scenario.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| one-slot deterministic step | valid lifecycle | exact ordered lifecycle with value+taint | integration |
| encode failure | serializer failure | `Err(RuntimeError::EncodeFailed)` | unit |
| append poisoned | poisoned mutex | `Err(RuntimeError::JournalPoisoned)` | unit |
| strict queued append | strict profile | `Err(RuntimeError::UnsupportedAsyncStrictAck)` | unit |
| seq 0..2 | three events | `[EventSeq(0), EventSeq(1), EventSeq(2)]` | unit |
| seq overflow | `EventSeq(u64::MAX)` | `Err(RuntimeError::StorageJournalAppend { source: Arc(JournalError::SequenceOverflow) })` | unit/kani |
| default drain | no queue | `Ok(JournalWriterFlushReport { drained: 0, written: 0 })` | unit |
| successful drain | 3 queued events | `Ok(JournalWriterFlushReport { drained: 3, written: 3 })` | integration |
| failed drain | poisoned storage lock | `Err(RuntimeError::StorageJournalAppend { source: Arc(JournalError::WriteLockPoisoned) })` | integration |
| second drain | already drained | `Ok(JournalWriterFlushReport { drained: 0, written: 0 })` and no duplicate events | unit/integration |
| duplicate sequence | same run/seq different event | `Err(JournalError::DuplicateEvent { run: 42, seq: 7 })` | integration |
| storage encode | postcard encode failure | `Err(JournalError::Encode(_))` | unit |
| complete recovery | value Some, taint Some | exact `RecoveredSlotEntry`; unsupported supported | integration |
| zero-slot recovery | no slot writes | summary slots `0`, seed slots `[]` | unit |
| missing value | `value: None` | `unsupported.slot_values == true`, hydration `Err(InvalidRecoveryHydration)` | unit |
| corrupt value | invalid bytes | `unsupported.slot_values == true`, `unsupported.slot_taint == true` | unit/fuzz |
| missing taint | `taint: None` | `unsupported.slot_taint == true`; no clean default | unit |
| mixed frame seed runs | two run ids | exact `ReplayDivergence { step: ZERO, detail: "frame seed recovery received events for multiple runs" }` | unit |
| mixed summary runs | two run ids | exact `ReplayDivergence { step: ZERO, detail: "recovery summary received events for multiple runs" }` | unit |
| backward step order | step 2 then step 1 | exact `ReplayDivergence { step: 1, detail: "step 1 executed before previous step 2" }` | unit |
| no data | empty slice | `Err(RecoveryError::NoRecoveryData { run: RunId(0) })` | unit |
| digest mismatch | expected B, found A | `Err(CompiledIrDigestMismatch { expected: B, found: A })` | integration |
| dimension overflow | max index | `Err(FrameDimensionOverflow { run })` | unit/kani |
| summary-only summary | known summary | exact same `RecoveryRuntimeSummary` | unit |
| durable summary | seed summary | exact same `seed.summary` | unit |
| supported hydrate | valid seed | exact frame run/pc/step/slot/taint | integration |
| unsupported values | flag true | `Err(RuntimeError::InvalidRecoveryHydration)` | unit |
| unsupported taint | flag true | `Err(RuntimeError::InvalidRecoveryHydration)` | unit |
| pending actions unsupported | pending action + flag true | `Err(RuntimeError::InvalidRecoveryHydration)` | unit |
| invalid dimensions | index outside count | `Err(RuntimeError::InvalidRecoveryHydration)` | unit/kani |
| invalid PC | `pc >= step_count` | `Err(RuntimeError::InvalidRecoveryHydration)` | unit/kani |
| summary hydration | summary-only boundary | `Err(RuntimeError::UnsupportedFullRecoveryHydration)` | unit |
| no-output step | no slot event | no `SlotIdx::ZERO`; slot count unchanged | integration |

## 9. Static Resource, Panic, And Acceptance Checks

The implementer must collect concrete evidence for all gates below:

1. `moon ci` from workspace root passes.
2. Forbidden production-code construct scan over touched production files returns no matches: `rg -n "unsafe|\\.unwrap\\(|\\.expect\\(|panic!|todo!|unimplemented!|dbg!" crates/vb_runtime/src crates/vb_storage/src` with generated/test exceptions explicitly excluded.
3. Runtime-core serialization/network scan returns no matches: `rg -n "serde_json|serde_yaml|yaml|json|reqwest|hyper|http" crates/vb_runtime/src crates/vb_core/src` except documented projections outside runtime core.
4. `scripts/check-nightly-features.sh` or `moon run :nightly-feature-gate` passes for changed files.
5. Temp storage integration tests use owned temp directories and assert reopen behavior before drop; no persistent files under the workspace or `.beads` are created.
6. `cargo-mutants` or repository mutation workflow reports >=90% kill rate for touched files and the drain-specific mutants listed above are killed.
7. Fuzz and Kani targets at least compile; any newly discovered crash is converted into a regression scenario before close.

## Open Questions For Implementer

1. If current source still lacks contract-required `taint` or `Option<SlotIdx>` fields on `JournalEvent`, tests should first be written against the contract shape and fail red until implementation catches up.
