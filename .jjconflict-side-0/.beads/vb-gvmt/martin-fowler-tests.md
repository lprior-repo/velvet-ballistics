# Martin Fowler Test Plan: Generated Rust Semantic Parity

These scenarios are executable specifications. Implementation agents may choose exact Rust harness names, but must preserve the fixture values, error variants, and journal ordering below unless the runtime type names differ; any rename must be mapped one-to-one and reviewed.

## Shared Fixture Conventions
- Run ids and tickets are Fowler-level scenario vocabulary. The current generated
  API validates generated-local identity (`step`, `action_id`, `output_slot`,
  `ask_step`, `resume_step`, pending resume state) and relies on the outer runtime
  boundary to route run/ticket ownership.
- Run id: `RunId(1)`.
- First step: `StepIdx(0)` unless stated.
- Slots:
  - Slot `0`: action input or initial value.
  - Slot `1`: action output/result slot.
  - Slot `2`: finish result source when needed.
  - Slot `3`: ask answer slot.
- Values:
  - `SlotValue::I64(41)` for action input.
  - `SlotValue::I64(42)` for action output/result.
  - `SlotValue::Symbol(9001)` for ask answer.
- Taints: `Clean`, `DerivedFromSecret`, `Secret`.
- Action id: `ActionId(7)`.
- Action ticket: `ActionTicket(100)` or structurally equivalent runtime ticket with run `1`, step `1`, action `7`, output slot `1`, resume pc `2`.
- Ask ticket: `AskTicket(200)` or structurally equivalent runtime ticket with run `1`, step `1`, answer slot `3`, resume pc `2`.
- Required journal event names: `SlotWritten`, `ActionScheduled`, `ActionCompleted`, `RunFinished`; `AskAnswered` is required if the runtime schema supports it, otherwise the test must assert the approved substitute observation.

## Happy Path Tests

### given_suspended_action_when_completion_payload_matches_then_slot_written_action_completed_and_run_finished
Given:
- A supported workflow with steps:
  1. step `0`: `SetConst` writes slot `0 = I64(41)` with taint `Secret`.
  2. step `1`: `Do` action `ActionId(7)` reads input slot `0`, writes output slot `1`, suspends with resume pc `2`.
  3. step `2`: `Finish` reads slot `1`.
- The action contract mode permits output taint `DerivedFromSecret` for tainted input, or the fixture uses the runtime-equivalent non-clean conservative output.
When:
- Generated runner executes until suspension.
- A matching completion payload arrives: run `1`, step `1`, action `7`, ticket `100`, output slot `1`, output value `I64(42)`, output taint `DerivedFromSecret`.
- Generated runner resumes.
Then:
- Suspension observation is exactly `DriveError::ActionSuspend { step: 1, action_id: 7, input_slot: 0, resume_pc: 2 }` or runtime-equivalent suspension signal.
- Final observation is `Finished(SlotValue::I64(42), Taint::DerivedFromSecret)`.
- Slot `1` is exactly `I64(42)`.
- Taint for slot `1` is exactly `DerivedFromSecret`.
- Normalized journal events are exactly:
  1. `SlotWritten { run: 1, step: 0, slot: 0, value: I64(41), taint: Secret }`
  2. `ActionScheduled { run: 1, step: 1, action: 7, input_slot: 0, ticket: 100, resume_pc: 2 }`
  3. `SlotWritten { run: 1, step: 1, slot: 1, value: I64(42), taint: DerivedFromSecret }`
  4. `ActionCompleted { run: 1, step: 1, action: 7, ticket: 100, output_slot: 1 }`
  5. `RunFinished { run: 1, step: 2, result: I64(42), taint: DerivedFromSecret }`
- IR runner produces the same normalized terminal result, slot/taint state, and journal sequence.

### given_pending_ask_when_answer_payload_matches_then_answer_slot_written_and_run_advances
Given:
- A supported workflow with steps:
  1. step `0`: `Ask` uses prompt slot `0`, suspends with ask ticket `200`, answer slot `3`, resume pc `1` or paired `AskResume` step `1`.
  2. step `1`: `AskResume` writes answer slot `3` and advances to step `2`.
  3. step `2`: `Finish` reads slot `3`.
- Slot `0` contains a valid prompt handle; prompt content is not semantically inspected by this test.
When:
- Generated runner executes to ask suspension.
- A matching ask answer payload arrives: run `1`, suspended step `0`, ticket `200`, answer slot `3`, value `Symbol(9001)`, taint `Clean`.
- Generated runner resumes through `AskResume`.
Then:
- The answer slot `3` is exactly `Symbol(9001)`.
- Taint for slot `3` is exactly `Clean`.
- Final observation is `Finished(SlotValue::Symbol(9001), Taint::Clean)`.
- Journal ordering is exactly one of the two approved forms:
  - Preferred if `AskAnswered` exists:
    1. `AskAnswered { run: 1, step: 0, ticket: 200, answer_slot: 3, taint: Clean }`
    2. `SlotWritten { run: 1, step: 1, slot: 3, value: Symbol(9001), taint: Clean }`
    3. `RunFinished { run: 1, step: 2, result: Symbol(9001), taint: Clean }`
  - Waiver candidate only if no `AskAnswered` event exists:
    1. `SlotWritten { run: 1, step: 1, slot: 3, value: Symbol(9001), taint: Clean, source: AskResume(ticket: 200) }`
    2. `RunFinished { run: 1, step: 2, result: Symbol(9001), taint: Clean }`
- The generated pc after resume advances to the same next step as IR, never re-enters the suspended ask step.

### given_finish_reads_secret_tainted_result_slot_when_generated_run_finishes_then_finished_carries_secret_taint
Given:
- A workflow writes slot `2 = I64(42)` with taint `Secret` and then `Finish` reads slot `2`.
When:
- Generated and IR runners execute to completion.
Then:
- Both return `Finished(SlotValue::I64(42), Taint::Secret)`.
- `RunFinished` contains result `I64(42)` and taint `Secret`.
- There is no validation/runtime rejection solely because the finish result is tainted.

## Error Path Tests

### given_no_contract_deterministic_action_with_secret_input_when_clean_output_returns_then_taint_violation
Given:
- A supported workflow with:
  1. step `0`: writes slot `0 = I64(41)` with taint `Secret`.
  2. step `1`: `Do` action `ActionId(7)` reads slot `0`, target output slot `1`, deterministic/no-contract mode because generated `CompiledWorkflow` has no action contract table.
  3. step `2`: `Finish` would read slot `1` if reached.
- The action completion payload is otherwise valid: run `1`, step `1`, action `7`, ticket `100`, output slot `1`, output value `I64(42)`, output taint `Clean`.
When:
- Generated runner resumes the action completion.
Then:
- It returns exactly `Err(DriveError::TaintViolation { step: 1 })` or runtime-equivalent typed taint violation carrying step `1`.
- Slot `1` remains `Null`/unwritten exactly as it was before the invalid completion.
- Taint for slot `1` remains its pre-resume value, normally `Clean` for an unwritten slot.
- No `ActionCompleted` event is emitted for ticket `100`.
- No `RunFinished` event is emitted.
- Generated and IR runners agree on the typed error and non-mutation.

### given_pending_action_when_resume_ticket_step_does_not_match_then_invalid_resume_payload_and_no_mutation
Given:
- A suspended action pending ticket `100` for run `1`, step `1`, action `7`, output slot `1`, resume pc `2`.
- Slot `1` is initially `Null` with taint `Clean`.
When:
- A completion payload arrives with ticket `100` but step `2` instead of step `1`, output value `I64(42)`, taint `DerivedFromSecret`.
Then:
- Generated runner returns `Err(DriveError::InvalidResumePayload { reason: "step mismatch" })` or runtime-equivalent typed identity error.
- Slot `1` remains `Null`.
- Slot `1` taint remains `Clean`.
- pc and pending action state remain unchanged.
- No `SlotWritten`, `ActionCompleted`, or `RunFinished` event is appended after the invalid payload.

### given_pending_ask_when_resume_step_does_not_match_then_invalid_resume_payload_and_no_mutation
Given:
- A pending ask ticket `200` for run `1`, suspended step `0`, answer slot `3`, resume pc `1`.
- Slot `3` is initially `Null` with taint `Clean`.
When:
- An answer payload arrives with generated-local resume step `2` instead of resume pc `1`, answer slot `3`, value `Symbol(9001)`, taint `Clean`. A runtime wrapper may also reject a wrong ticket before calling the generated API, but that run/ticket check is outside this generated-runner boundary.
Then:
- Generated runner returns `Err(DriveError::InvalidResumePayload { reason: "resume step mismatch" })` or runtime-equivalent typed identity error.
- Slot `3` remains `Null`.
- Taint for slot `3` remains `Clean`.
- pc and pending ask state remain unchanged.
- No `AskAnswered`, `SlotWritten`, or `RunFinished` event is appended after the invalid payload.

### given_invalid_slot_index_when_generated_runner_reads_or_writes_then_slot_out_of_bounds_error
Given:
- A generated fixture with declared slot capacity `2`.
When:
- A generated step attempts to read or write slot `2`.
Then:
- It returns exactly `Err(DriveError::SlotOutOfBounds { slot: 2 })`.
- No unchecked indexing panic occurs.
- Journal is unchanged by the failed access.

### given_journal_capacity_exactly_full_when_next_required_event_needed_then_capacity_error_before_drop
Given:
- Journal capacity is `2` events.
- Two events are already recorded.
- Next step requires a `SlotWritten` or `ActionCompleted` event.
When:
- Generated runner attempts that step.
Then:
- It returns `Err(DriveError::JournalCapacityExceeded { needed: 1, capacity: 2 })` or runtime-equivalent typed capacity error.
- The existing two journal events remain unchanged and in order.
- The required new event is not silently dropped.
- No frame mutation that depends on the missing journal proof becomes observable.

## Edge Case Tests

### given_step_budget_one_when_two_steps_required_then_step_budget_exhausted_after_first_observable_side_effects_only
Given:
- A workflow with step `0` writing slot `0 = I64(41)` and step `1` finishing slot `0`.
- Step budget is exactly `1`.
When:
- Generated runner drives the workflow.
Then:
- Step `0` may complete and append exactly `SlotWritten { run: 1, step: 0, slot: 0, value: I64(41), taint: Clean }` if IR does the same under budget accounting.
- Before executing step `1`, generated runner returns exactly `Err(DriveError::StepBudgetExhausted)`.
- No `RunFinished` event is emitted.
- IR and generated budget behavior match exactly; if IR decrements budget before step `0`, both must instead produce no slot write. The test must assert the actual IR convention and require generated parity.

### given_duplicate_conflicting_action_completion_when_resume_called_then_completion_already_recorded_or_invalid_resume_and_no_mutation
Given:
- Action ticket `100` has already completed with output `I64(42)` and taint `DerivedFromSecret`.
When:
- A duplicate completion for ticket `100` arrives with output `I64(41)` and taint `DerivedFromSecret`.
Then:
- Generated runner returns `ActionError::CompletionAlreadyRecorded` mapped into the generated typed error, or a runtime-equivalent replay divergence/invalid resume error.
- Slot `1` remains `I64(42)` with taint `DerivedFromSecret`.
- No second `ActionCompleted` event with the conflicting digest is appended.

### given_unsupported_final_ir_primitive_when_emit_requested_then_typed_unsupported_primitive_error
Given:
- A `CompiledWorkflow` containing an unsupported generated-mode primitive such as `TogetherStart`, `ReduceStart`, or `RepeatStart`.
When:
- `emit_rust_workflow` or generated subset validation is called.
Then:
- It returns a typed unsupported primitive/codegen error naming the primitive.
- It does not emit partial generated Rust that could execute with wrong semantics.

## Contract Verification Tests

### given_supported_workflow_when_run_in_ir_and_generated_then_terminal_result_value_taint_and_error_match
Given:
- Any supported generated-subset workflow from the bounded property generator.
When:
- The same initial slots, taints, action outcomes, ask answers, budgets, and journal capacities are supplied to IR and generated runners.
Then:
- Terminal result kind matches exactly.
- If finished, result `SlotValue` and result `Taint` match exactly.
- If error, typed error variant and semantic fields match exactly.
- If suspended, suspension kind and semantic fields match exactly.

### given_action_and_finish_workflow_when_generated_and_ir_run_then_journal_event_order_matches_exactly
Given:
- The action fixture from `given_suspended_action_when_completion_payload_matches_then_slot_written_action_completed_and_run_finished`.
When:
- IR and generated runners execute through completion.
Then:
- Normalized journal event sequence is byte-for-byte equal after excluding non-semantic encoding details.
- Required event order is `SlotWritten(step 0)` then `ActionScheduled(step 1)` then `SlotWritten(step 1)` then `ActionCompleted(step 1)` then `RunFinished(step 2)`.

### given_generated_source_when_static_scanned_then_no_forbidden_hot_path_constructs
Given:
- Emitted generated Rust for all semantic parity fixtures.
When:
- Static scan / compile gate runs.
Then:
- Source contains no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked `slots[` access, unchecked `CONSTANTS[` access, unchecked slicing, unchecked casts, JSON, runtime YAML, HTTP, or runtime string reference resolution.

## Test Exit Criteria
- Every scenario above must become executable or receive an explicit reviewer-approved waiver.
- Tests must assert exact values and errors listed here; broad `is_err` or substring-only checks are insufficient.
- Generated-vs-IR parity must compare terminal result, taint, journal signatures, and typed errors; source-pattern counting alone is not acceptance evidence.
