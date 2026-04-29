# Test and Benchmark Plan: Velvet Ballastics

## Summary
- Scope: compiled IR hot loop, numeric slots, bounded `MemoryIngress`, Fjall append-only journal, replay/durability, and proof that HTTP is not on the hot path.
- Behaviors identified: 40
- Trophy allocation target: 14 unit / 19 integration / 2 e2e / 5 static checks.
- Proptest invariants: 13
- Fuzz targets: 4
- Kani harnesses: 7
- Mutation target: `cargo-mutants` kill rate >= 90% overall and 100% for `vb-core` engine error branches.
- Style constraint: bounded, explicit, JPL/Holzmann-oriented Rust. No unbounded queues, no hidden dynamic dispatch/string lookup in hot loop, no HTTP runtime dependency.

## 1. Behavior Inventory

1. `StepBudget` returns `EngineError::EmptyStepBudget` when caller supplies zero.
2. `StepBudget` accepts non-zero execution budgets when caller supplies any `u32` greater than zero.
3. `RunFrame::new` initializes current step to workflow entry when a run is created.
4. `RunFrame::new` initializes every numeric slot to `SlotValue::Null` when a workflow declares slots.
5. `RunFrame::new` initializes every taint marker to `Taint::Clean` when a workflow declares slots.
6. `RunFrame::slot` returns the exact slot value when slot index is in bounds.
7. `RunFrame::slot` returns `EngineError::SlotOutOfBounds { slot }` when slot index is outside frame.
8. `RunFrame::taint` returns the exact taint when slot index is in bounds.
9. `RunFrame::taint` returns `EngineError::SlotOutOfBounds { slot }` when slot index is outside frame.
10. `CompiledWorkflow::node` returns a node reference when step index is in bounds.
11. `CompiledWorkflow::node` returns `None` when step index is outside the node array.
12. `CompiledWorkflow::constant` returns a constant reference when constant index is in bounds.
13. `CompiledWorkflow::constant` returns `None` when constant index is outside the constant pool.
14. `step_once` returns `EngineError::InvalidProgramCounter { step }` when run program counter references no node.
15. `step_once` writes a constant to the configured output slot when executing `SetConst`.
16. `CompiledWorkflow::try_from_parts` makes missing `SetConst` output unrepresentable by requiring `CompiledNodeKind::SetConst { output, .. }`.
17. `step_once` returns `EngineError::ConstOutOfBounds { constant }` when `SetConst` references a missing constant.
18. `step_once` returns `EngineError::SlotOutOfBounds { slot }` when `SetConst` writes outside the frame.
19. `step_once` copies exact slot value and taint when executing `Copy`.
20. `CompiledWorkflow::try_from_parts` makes missing `Copy` output unrepresentable by requiring `CompiledNodeKind::Copy { output, .. }`.
21. `step_once` returns `EngineError::SlotOutOfBounds { slot }` when `Copy` source or output is outside the frame.
22. `step_once` branches to `on_true` when `Choose` condition slot contains `SlotValue::Bool(true)`.
23. `step_once` branches to `on_false` when `Choose` condition slot contains `SlotValue::Bool(false)`.
24. `step_once` returns `EngineError::NonBoolCondition { slot }` when `Choose` condition slot is not boolean.
25. `step_once` returns `EngineError::SlotOutOfBounds { slot }` when `Choose` condition slot is outside the frame.
26. `step_once` returns `EngineSignal::Finished(exact_value)` when `Finish` reads a valid result slot.
27. `step_once` returns `EngineError::SlotOutOfBounds { slot }` when `Finish` result slot is outside the frame.
28. `run_until_blocked` returns `EngineSignal::BudgetExhausted` when budget expires before finish.
29. `run_until_blocked` returns `EngineSignal::Finished(exact_value)` when workflow finishes inside budget.
30. `run_until_blocked` propagates the exact `EngineError` returned by `step_once` when a node is invalid.
31. `SlotValue::is_true` returns `true` only for `SlotValue::Bool(true)` when called on any slot value.
32. `MemoryIngress::bounded` creates a bounded queue with initial `len() == 0` and `is_empty() == true`.
33. `MemoryIngress::try_submit` accepts frames until capacity is reached.
34. `MemoryIngress::try_submit` returns `IpcError::Full` immediately when bounded capacity is full.
35. `MemoryIngress::try_recv` returns frames in FIFO order when frames are queued.
36. `MemoryIngress::try_recv` returns `Ok(None)` when queue is empty.
37. `MemoryIngress` never exceeds configured capacity under concurrent producers.
38. `FjallJournal::append_journaled` stores postcard-encoded `JournalEvent` under fixed-width `[RunId_16B | EventSeq_8B]` key when event is appended without an immediate fsync barrier.
39. `FjallJournal::append_strict` stores the event and creates a strict durability barrier before returning.
40. The runtime hot path contains no HTTP, YAML, JSON routing, unbounded channel, per-step task spawn, or blocking filesystem call in deterministic `vb-core` transitions.

## 2. Trophy Allocation

| Behavior(s) | Layer | Test names | Rationale |
|---|---|---|---|
| 1-2 | Unit | `step_budget_returns_empty_step_budget_when_zero`, `step_budget_accepts_max_u32_when_non_zero` | Pure bounded value construction. |
| 3-9 | Unit | `run_frame_initializes_entry_slots_and_taint_when_created`, `run_frame_returns_slot_out_of_bounds_when_slot_missing`, `run_frame_returns_taint_out_of_bounds_when_slot_missing` | Numeric slot invariants are local and exhaustive. |
| 10-13 | Unit | `compiled_workflow_returns_node_when_step_in_bounds`, `compiled_workflow_returns_none_when_step_out_of_bounds`, `compiled_workflow_returns_constant_when_index_in_bounds`, `compiled_workflow_returns_none_when_constant_out_of_bounds` | Immutable IR lookup behavior. |
| 14-31 | Unit + integration | Engine node micro-tests plus compiled mini-workflow tests named below | Hot loop must be tested both per transition and as real workflow slices. |
| 32-37 | Integration | `memory_ingress_*` names below | Uses real crossbeam bounded channel and real producer threads. |
| 38-39 | Integration | `fjall_journal_*` names below | Uses real temporary Fjall database and process reopen. |
| 40 | Static + e2e | `hot_path_has_no_forbidden_dependencies`, `cli_help_declares_no_http_hot_path` | Architecture boundary is enforced by dependency graph/source scan plus binary surface. |

## 3. BDD Scenarios

### Behavior: Step budget is non-zero
- Test: `fn step_budget_returns_empty_step_budget_when_zero()`
  - Given: no engine state.
  - When: constructing `StepBudget::new(0)`.
  - Then: result equals `Err(EngineError::EmptyStepBudget)`.
- Test: `fn step_budget_accepts_max_u32_when_non_zero()`
  - Given: no engine state.
  - When: constructing `StepBudget::new(u32::MAX)`.
  - Then: subsequent `run_until_blocked` with a one-step finish workflow can consume the budget and returns `EngineSignal::Finished(expected_value)`.

### Behavior: Run frame numeric slots initialize deterministically
- Test: `fn run_frame_initializes_entry_slots_and_taint_when_created()`
  - Given: a compiled workflow with entry `StepIdx::new(2)` and three slots.
  - When: creating `RunFrame::new(RunId::new(9), &workflow)`.
  - Then: `id()` equals `RunId::new(9)`, `current_step()` equals `StepIdx::new(2)`, `steps_executed()` equals `0`, slots 0..2 equal `SlotValue::Null`, and taints 0..2 equal `Taint::Clean`.
- Test: `fn run_frame_returns_slot_out_of_bounds_when_slot_missing()`
  - Given: a frame with one slot.
  - When: reading `SlotIdx::new(1)`.
  - Then: result equals `Err(EngineError::SlotOutOfBounds { slot: SlotIdx::new(1) })`.
- Test: `fn run_frame_returns_taint_out_of_bounds_when_slot_missing()`
  - Given: a frame with one slot.
  - When: reading taint for `SlotIdx::new(1)`.
  - Then: result equals `Err(EngineError::SlotOutOfBounds { slot: SlotIdx::new(1) })`.

### Behavior: Compiled IR lookup is checked
- Test: `fn compiled_workflow_returns_node_when_step_in_bounds()` — Then: returned node equals the exact `CompiledNode` inserted at that index.
- Test: `fn compiled_workflow_returns_none_when_step_out_of_bounds()` — Then: returned value equals `None` for `StepIdx::new(node_count)`.
- Test: `fn compiled_workflow_returns_constant_when_index_in_bounds()` — Then: returned constant equals exact expected `SlotValue`.
- Test: `fn compiled_workflow_returns_none_when_constant_out_of_bounds()` — Then: returned value equals `None` for `ConstIdx::new(constant_count)`.

### Behavior: Hot loop executes `SetConst`
- Test: `fn step_once_writes_constant_and_advances_when_set_const_is_valid()`
  - Given: a workflow with `SetConst { value: ConstIdx::new(0) }`, output slot 0, next step 1, and constant `SlotValue::I64(42)`.
  - When: executing `step_once`.
  - Then: signal equals `EngineSignal::Continue`, slot 0 equals `SlotValue::I64(42)`, taint 0 equals `Taint::Clean`, current step equals `StepIdx::new(1)`, and steps executed equals `1`.
- Compile-time shape: `SetConst` cannot be constructed without an `output: SlotIdx`; runtime missing-output tests are obsolete.
- Test: `fn step_once_returns_const_out_of_bounds_when_set_const_constant_missing()` — Then: exact error `EngineError::ConstOutOfBounds { constant: ConstIdx::new(1) }`.
- Test: `fn step_once_returns_slot_out_of_bounds_when_set_const_output_missing()` — Then: exact error `EngineError::SlotOutOfBounds { slot: SlotIdx::new(1) }`.

### Behavior: Hot loop executes `Copy`
- Test: `fn step_once_copies_value_and_taint_when_copy_is_valid()`
  - Given: slot 0 contains `SlotValue::Text("secret".into())` and taint `Taint::DerivedFromSecret` after prior valid setup.
  - When: executing `Copy { source: SlotIdx::new(0) }` to output slot 1.
  - Then: slot 1 equals `SlotValue::Text("secret".into())`, taint 1 equals `Taint::DerivedFromSecret`, current step advances, and signal equals `EngineSignal::Continue`.
- Compile-time shape: `Copy` cannot be constructed without an `output: SlotIdx`; runtime missing-output tests are obsolete.
- Test: `fn step_once_returns_slot_out_of_bounds_when_copy_source_missing()` — Then: exact error `EngineError::SlotOutOfBounds { slot: missing_source }`.
- Test: `fn step_once_returns_slot_out_of_bounds_when_copy_output_missing()` — Then: exact error `EngineError::SlotOutOfBounds { slot: missing_output }`.

### Behavior: Hot loop executes `Choose`
- Test: `fn step_once_jumps_to_true_target_when_condition_is_true()` — Then: current step equals `on_true` and signal equals `EngineSignal::Continue`.
- Test: `fn step_once_jumps_to_false_target_when_condition_is_false()` — Then: current step equals `on_false` and signal equals `EngineSignal::Continue`.
- Test: `fn step_once_returns_non_bool_condition_when_condition_is_i64()` — Then: exact error `EngineError::NonBoolCondition { slot: condition }`.
- Test: `fn step_once_returns_non_bool_condition_when_condition_is_null_text_or_bytes()` — Then: each non-bool class returns exact `EngineError::NonBoolCondition { slot: condition }`.
- Test: `fn step_once_returns_slot_out_of_bounds_when_choose_condition_missing()` — Then: exact error `EngineError::SlotOutOfBounds { slot: condition }`.

### Behavior: Hot loop executes `Finish` and budgets
- Test: `fn step_once_returns_finished_value_when_finish_result_slot_exists()` — Then: signal equals `EngineSignal::Finished(exact_slot_value)`.
- Test: `fn step_once_returns_slot_out_of_bounds_when_finish_result_missing()` — Then: exact error `EngineError::SlotOutOfBounds { slot: result }`.
- Test: `fn run_until_blocked_returns_finished_when_workflow_finishes_within_budget()` — Then: result equals `Ok(EngineSignal::Finished(SlotValue::I64(42)))` and steps executed equals exact transition count before finish.
- Test: `fn run_until_blocked_returns_budget_exhausted_when_chain_exceeds_budget()` — Then: result equals `Ok(EngineSignal::BudgetExhausted)` and steps executed equals the budget count.
- Test: `fn run_until_blocked_propagates_invalid_program_counter_when_pc_out_of_bounds()` — Then: exact error `EngineError::InvalidProgramCounter { step }`.

### Behavior: Slot values expose boolean truth only
- Test: `fn slot_value_is_true_returns_true_only_for_bool_true()`
  - Given: `Null`, `Bool(false)`, `Bool(true)`, `I64(1)`, non-empty `Text`, and non-empty `Bytes`.
  - When: calling `is_true`.
  - Then: only `Bool(true)` returns `true`; every other exact case returns `false`.

### Behavior: Bounded MemoryIngress provides non-blocking backpressure
- Test: `fn memory_ingress_starts_empty_when_bounded_queue_created()` — Then: `len()` equals `0` and `is_empty()` equals `true`.
- Test: `fn memory_ingress_accepts_frames_until_capacity_is_reached()` — Then: each submit up to capacity returns `Ok(())` and `len()` equals capacity.
- Test: `fn memory_ingress_returns_full_when_capacity_is_exhausted()` — Then: overflow submit returns `Err(IpcError::Full)` and `len()` remains capacity.
- Test: `fn memory_ingress_receives_frames_in_fifo_order_when_frames_are_queued()` — Then: received frames equal the exact submitted frames in order.
- Test: `fn memory_ingress_returns_none_when_queue_is_empty()` — Then: result equals `Ok(None)`.
- Test: `fn memory_ingress_never_exceeds_capacity_when_multiple_producers_submit_concurrently()` — Then: successful submissions count <= capacity, `IpcError::Full` count equals attempted minus successful, and final `len()` equals successful count.
- Test: `fn memory_ingress_reports_disconnected_when_receiver_or_sender_side_is_dropped()` — If API visibility permits constructing a disconnected instance or a test-only harness, Then: exact `Err(IpcError::Disconnected)`.

### Behavior: Fjall append-only journal stores ordered durable binary events
- Test: `fn journal_event_returns_run_id_and_seq_for_every_variant()` — Then: each variant returns exact embedded `RunId` and `EventSeq`.
- Test: `fn journal_key_is_24_bytes_and_big_endian_ordered()` — Then: key bytes equal `[run.to_be_bytes(), seq.to_be_bytes()].concat()` and lexical ordering matches numeric `(run, seq)` ordering.
- Test: `fn fjall_journal_reopens_appended_event_when_persist_strict_completed()`
  - Given: a temporary journal path and a `RunAccepted` event.
  - When: appending, calling `persist_strict`, dropping journal, reopening database through public/test read helper.
  - Then: decoded event equals the exact original `JournalEvent::RunAccepted { run, seq, workflow }`.
- Test: `fn fjall_journal_preserves_per_run_sequence_order_when_multiple_events_appended()` — Then: range scan/replay yields seq `[0, 1, 2, 3]` for the run with exact variant order.
- Test: `fn fjall_journal_keeps_runs_separated_when_run_ids_differ()` — Then: replay for run A returns only run A events and replay for run B returns only run B events.
- Test: `fn fjall_journal_returns_fjall_error_when_path_cannot_be_opened()` — Then: result matches `Err(JournalError::Fjall(_))` for a non-directory/permission-denied path.
- Test: `fn fjall_journal_never_uses_json_as_durable_record_when_event_appended()` — Then: persisted value decodes by `postcard` to exact event and does not begin with `{` or `[` JSON delimiters for seed events.

### Behavior: Replay and durability rebuild state from journal
- Test: `fn replay_rebuilds_finished_run_when_ordered_events_are_complete()`
  - Given: persisted `RunAccepted`, deterministic `StepStarted`/`StepSucceeded`, and `RunFinished` events for one run.
  - When: replaying from ordered journal events.
  - Then: replay state equals `Finished { run, result_slot }` with last applied seq equal to final event seq.
- Test: `fn replay_rejects_or_quarantines_gap_when_event_sequence_skips_value()`
  - Given: events with seq `0, 1, 3`.
  - When: replaying.
  - Then: exact replay error should be `JournalError::SequenceGap { expected: EventSeq::new(2), actual: EventSeq::new(3) }` from `events_for_run`.
- Test: `fn replay_is_idempotent_when_same_journal_is_replayed_twice()` — Then: first replay state equals second replay state byte-for-byte/logically.

### Behavior: No HTTP hot path
- Test: `fn hot_path_has_no_forbidden_dependencies()`
  - Given: workspace manifests.
  - When: checking `vb-core`, `vb-ipc`, and `vb-storage` dependency trees.
  - Then: no dependency name equals `hyper`, `axum`, `actix-web`, `reqwest`, `tower-http`, `http`, `serde_json`, `serde_yaml`, or `tokio` in hot-path crates.
- Test: `fn vb_core_contains_no_async_spawn_or_blocking_filesystem_calls()`
  - Given: `crates/vb-core/src/**/*.rs`.
  - When: scanning source text in an architecture test.
  - Then: forbidden tokens `async fn`, `.await`, `tokio::spawn`, `std::fs`, `serde_json`, `serde_yaml`, and `HashMap<String` are absent.
- Test: `fn cli_help_declares_no_http_hot_path()`
  - Given: installed/debug binary.
  - When: running `velvet-ballastics help`.
  - Then: stdout contains `compiled IR`, `bounded IPC`, `Fjall journal`, and `no HTTP hot path`.

## 4. Proptest Invariants

1. `StepBudget::new`: for any `value: u32`, returns `Err(EngineError::EmptyStepBudget)` iff `value == 0`; otherwise accepted and usable.
2. `StepIdx::new/as_usize`: for any `u16`, `StepIdx::new(n).as_usize() == usize::from(n)`.
3. `SlotIdx::new/as_usize`: for any `u16`, `SlotIdx::new(n).as_usize() == usize::from(n)`.
4. `ConstIdx::new/as_usize`: for any `u16`, `ConstIdx::new(n).as_usize() == usize::from(n)`.
5. `RunId::new/as_u128`: for any `u128`, round-trip returns same integer.
6. `WorkflowDigest::from_bytes/as_bytes`: for any `[u8; 32]`, round-trip returns same bytes.
7. `SlotValue::is_true`: for any generated `SlotValue`, result equals `matches!(value, SlotValue::Bool(true))`.
8. `CompiledWorkflow::node`: for any non-empty node vector and any `u16`, result is `Some(exact_node)` iff index < node count, otherwise `None`.
9. `CompiledWorkflow::constant`: for any constant vector and any `u16`, result is `Some(exact_constant)` iff index < constant count, otherwise `None`.
10. `run_until_blocked` deterministic chain: for any chain length `1..=1024` and budget `>= chain transitions`, final value equals last constant and steps executed equals expected transition count.
11. `run_until_blocked` budget: for any chain length `2..=1024` and budget `< required transitions`, signal equals `EngineSignal::BudgetExhausted` and steps executed equals budget.
12. `MemoryIngress`: for any capacity `1..=1024` and submit count `0..=2048`, successful submit count equals `min(capacity, submit_count)`, overflow count equals `submit_count.saturating_sub(capacity)`, and receive order equals submitted prefix.
13. `journal_key`: for any pair `(run_a, seq_a) < (run_b, seq_b)` in numeric tuple order, key_a lexicographically precedes key_b.

Anti-invariant classes that must fail with exact errors: zero budget, out-of-range slot, out-of-range constant, out-of-range program counter, non-bool choose condition, full ingress queue, impossible journal open path.

## 5. Fuzz Targets

1. `fuzz_postcard_journal_event_decode`
   - Input type: arbitrary bytes.
   - Risk: panic, excessive allocation, accepting malformed events that violate variant shape.
   - Corpus seeds: postcard encodings for all four `JournalEvent` variants; empty bytes; one-byte inputs; truncated valid event; valid event with trailing garbage; max `RunId`/`EventSeq`.
   - Expected: decode either returns exact valid `JournalEvent` or `postcard::Error`; never panics or OOMs.
2. `fuzz_ingress_frame_payload_boundary`
   - Input type: arbitrary `Bytes` payload plus generated `RunId`/`WorkflowDigest`.
   - Risk: unbounded allocation assumptions and binary payload edge cases at IPC boundary.
   - Corpus seeds: empty payload, `{}`, invalid UTF-8, 1 byte, 4 KiB, `MaxPayloadBytes::DEFAULT`, and one byte over `MaxPayloadBytes::new(NonZeroUsize::new(4096))`.
   - Expected: queue stores and returns exact byte payload or exact `IpcError::Full`; no parsing in hot ingress.
3. `fuzz_cli_argument_parser`
   - Input type: arbitrary OS-string-compatible bytes/strings.
   - Risk: panic on invalid or unknown command input.
   - Corpus seeds: `help`, `--help`, `-h`, `version`, `--version`, `-V`, empty, invalid UTF-8 where platform permits.
   - Expected: command selection is help or version only; no panic.
4. Future `fuzz_yaml_to_compiled_ir_cold_boundary`
   - Input type: YAML bytes.
   - Risk: parser panic, unbounded expansion, accepting cyclic/backward graph or string refs into hot path.
   - Corpus seeds: minimal workflow, max slots, invalid UTF-8, anchors/aliases, deeply nested YAML, unknown keys, backward graph cycle.
   - Expected: exact validation error variants; accepted output contains numeric `StepIdx`/`SlotIdx` only.

## 6. Kani Harnesses

1. `kani_step_budget_zero_iff_empty_error`
   - Property: `StepBudget::new(n)` rejects exactly `n == 0`.
   - Bound: all `u32` values.
   - Rationale: budget is a safety guard for bounded execution.
2. `kani_numeric_index_widening_preserves_value`
   - Property: `StepIdx`, `SlotIdx`, and `ConstIdx` `as_usize()` equals exact `u16` widening.
   - Bound: all `u16` values.
   - Rationale: no unchecked/narrowing index math in hot loop.
3. `kani_journal_key_is_fixed_width`
   - Property: journal key construction always yields exactly 24 bytes.
   - Bound: all `u128` run IDs and all `u64` seq values.
   - Rationale: Fjall key ordering/durability depends on fixed layout.
4. `kani_journal_key_order_matches_numeric_tuple_order`
   - Property: big-endian key bytes preserve `(RunId, EventSeq)` ordering.
   - Bound: symbolic `u128/u64` pairs with assume pair A < pair B.
   - Rationale: replay range scans rely on ordering.
5. `kani_run_until_blocked_budget_never_executes_more_than_budget`
   - Property: for bounded acyclic workflows up to 8 nodes, steps executed delta <= budget.
   - Bound: 8 nodes, budget 1..=8.
   - Rationale: Holzmann boundedness guarantee.
6. `kani_choose_branch_targets_are_exact`
   - Property: boolean condition true selects `on_true`; false selects `on_false`.
   - Bound: symbolic bool and symbolic target indices within 8 nodes.
   - Rationale: branch correctness in compiled IR state machine.
7. `kani_memory_ingress_capacity_accounting_model`
   - Property: abstract model of submit/recv never exceeds capacity.
   - Bound: capacity 1..=8, operations length <= 16.
   - Rationale: bounded ingress is a safety contract; use a model if crossbeam cannot be verified directly.

## 7. Mutation Testing Checkpoints

Threshold: `cargo mutants --workspace` must kill >= 90% of mutations. `vb-core::engine` branch/error mutations must be 100% killed.

- Change `value == 0` to `value != 0` in `StepBudget::new` -> killed by `step_budget_returns_empty_step_budget_when_zero` and `step_budget_accepts_max_u32_when_non_zero`.
- Remove `checked_add` error path in `jump_to` -> killed by Kani harness or targeted overflow harness once a test-only near-overflow constructor exists.
- Change `plan.node(pc).ok_or(EngineError::InvalidProgramCounter { step: pc })` to default first node -> killed by `run_until_blocked_propagates_invalid_program_counter_when_pc_out_of_bounds`.
- Replace variant-specific `output: SlotIdx` fields with optional output -> killed by IR validation/compile-fail contract tests.
- Change constant index used by `SetConst` -> killed by `step_once_writes_constant_and_advances_when_set_const_is_valid`.
- Drop taint copy in `Copy` -> killed by `step_once_copies_value_and_taint_when_copy_is_valid`.
- Swap `on_true` and `on_false` in `Choose` -> killed by true/false branch tests.
- Treat non-bool as false in `Choose` -> killed by `step_once_returns_non_bool_condition_when_condition_is_i64`.
- Return `Continue` instead of `Finished` -> killed by finish and run-until-blocked exact signal tests.
- Change `while remaining > 0` boundary -> killed by budget exhausted and exact step count tests.
- Change `TrySendError::Full` mapping -> killed by `memory_ingress_returns_full_when_capacity_is_exhausted`.
- Change FIFO receive to drop/reorder in IPC wrapper -> killed by `memory_ingress_receives_frames_in_fifo_order_when_frames_are_queued`.
- Change journal key to little-endian -> killed by `journal_key_is_24_bytes_and_big_endian_ordered` and proptest order invariant.
- Omit seq bytes from journal key -> killed by fixed-width and multi-event ordering tests.
- Remove `persist_strict` call to `SyncAll` -> killed by durability/reopen test plus benchmark validation of strict path.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| budget happy path | `1..=u32::MAX` | accepted budget usable to finish exact workflow | unit/property |
| budget zero | `0` | `Err(EngineError::EmptyStepBudget)` | unit/property |
| numeric slot min | `SlotIdx::new(0)` in bounds | exact `SlotValue`/`Taint` | unit |
| numeric slot max valid | `slot_count - 1` | exact `SlotValue`/`Taint` | unit/property |
| numeric slot first invalid | `slot_count` | `Err(EngineError::SlotOutOfBounds { slot })` | unit/property |
| save const happy | valid constant/output | `EngineSignal::Continue`; exact slot value | unit/integration |
| save const missing output | impossible IR shape | compile-time/type-level rejection | unit/compile-fail |
| save const missing constant | invalid `ConstIdx` | `Err(EngineError::ConstOutOfBounds { constant })` | unit |
| copy happy | valid source/output | exact copied value and taint | unit/integration |
| choose true | `SlotValue::Bool(true)` | current step equals `on_true` | unit |
| choose false | `SlotValue::Bool(false)` | current step equals `on_false` | unit |
| choose invalid type | Null/I64/Text/Bytes | `Err(EngineError::NonBoolCondition { slot })` | unit/property |
| finish happy | valid result slot | `EngineSignal::Finished(exact_value)` | unit/integration |
| run budget exhausted | chain longer than budget | `EngineSignal::BudgetExhausted`; exact step count | integration/property |
| ingress empty | no submitted frames | `Ok(None)`, `len() == 0` | integration |
| ingress full | capacity N, N+1 submits | final submit `Err(IpcError::Full)` | integration/property |
| ingress concurrency | many producers | successes <= capacity; no blocking | integration/stress |
| journal event variants | all variants | exact `run_id()` and `seq()` | unit/property |
| journal key boundaries | run/seq min/max | 24 bytes big-endian | unit/property/Kani |
| durability reopen | append + strict persist + reopen | decoded event equals original | integration |
| no HTTP hot path | dependency/source graph | forbidden dependencies/tokens absent | static/e2e |

## 9. Benchmark Plan

Use Criterion for micro/throughput benchmarks and Iai-Callgrind or `perf stat` for instruction/cache regression checks. Benchmarks must run with default `bench` profile and max-performance profile; record CPU model, governor, rustc nightly, and `RUSTFLAGS`.

### Engine hot loop benchmarks
- `bench_engine_step_once_save_const_single_transition`: one `SetConst` node compiled from public `save`, assert throughput in transitions/sec and ns/transition.
- `bench_engine_run_save_chain_10_steps`: 10 deterministic `SetConst`/`Copy` IR steps ending in `Finish`.
- `bench_engine_run_save_chain_1000_steps`: 1,000 deterministic steps; reports ns/step and branch miss rate with `perf`.
- `bench_engine_choose_true_branch`: boolean true branch with hot condition slot.
- `bench_engine_choose_false_branch`: boolean false branch with hot condition slot.
- `bench_engine_finish_no_observability`: minimal run to finish with no journal/observability calls.
- `bench_engine_numeric_slots_read_write_i64`: repeated checked slot writes/reads over compact numeric slots.
- `bench_engine_numeric_slots_copy_bytes_arc_cost`: copies `Bytes` payload through slot to quantify clone/reference cost.

### IPC benchmarks
- `bench_memory_ingress_try_submit_capacity_1024`: producer-only submit until full; report frames/sec.
- `bench_memory_ingress_submit_recv_single_thread`: submit+recv pair loop; report round trips/sec.
- `bench_memory_ingress_mpsc_4_producers_capacity_4096`: four producer threads, one consumer, bounded queue; report p50/p95 submit latency and full-rate.
- `bench_memory_ingress_backpressure_full_queue`: full queue submit latency; must remain immediate/non-blocking.

### Fjall journal and replay benchmarks
- `bench_fjall_append_run_accepted_no_persist`: append only, no explicit barrier.
- `bench_fjall_append_step_events_no_persist_batch_1000`: 1,000 append batch, no strict persist.
- `bench_fjall_group_commit_100_events`: append 100 events then `persist_strict`; report events/sec and barrier latency.
- `bench_fjall_strict_persist_each_event`: append + `persist_strict` per event; report p50/p95/p99 latency.
- `bench_replay_ordered_journal_1000_events`: replay ordered events for one run.
- `bench_replay_interleaved_runs_100x100_events`: replay range/prefix workload with 100 runs.
- `bench_jsonl_projection_cold_path_1000_events`: postcard event to JSONL projection outside hot loop; keep separate from engine benchmarks.

### Architecture regression benchmarks/checks
- `bench_no_http_hot_path_dependency_baseline`: not a timing benchmark; stores dependency graph artifact proving no HTTP crates in hot path.
- `bench_engine_vs_journal_boundary`: run deterministic chain with and without journal append to prove disk barrier is not hidden inside pure hot-loop benchmark.

Benchmark acceptance gates:
- Any PR claiming speed must include before/after numbers for affected benchmark names above.
- Engine hot-loop benchmarks must not include YAML parsing, JSON/JSONL projection, Fjall `persist`, HTTP routing, or task spawning.
- Storage benchmarks must label durability mode: `memory`, `journaled`, `group_commit`, or `strict`.
- Regressions > 5% in hot-loop ns/step or ingress throughput require an explicit acceptance note.

## 10. Validation Commands

Static and unit/integration:

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly test --workspace --all-targets
cargo +nightly test -p vb-core --lib
cargo +nightly test -p vb-ipc --lib
cargo +nightly test -p vb-storage --lib
cargo +nightly run -p velvet-ballastics -- help
cargo +nightly run -p velvet-ballastics -- version
cargo tree -p vb-core
cargo tree -p vb-ipc
cargo tree -p vb-storage
```

Property, fuzz, model checking, mutation:

```bash
cargo +nightly test --workspace --features proptest
cargo +nightly fuzz run fuzz_postcard_journal_event_decode
cargo +nightly fuzz run fuzz_ingress_frame_payload_boundary
cargo +nightly fuzz run fuzz_cli_argument_parser
cargo +nightly kani -p vb-core
cargo +nightly kani -p vb-storage
cargo mutants --workspace --minimum-test-timeout 20 --timeout-multiplier 4
```

Benchmarks:

```bash
cargo +nightly bench --workspace
cargo +nightly bench -p vb-core --bench engine_hot_loop
cargo +nightly bench -p vb-ipc --bench memory_ingress
cargo +nightly bench -p vb-storage --bench fjall_journal
RUSTFLAGS="-C target-cpu=native" cargo +nightly bench --workspace --profile maxperf
perf stat -d cargo +nightly bench -p vb-core --bench engine_hot_loop
```

Architecture guardrails:

```bash
cargo tree -p vb-core | tee target/vb-core-tree.txt
cargo tree -p vb-ipc | tee target/vb-ipc-tree.txt
cargo tree -p vb-storage | tee target/vb-storage-tree.txt
```

The architecture test suite must fail if the generated dependency trees contain `hyper`, `axum`, `actix-web`, `reqwest`, `tower-http`, `serde_json`, `serde_yaml`, or `tokio` in hot-path crates.

## Open Questions for Implementers

1. Extend `FjallJournal::events_for_run` coverage to include persisted reopen, per-run separation, and exact `JournalError::SequenceGap` handling for skipped sequence numbers.
2. Add a higher-level replay hydrator only when the runtime state model grows beyond raw journal event replay; until then tests should use `events_for_run` instead of private fields.
3. Expand ingress payload fuzzing around `MaxPayloadBytes::DEFAULT`, explicit `MaxPayloadBytes::new(NonZeroUsize::new(4096))` limits, and max+1 rejection.
4. Decide whether disconnected `MemoryIngress` can be tested through public API; if not, remove `Disconnected` from public error or expose a safe split sender/receiver API.
