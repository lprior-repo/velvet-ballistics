# Implementation: vb-qi37.14.1 - `run --step` Single-Step CLI Command

## Bead Details
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Status**: Implemented

## Summary

Implemented structured JSON/JSONL output for the `run --step` command with pc/slot/taint/state deltas reporting.

## Changes Made

### 1. Modified `cmd_run_step` (app_impl.rs)
- Added `output: OutputFormat` parameter to support JSON/JSONL output formats
- Added structured error output for PRE-001 (durability not none), PRE-002 (step not found), and PRE-004 (step input decode) failures
- PRE-001 and PRE-002 failures now return exit code 2 (`CliExitCode::VerificationFailed`) per contract
- PRE-004 failures also return exit code 2 per contract

### 2. Modified `execute_step_isolated` (app_impl.rs)
- Added `output: OutputFormat` parameter
- Captures before state (pc, slots, taint, states) before calling `step_once`
- Captures after state after `step_once` returns
- Computes deltas: pc_delta, slot_deltas, taint_deltas, state_deltas
- Passes captured state to `print_step_result` for structured output

### 3. Added `StepStateSnapshots` struct (app_impl.rs)
- New struct to hold before/after state snapshots
- Provides `to_before_json()` and `to_after_json()` methods for JSON serialization

### 4. Added Delta Computation Functions (app_impl.rs)
- `compute_slot_deltas(before, after)` - computes slot value changes
- `compute_taint_deltas(before, after)` - computes taint marker changes
- `compute_state_deltas(before, after)` - computes step state changes
- All functions use `.get()` for safe slice access (no panics)

### 5. Added `build_step_result_json` (app_impl.rs)
- Builds the full JSON result object per contract specification
- Includes: step, kind, signal, before, after, deltas, and output_slot (if present)

### 6. Added `write_contract_error_json` (app_impl.rs)
- Outputs contract-format error JSON directly to stderr
- Used for PRE-001 through PRE-004 failures
- Format: `{"error": "...", "message": "...", ...context}`

### 7. Updated `print_step_result` (app_impl.rs)
- Now accepts `output: OutputFormat` and `snapshots: StepStateSnapshots`
- Dispatches to JSON/JSONL or text output based on format

### 8. Added `StepState::Serialize` (frame.rs)
- Added `#[derive(serde::Serialize)]` to `StepState` enum to support JSON serialization

### 9. Added RunFrame Snapshot Methods (frame.rs)
- `slots_snapshot()` - returns `Vec<Option<SlotValue>>` copy of all slots
- `taint_snapshot()` - returns `Vec<Taint>` copy of all taint markers
- `states_snapshot()` - returns `Vec<StepState>` copy of all step states

### 10. Updated Unit Tests (main_tests.rs)
- `execute_step_isolated_set_const_step_succeeds` - added `OutputFormat::Text` parameter
- `decode_step_inputs_empty_data_returns_empty` - added `OutputFormat::Text` parameter
- `decode_step_inputs_invalid_data_returns_error` - updated to expect `CliExitCode::VerificationFailed`

## JSON Output Schema

When using `--output json` or `--output jsonl`, the output includes:

```json
{
  "step": <u16>,
  "kind": "<node_kind>",
  "signal": "<signal_name>",
  "before": {
    "pc": <StepIdx>,
    "slots": [...],
    "taint": [...],
    "states": [...]
  },
  "after": {
    "pc": <StepIdx>,
    "slots": [...],
    "taint": [...],
    "states": [...]
  },
  "deltas": {
    "pc_delta": {"before": <StepIdx>, "after": <StepIdx>},
    "slot_deltas": [{"slot": <u16>, "before": <Option<SlotValue>>, "after": <Option<SlotValue>>}, ...],
    "taint_deltas": [{"slot": <u16>, "before": <Taint>, "after": <Taint>}, ...],
    "state_deltas": [{"step": <u16>, "before": <StepState>, "after": <StepState>}, ...]
  },
  "output_slot": {
    "slot": <SlotIdx>,
    "value": <SlotValue>,
    "taint": <Taint>
  }
}
```

## Error Output Schema

Contract-format errors for PRE failures:

```json
{"error": "durability_not_none", "message": "step isolation requires --durability none"}
{"error": "step_not_found", "step": <u16>, "message": "step N not found in workflow"}
{"error": "step_input_decode_error", "message": "step-input decode error: ..."}
```

## Exit Codes

- `0` (SUCCESS): Step executed and returned an EngineSignal
- `1` (RuntimeFailed): step_once() returned an error
- `2` (VerificationFailed): PRE-001 through PRE-004 precondition failures
- `3` (CompileFailed): Workflow compilation failed
- `4` (RuntimeFailed): Runtime execution failed

## Files Modified

1. `crates/vb_cli/src/app_impl.rs` - Main implementation changes
2. `crates/vb_cli/src/main_tests.rs` - Updated unit tests
3. `crates/vb_core/src/frame.rs` - Added Serialize to StepState, added snapshot methods

## Verification

- `cargo check --workspace` - passes
- `cargo clippy --package vb_cli` - No issues found
- `cargo clippy --package vb_core` - No issues found
- `cargo test --package vb_cli` - 581 passed, 1 ignored

## Constraints Followed

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
- Used `.get()` instead of direct indexing to avoid panics
- Used typed errors and Result-based error handling throughout
- Used `serde_json::Map` instead of indexing for building JSON objects
