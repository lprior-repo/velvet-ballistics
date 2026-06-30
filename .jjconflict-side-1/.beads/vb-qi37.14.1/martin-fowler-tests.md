# Martin Fowler Test Plan: `run --step`

## Test Naming Convention

Test names follow the pattern: `test_<given>_<when>_<then>` using expressive, behavior-describing names.

## Happy Path Tests

### test_run_step_executes_one_step_and_returns_correct_signal

**Given**: A valid compiled workflow with two nodes (Nop at step 0, Finish at step 1)
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The command succeeds with exit code 0, reports step index 0, node kind "nop", and signal "Continue"

### test_run_step_with_output_slot_reports_value_and_taint

**Given**: A valid compiled workflow with a SetConst node that writes to slot 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The JSON output includes `output: {value: 42, taint: Clean}` for the output slot

### test_run_step_finish_node_returns_finished_signal

**Given**: A valid compiled workflow with a Finish node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The signal is "Finished" and the JSON output includes the result value and taint

### test_run_step_do_node_returns_awaiting_action

**Given**: A valid compiled workflow with a Do node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The signal is "AwaitingAction" and the step state is "Running"

### test_run_step_wait_node_returns_awaiting_wait

**Given**: A valid compiled workflow with a WaitUntil node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The signal is "AwaitingWait" and the step state is "Waiting"

### test_run_step_ask_node_returns_awaiting_ask

**Given**: A valid compiled workflow with an Ask node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The signal is "AwaitingAsk" and the step state is "Asking"

## Error Path Tests

### test_run_step_with_nondurability_rejects_with_validation_failed

**Given**: A valid compiled workflow
**When**: The user runs `velvet-ballistics run --step 0 --durability strict --step-input <empty> <workflow>`
**Then**: The command exits with code 2 (ValidationFailed) and prints "step isolation requires --durability none"

### test_run_step_with_out_of_bounds_step_id_reports_not_found

**Given**: A valid compiled workflow with 3 nodes (step IDs 0, 1, 2)
**When**: The user runs `velvet-ballistics run --step 99 --step-input <empty> <workflow>`
**Then**: The command exits with code 2 and prints "step 99 not found in workflow"

### test_run_step_with_invalid_workflow_reports_compile_error

**Given**: An invalid workflow YAML file that fails compilation
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <invalid-workflow>`
**Then**: The command exits with code 2 (ValidationFailed) and the output contains "compile error"

### test_run_step_with_malformed_step_input_reports_decode_error

**Given**: A valid compiled workflow
**When**: The user runs `velvet-ballistics run --step 0 --step-input <malformed-bytes> <workflow>`
**Then**: The command exits with code 2 and prints "step-input decode error"

### test_run_step_with_slot_uninitialized_error_reports_correct_error

**Given**: A compiled workflow where step 0 reads slot 5 but slot 5 was never written
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The JSON error output contains `{"error": "slot_uninitialized", "slot": 5}`

### test_run_step_engine_error_in_json_format_reports_error_code_and_message

**Given**: A compiled workflow that triggers an engine error at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> --output json <workflow>`
**Then**: The JSON output is a single valid JSON object containing `error`, `code`, and `message` fields

### test_run_step_engine_error_in_jsonl_format_reports_error_object

**Given**: A compiled workflow that triggers an engine error at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> --output jsonl <workflow>`
**Then**: The output is a single valid JSON line with `error`, `code`, and `message` fields

## Edge Case Tests

### test_run_step_with_empty_step_input_succeeds

**Given**: A valid compiled workflow with a SetConst node that writes to slot 0 without reading input slots
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty-file> <workflow>`
**Then**: The command succeeds — an empty step input is valid and decodes to `Box<[SlotValue]>::from([])`

### test_run_step_zero_slot_count_workflow

**Given**: A valid compiled workflow with `slot_count: 0` and a Nop node
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The command succeeds with signal Continue; slot_deltas is empty

### test_run_step_with_eval_expr_node

**Given**: A valid compiled workflow with an EvalExpr node at step 0 that evaluates `42 + 1`
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The output slot value is `43` and signal is `Continue`

### test_run_step_with_build_object_node

**Given**: A valid compiled workflow with a BuildObject node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The output slot contains an Object handle and signal is `Continue`

### test_run_step_with_build_list_node

**Given**: A valid compiled workflow with a BuildList node at step 0
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The output slot contains a List handle and signal is `Continue`

### test_run_step_with_jump_node

**Given**: A valid compiled workflow with a Jump node at step 0 targeting step 2
**When**: The user runs `velvet-ballistics run --step 0 --step-input <empty> <workflow>`
**Then**: The pc_delta.after is 2 and signal is `Continue`

## Contract Verification Tests

### test_precondition_durability_gate_enforced_before_step_execution

**Given**: A valid compiled workflow
**When**: `cmd_run_step` is called with `durability == DurabilityMode::Strict`
**Then**: `step_once` is never called; error message is printed; exit code is ValidationFailed

### test_precondition_step_bounds_validated_before_step_execution

**Given**: A valid compiled workflow with N nodes
**When**: `cmd_run_step` is called with `step_id >= N`
**Then**: `step_once` is never called; "step not found" is printed; exit code is ValidationFailed

### test_postcondition_exactly_one_step_once_called

**Given**: A valid compiled workflow
**When**: `execute_step_isolated` is called
**Then**: `step_once` is called exactly once; the returned signal is passed to `print_step_result`

### test_postcondition_delta_contains_pc_before_and_after

**Given**: A valid compiled workflow with a Nop node at step 0
**When**: `run --step 0` is executed
**Then**: The JSON output contains `deltas.pc_delta.before` and `deltas.pc_delta.after` with correct values

### test_postcondition_delta_contains_slot_changes_only

**Given**: A valid compiled workflow with SetConst writing to slot 0
**When**: `run --step 0` is executed
**Then**: `deltas.slot_deltas` contains exactly one entry for slot 0; slots that did not change are absent

### test_postcondition_delta_contains_taint_changes_only

**Given**: A valid compiled workflow that produces a slot with taint DerivedFromSecret
**When**: `run --step 0` is executed
**Then**: `deltas.taint_deltas` contains entries only for slots whose taint changed from Clean

### test_postcondition_delta_contains_state_changes_only

**Given**: A valid compiled workflow with a Nop node at step 0
**When**: `run --step 0` is executed
**Then**: `deltas.state_deltas` contains one entry for step 0: `before: Pending, after: Succeeded`

### test_postcondition_error_reported_in_requested_format

**Given**: A compiled workflow that triggers an engine error
**When**: `run --step 0 --output json` is executed
**Then**: stdout contains valid JSON with error fields; stderr is empty

### test_invariant_step_state_matches_signal_after_step

**Given**: A valid compiled workflow
**When**: `run --step N` is executed for each node kind (Nop, EvalExpr, BuildObject, BuildList, Jump, Do, WaitUntil, Ask, Finish)
**Then**: For each node kind, `frame.states[N]` matches the expected `StepState` for the returned `EngineSignal`

### test_invariant_pc_in_bounds_after_step

**Given**: A valid compiled workflow
**When**: `run --step N` is executed for any valid step N
**Then**: `0 <= pc_after < step_count`

### test_invariant_taint_always_valid_after_write

**Given**: A valid compiled workflow with SetConst at step 0
**When**: `run --step 0` is executed
**Then**: All entries in `frame.taint` are one of {Clean, DerivedFromSecret, Secret}

## Given-When-Then Scenarios

### Scenario 1: Single-step execution with JSON output and delta reporting

**Given**: A valid compiled workflow stored at `/workflow.vbc` with a SetConst node at step 0
**And**: `--step-input` points to an empty file
**And**: `--output json` is specified
**When**: The user runs `velvet-ballistics run --step 0 --step-input /dev/null --output json /workflow.vbc`
**Then**:
- The command exits with code 0
- stdout is a single valid JSON object
- The JSON object contains: `step: 0`, `kind: "set_const"`, `signal: "Continue"`
- The JSON object contains `deltas.pc_delta.before: 0` and `deltas.pc_delta.after: 1`
- The JSON object contains `deltas.slot_deltas` with one entry for the written slot
- The JSON object contains `deltas.taint_deltas` if the taint changed from Clean
- The JSON object contains `deltas.state_deltas` with one entry for step 0: `before: "Pending", after: "Succeeded"`
- stderr is empty

### Scenario 2: Step not found error in JSON Lines format

**Given**: A valid compiled workflow stored at `/workflow.vbc` with 3 nodes (step IDs 0, 1, 2)
**When**: The user runs `velvet-ballistics run --step 99 --step-input /dev/null --output jsonl /workflow.vbc`
**Then**:
- The command exits with code 2
- stdout is a single valid JSON line
- The JSON object contains `error: "step_not_found"`, `step: 99`, `message: "step 99 not found in workflow"`
- stderr is empty

### Scenario 3: Durability gate rejection

**Given**: A valid compiled workflow stored at `/workflow.vbc`
**When**: The user runs `velvet-ballistics run --step 0 --durability journaled --step-input /dev/null /workflow.vbc`
**Then**:
- The command exits with code 2
- stderr contains "step isolation requires --durability none"
- `step_once` is never called
- No output is produced to stdout

### Scenario 4: Engine error in JSON format

**Given**: A compiled workflow at `/workflow.vbc` where step 0 reads slot 99 (out of bounds)
**When**: The user runs `velvet-ballistics run --step 0 --step-input /dev/null --output json /workflow.vbc`
**Then**:
- The command exits with code 1 (RuntimeFailed)
- stdout is a single valid JSON object
- The JSON object contains `error: "slot_out_of_bounds"`, `slot: 99`, and a message field
- stderr is empty
