# Verification Layers: `run --step` (vb-qi37.14.1)

## Boundary

- **Verus-owned kernel**: `mark_step_after_signal`, `step_once`, `RunFrame::write_slot_with_taint`, `RunFrame::new`, `set_pc`, taint lattice (`join_taint`, `is_valid_step_state_transition`)
- **TLA+ temporal model**: None — single-shot pure function. See `tla-spec.md`.
- **Theorem projection**: None — step-state table is 9×9 boolean matrix verified by Kani + unit tests. See `lean-contract.md`.
- **Runtime shell**: CLI argument parsing (`args.rs`), file I/O, output formatting (Text/Json/Jsonl), exit code mapping, `cmd_run_step`, `execute_step_isolated`, `print_step_result`, `print_step_result_json`, `print_step_result_jsonl`
- **External systems**: None — fully self-contained CLI command

## Layer Assignment

### Contract Clause → Verification Layer Matrix

| Clause | Layer 1 | Layer 2 | Layer 3 | Layer 4 |
|--------|---------|---------|---------|---------|
| PRE-001 (durability gate) | unit test | integration test | — | — |
| PRE-002 (step bounds) | unit test | Kani | integration test | — |
| PRE-003 (compile workflow) | unit test | integration test | — | — |
| PRE-004 (decode step input) | unit test | integration test | — | — |
| PRE-005 (output format) | unit test | integration test | — | — |
| POST-001 (exactly one step_once) | integration test | unit test | — | — |
| POST-002 (output format respected) | integration test (JSON/JSONL) | unit test | — | — |
| POST-003 (pc/signal reported) | integration test | unit test | — | — |
| POST-004 (delta reporting) | integration test (JSON/JSONL) | unit test | — | — |
| POST-005 (output slot value/taint) | integration test | unit test | — | — |
| POST-006 (error in output format) | integration test (JSON/JSONL) | unit test | — | — |
| POST-007 (durability error exit) | unit test | integration test | — | — |
| POST-008 (exit codes) | integration test | unit test | — | — |
| INV-001 (RunFrame::new bounds) | Verus | Kani | unit test | — |
| INV-002 (step-state mapping) | Verus | Kani | unit test | — |
| INV-003 (slot initialized) | Kani | unit test | — | — |
| INV-004 (PC bounds) | Verus | Kani | unit test | — |
| INV-005 (exactly one step_once) | integration test | code review | — | — |
| INV-006 (taint validity) | Verus | Kani | unit test | — |
| ERR-001 (all EngineError variants) | unit test | Kani | integration test | — |

## Verus Scope

### Target: `mark_step_after_signal` (INV-002)

- **Module**: `crates/vb_core/src/engine/step.rs`
- **Spec/Proof function**: `spec_mark_step_after_signal`, `proof_inv_step_state_mapping`
- **Invariant**: The function body is a total match on `EngineSignal` that writes exactly one `StepState` variant. The Verus proof verifies exhaustiveness — that all `EngineSignal` variants are handled and each maps to the correct `StepState`.
- **Trusted boundary**: `EngineSignal` and `StepState` are closed enums; `run.mark_*` methods perform their own bounds validation.
- **Shell exclusions**: No I/O, no async, no storage, no wall-clock time, no FFI.

### Target: `step_once` (INV-004 + INV-002)

- **Module**: `crates/vb_core/src/engine/step.rs`
- **Spec function**: `spec_step_once_pc_result(plan, run, store) -> (EngineSignal, StepIdx)`
- **Invariant**: `spec_post_pc_in_bounds(result, run.step_count)` — the returned PC is always `< step_count`.
- **Trusted boundary**: `CompiledWorkflow::node(pc)` returns `None` if and only if `pc >= node_count`; `set_pc` validates before writing.
- **Shell exclusions**: No I/O, no async, no storage.

### Target: `RunFrame::write_slot_with_taint` (INV-006)

- **Module**: `crates/vb_core/src/frame.rs`
- **Lemma**: `lemma_taint_valid_write(slot, taint)` — if the method returns `Ok(())`, then `taint ∈ {Clean, DerivedFromSecret, Secret}`.
- **Trusted boundary**: `Taint` enum has exactly three variants; no raw u8-to-Taint conversion.
- **Shell exclusions**: No I/O, no async, no storage.

## Kani Scope

### Harness 1: `step_once` panic freedom + INV-002 + INV-004

- **Target**: `crates/vb_core/src/engine/step.rs::step_once`
- **Claim**: `step_once` never panics for any valid `CompiledWorkflow`, `RunFrame`, `ValueStore` inputs within the bounded model.
- **Arbitrary inputs**: Implement `kani::Arbitrary` for `CompiledWorkflow` (construct from arbitrary `WorkflowParts`), `RunFrame` (construct with `RunFrame::new` or `reinitialize`), `ValueStore` (empty store is sufficient for single-step).
- **Checks**: `kani::cover` for each `EngineSignal` variant; assert PC in bounds after return; assert `states[step]` matches signal mapping.
- **Bounded model**: `step_count ∈ [1, 16]`, `slot_count ∈ [0, 32]` — sufficient to exercise all `CompiledNodeKind` dispatch paths.

### Harness 2: `step_state_transition` exhaustiveness

- **Target**: `vb_proof_kernels::step_state::is_valid_transition`
- **Claim**: The 9×9 boolean matrix returns `true` for all and only valid `StepState → StepState` transitions.
- **Checks**: For each of 81 `(current, new)` pairs, assert the matrix entry matches expected validity. Equivalent to exhaustive unit test but with formal bounded guarantee.

## Unit Test Scope (Rust)

### Engine unit tests (`step.rs`)

- `step_once_nop_advances_pc_and_returns_continue` — Nop dispatch
- `step_once_finish_returns_finished_with_value_and_taint` — Finish dispatch
- `step_once_do_returns_awaiting_action` — Do node dispatch
- `step_once_wait_returns_awaiting_wait` — WaitUntil dispatch
- `step_once_ask_returns_awaiting_ask` — Ask dispatch
- `step_once_jump_advances_pc_to_target` — Jump dispatch
- `step_once_eval_expr_writes_result_to_output_slot` — EvalExpr dispatch
- `step_once_build_object_writes_object_to_output_slot` — BuildObject dispatch
- `step_once_build_list_writes_list_to_output_slot` — BuildList dispatch
- `step_once_error_handler_jumps_to_body` — ErrorHandler dispatch
- `resume_action_completion_writes_output_and_advances_pc` — resume path
- `resume_action_failure_marks_step_failed` — resume failure path
- `journal_action_suspended_captures_all_fields` — journal event

### Frame unit tests (`frame.rs`)

- `step_state_valid_transitions` — exhaustive transition validity
- `write_slot_with_taint_preserves_taint_invariant`
- `new_rejects_zero_step_count`
- `new_rejects_out_of_bounds_first_step`

## Integration Test Scope (`cli_integration.rs`)

- `run_step_executes_one_step_and_reports_result` — happy path
- `run_step_with_valid_step_id_returns_correct_signal` — PRE-002
- `run_step_with_invalid_step_id_reports_not_found` — PRE-002
- `run_step_with_nondurability_rejects` — PRE-001 / POST-007
- `run_step_output_json_contains_delta_fields` — POST-002, POST-003, POST-004
- `run_step_output_jsonl_contains_delta_fields` — POST-002, POST-003, POST-004
- `run_step_output_json_error_contains_error_fields` — POST-006
- `run_step_output_jsonl_error_contains_error_fields` — POST-006
- `run_step_with_valid_step_input_deserializes` — PRE-004
- `run_step_with_invalid_step_input_reports_decode_error` — PRE-004
- `run_step_exit_code_success_on_valid_execution` — POST-008
- `run_step_exit_code_validation_failed_on_precondition_failure` — POST-008
- `run_step_exit_code_runtime_failed_on_engine_error` — POST-008

## Static Analysis (Clippy)

- `cargo clippy --workspace --lib --bins -- -D warnings` on `vb_cli` and `vb_core`
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented` in production paths
- No `unsafe` in `vb_core/src/engine/step.rs` (enforced by `#![forbid(unsafe_code)]`)

## Waiver: TLA+

See `tla-spec.md` for the formal non-applicability rationale. No temporal model needed.

## Waiver: Lean/Aeneas/Hax

See `lean-contract.md`. Step-state table is 9×9 boolean, verified by Kani and unit tests.
