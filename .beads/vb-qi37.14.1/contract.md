# Contract Specification: `run --step` Single-Step CLI Command

## Context

- **Feature**: `velvet-ballastics run --step <id> --step-input <path> [--output text|json|jsonl]`
- **Bead ID**: vb-qi37.14.1
- **Domain terms**:
  - `StepIdx`: u16 newtype, index into compiled workflow node array
  - `SlotIdx`: u16 newtype, index into frame slot array
  - `SlotValue`: enum (I64, Bool, Null, Object, List, FiniteF64)
  - `Taint`: enum (Clean, DerivedFromSecret, Secret) — secret-propagation marker
  - `StepState`: enum (Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled)
  - `EngineSignal`: enum (Continue, Finished, AwaitingAction, AwaitingWait, AwaitingAsk, StepBudgetExhausted)
  - `EngineError` = `CoreError`: typed error enum with stable diagnostic codes
  - `RunFrame`: hot frame with pc, slots[], taint[], states[]
  - `CompiledWorkflow`: compiled workflow artifact with node array
  - `DurabilityMode`: (Strict, Journaled, None)
  - `OutputFormat`: (Text, Json, Jsonl)
- **Assumptions**:
  - A1: Workflow file passed to `--step` is pre-compiled and accepted (durability gate already passed)
  - A2: Step input is encoded as postcard-serialized `Box<[SlotValue]>`
  - A3: The CLI binary is always invoked as a single-process command; no concurrent step execution within one invocation
  - A4: `step_once()` executes exactly one node transition from the current PC
  - A5: `EngineSignal` variants fully enumerate all possible step outcomes
  - A6: `CoreError` variants fully enumerate all recoverable/unrecoverable step errors
- **Open questions**:
  - Q1: Should `--step` accept a step name (string) in addition to a u16 step ID? Currently only u16.
  - Q2: Should JSON output include the full `SlotValue` serialization, or a summary? Currently undefined.
  - Q3: Should delta reporting include only changed slots (diff), or all slots (full frame snapshot)? Acceptance criteria says "deltas".

## Preconditions

- **PRE-001**: `durability == DurabilityMode::None` — step isolation requires no durability layer. If any other durability mode is supplied, the command must exit with an error and must NOT execute the step.
- **PRE-002**: `step_id` must be a valid index into the compiled workflow's node array (`step_id < node_count`). If out-of-bounds, the command must exit with an error and report the step as not found.
- **PRE-003**: The workflow file must be readable and compile-successfully via `vb_compile::compile_workflow`. Compile errors must be reported in the output format requested.
- **PRE-004**: The step input file must be readable and decodable as postcard-serialized `Box<[SlotValue]>`. An empty file is valid and yields an empty input slot list.
- **PRE-005**: `OutputFormat` argument must be one of {Text, Json, Jsonl}. If absent, defaults to Text.

## Postconditions

- **POST-001**: `step_once()` is called exactly once for the specified step ID, from a freshly constructed `RunFrame`.
- **POST-002**: The command reports the step result in the requested `OutputFormat` (Text/Json/Jsonl).
- **POST-003**: The reported output includes: step index, node kind, program counter after execution (pc_after), and the `EngineSignal` variant.
- **POST-004**: When `OutputFormat` is Json or Jsonl, the output includes `deltas` object with:
  - `slot_deltas`: array of `{slot, before, after}` for each slot, only including slots that changed
  - `taint_deltas`: array of `{slot, before, after}` for each slot whose taint changed
  - `state_deltas`: array of `{step, before, after}` for each step whose state changed
  - `pc_delta`: `{before: StepIdx, after: StepIdx}`
- **POST-005**: When the step produces an output slot, the output slot value and taint are included in the result.
- **POST-006**: When `step_once()` returns `Err(EngineError)`, the error is reported in the requested output format with its diagnostic code and a human-readable message.
- **POST-007**: When `durability != None`, the command exits with `CliExitCode::ValidationFailed` and prints an error message; no step is executed.
- **POST-008**: Exit code is `SUCCESS` (0) when the step executes and returns any `EngineSignal`. Exit code is `RuntimeFailed` (1) when `step_once()` returns an error. Exit code is `ValidationFailed` (2) for preconditions PRE-001 through PRE-004 failures.

## Invariants

- **INV-001**: `RunFrame::new` must succeed for any `(run_id, first_step, step_count, slot_count)` where `step_count > 0` and `first_step < step_count`. The frame is freshly constructed for each `run --step` invocation.
- **INV-002**: After `step_once()` returns, the frame's `states[step_id]` must reflect the correct `StepState` for the executed step, consistent with the returned `EngineSignal`:
  - `Continue` → `Succeeded`
  - `Finished` → `Succeeded`
  - `AwaitingAction` / `StepBudgetExhausted` → `Running`
  - `AwaitingWait` → `Waiting`
  - `AwaitingAsk` → `Asking`
  - any error → `Failed`
- **INV-003**: No slot in the frame is read that was not first written in the same step execution (slots are initialized to `None`; reading `None` before write is a `SlotUninitialized` error surfaced by the engine).
- **INV-004**: The `pc` after `step_once()` is always within `[0, step_count)`.
- **INV-005**: `step_once()` is called exactly once per `run --step` invocation (budget is 1, enforced by the CLI layer calling `step_once` directly without a budget wrapper).
- **INV-006**: Taint on every slot is always one of `{Clean, DerivedFromSecret, Secret}` after any write via `write_slot_with_taint`.

## Error Taxonomy

All errors from `step_once()` are of type `EngineError` (alias for `CoreError`). The CLI maps these to structured output:

- `EngineError::InvalidProgramCounter { step }` → reported as `{"error": "invalid_program_counter", "step": <u16>, "message": "..."}`
- `EngineError::MissingNextStep { step }` → `"missing_next_step"`
- `EngineError::SlotOutOfBounds { slot }` → `"slot_out_of_bounds"`
- `EngineError::SlotUninitialized { slot }` → `"slot_uninitialized"` (precondition or engine bug)
- `EngineError::MissingOutputSlot { step }` → `"missing_output_slot"`
- `EngineError::StepStateOutOfBounds { step }` → `"step_state_out_of_bounds"`
- `EngineError::TypeMismatch { expected, found }` → `"type_mismatch"`
- `EngineError::DivisionByZero` → `"division_by_zero"`
- `EngineError::NonFiniteNumber` → `"non_finite_number"`
- `EngineError::ResourceLimitExceeded { resource }` → `"resource_limit_exceeded"`
- `EngineError::BudgetParse { reason }` → `"budget_parse_error"` (not expected in step mode)
- `EngineError::StepCounterOverflow` → `"step_counter_overflow"` (not expected in single-step)
- `EngineError::UnsupportedPrimitive { primitive }` → `"unsupported_primitive"`
- All other `CoreError` variants → `"internal_error"` with the variant name in the message

Errors before `step_once()` (PRE failures):
- `"durability_not_none"` — for PRE-001
- `"step_not_found"` — for PRE-002
- `"workflow_compile_error"` — for PRE-003
- `"step_input_decode_error"` — for PRE-004

## Contract Signatures

```rust
// CLI layer
fn cmd_run_step(
    workflow: &std::path::Path,
    durability: DurabilityMode,
    target: &StepTarget,
    output: OutputFormat,
) -> ExitCode

// Core engine
pub fn step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError>

// Output formatting
fn print_step_result_jsonl(
    step: StepIdx,
    node: &CompiledNode,
    frame: &RunFrame,
    signal: &EngineSignal,
    before_pc: StepIdx,
) -> String

fn print_step_result_json(
    step: StepIdx,
    node: &CompiledNode,
    frame: &RunFrame,
    signal: &EngineSignal,
    before_pc: StepIdx,
) -> String
```

## Verus-Owned Clauses

- **INV-002**: `step_once()` post-signal state mapping is a pure function from `(EngineSignal, step_id)` to `StepState`. Verified by:
  - Verus proof: `proof_step_state_mapping_invariant(plan, run, signal)` in `vb_core/src/engine/step_verus.rs`
  - Unit test: `step_once_*` test suite in `vb_core/src/engine/step.rs` covers all signal variants
  - Kani harness: bounded model check over all `EngineSignal × StepState` combinations
- **INV-004**: PC bounds after `step_once()` — Rust-level invariant proven by Verus lemma and checked by Kani
- **INV-006**: Taint enum variants are always valid — verified by Verus `is_valid_taint()` lemma

## TLA+-Owned Clauses

- **INV-001 + INV-003 + INV-005**: Single-step execution is atomic and isolated — there is no temporal/loop behavior to model in TLA+ since `run --step` executes exactly one transition with no loop. TLA+ is not applicable for this feature.
  - Rationale: The feature is a single-shot CLI command. There is no state machine, no loop, no protocol, no concurrent actors, no retry logic, no claim/lease, and no liveness property to verify. The execution is a pure function from (workflow, frame, inputs) to (signal, frame'). The temporal model would be a single-state dot, which provides no verification value.

## Theorem-Owned Clauses

None. The `StepState` transition table in `vb_proof_kernels::step_state::is_valid_transition` is a 9×9 boolean matrix that can be exhaustively verified by Kani/proptest without a theorem prover.

## Non-goals

- NG-1: Multi-step execution (handled by `run` without `--step`)
- NG-2: Action resume/completion via tickets (handled by separate `resume` command)
- NG-3: Wait/ask external event handling (suspended by `AwaitingWait`/`AwaitingAsk` signals — caller must handle externally)
- NG-4: Durability beyond `DurabilityMode::None` for step isolation
- NG-5: Step name resolution (only numeric step ID currently)
- NG-6: Concurrent step execution within one CLI invocation
