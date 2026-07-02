# Assurance Bundle - vb-qi37.14.1

## Bead
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Date**: 2026-05-18

---

## Acceptance Criteria Evidence

### AC-1: `run --step` executes exactly one step

**Contract Clause**: POST-001, INV-005

**Evidence**:
- **Integration Test** (`crates/vb_cli/tests/vb_qi37_14_1_run_step.rs`):
  - `run_step_executes_single_step_and_reports_correct_index` (line 607): Verifies step index 0 executes and reports correctly
- **Unit Test** (`crates/vb_cli/src/main_tests.rs`):
  - `execute_step_isolated_set_const_step_succeeds` (line 765): Directly calls `execute_step_isolated` and verifies SUCCESS exit code
- **Black-Hat Review** (`.beads/vb-qi37.14.1/black-hat-review.md` line 67): "Single-Step Execution (INV-005 / POST-001): `execute_step_isolated` calls `vb_core::step_once()` exactly once."
- **Implementation** (`crates/vb_cli/src/app_impl.rs:1607`): `vb_core::step_once(compiled, &mut frame, &mut store)` is called once inside `execute_step_isolated`; no loop, no budget wrapper.

**Verdict**: PASS - Evidence directly maps to AC-1.

---

### AC-2: Reports pc/slot/taint/state deltas

**Contract Clause**: POST-004

**Evidence**:
- **Integration Test** (`vb_qi37_14_1_run_step.rs`):
  - `run_step_delta_json_pc_delta_has_before_and_after` (line 867): Verifies `pc_delta` with `before` and `after` fields
  - `run_step_delta_json_slot_deltas_is_array_with_changes` (line 928): Verifies `slot_deltas` array with `slot`, `before`, `after`
  - `run_step_delta_json_state_deltas_has_before_after` (line 1003): Verifies `state_deltas` array with `step`, `before`, `after`
  - `run_step_delta_json_taint_deltas_is_array` (line 1078): Verifies `taint_deltas` is an array
  - `run_step_json_output_has_required_schema_fields` (line 659): Verifies all four delta types present in `deltas` object
- **Implementation** (`app_impl.rs:1600-1639`): Snapshots captured before/after `step_once`, deltas computed via `compute_slot_deltas`, `compute_taint_deltas`, `compute_state_deltas`
- **Black-Hat Review** (line 72): "Delta Reporting (POST-004): All four delta types present: pc_delta, slot_deltas, taint_deltas, state_deltas."

**Verdict**: PASS - All four delta types verified by tests and confirmed by black-hat review.

---

### AC-3: Respects durability gates

**Contract Clause**: PRE-001, POST-007

**Evidence**:
- **Integration Test** (`vb_qi37_14_1_run_step.rs`):
  - `run_step_rejects_durability_strict` (line 123): Verifies exit code 2 when `--durability strict`
  - `run_step_rejects_durability_journaled` (line 176): Verifies exit code 2 when `--durability journaled`
  - `run_step_durability_not_none_exits_with_validation_failed` (line 1311): Explicit POST-007 test
- **Implementation** (`app_impl.rs:1477-1491`): Early return with `CliExitCode::ValidationFailed` if `durability != DurabilityMode::None`
- **Black-Hat Review** (line 69): "Durability Gate (PRE-001): Correctly enforces `DurabilityMode::None` with exit code 2."
- **Exit Code** (`exit_code.rs:18`): `ValidationFailed = 2`

**Verdict**: PASS - Durability gate enforced with correct exit code.

---

### AC-4: Has tests for valid and invalid step requests

**Contract Clause**: PRE-002, PRE-003, PRE-004, PRE-005, POST-002, POST-003, POST-006, POST-008

**Valid Request Tests**:
- `run_step_executes_single_step_and_reports_correct_index` (line 607): Valid step ID 0
- `run_step_json_flag_produces_valid_json` (line 442): Valid JSON output
- `run_step_jsonl_flag_produces_valid_jsonl` (line 495): Valid JSONL output
- `run_step_text_output_is_human_readable` (line 552): Valid text output
- `run_step_success_exits_with_code_0` (line 1361): Exit code 0 for success
- `run_step_empty_step_input_succeeds` (line 1513): Empty input is valid

**Invalid Request Tests**:
- `run_step_invalid_step_id_reports_not_found` (line 227): Out-of-bounds step ID → non-success
- `run_step_invalid_step_id_json_includes_error_details` (line 280): JSON error for invalid step
- `run_step_compile_error_reports_failure` (line 343): Invalid YAML → compile error
- `run_step_compile_error_json_includes_errors` (line 388): JSON error for compile failure
- `run_step_malformed_step_input_exits_with_code_2` (line 1463): Invalid postcard data → exit code 2
- `run_step_validation_failure_exits_with_code_2` (line 1412): Validation failures → exit code 2

**Verdict**: PASS - Comprehensive valid/invalid coverage across all precondition and postcondition failure modes.

---

## Formal Verification Evidence

### Verus (55 lemmas across 3 files)

| File | Clauses Verified |
|------|-----------------|
| `verification/verus/step_state_machine.rs` | INV-002 (step state mapping), INV-006 (taint validity) |
| `verification/verus/run_frame_invariant.rs` | INV-001 (frame construction), INV-004 (PC bounds) |
| `verification/verus/signals_invariant.rs` | Signal enum exhaustiveness |

**Machine Gate**: "55 lemmas verified across 3 Verus files. No errors."

### Kani (BLOCKED_TOOLING waived)

- 6 Kani harnesses timeout due to `SlotValue` symbolic complexity
- 4 Verus proofs compensate for same invariants (INV-002, INV-004, INV-006)
- Waived per machine-gate-report: "BLOCKED_TOOLING"

### TLA+ (Non-applicable)

Per contract.md line 141-142: Single-shot CLI command has no state machine, loop, protocol, or liveness property. TLA+ spec would be a single-state dot.

---

## Black-Hat Review Evidence

**Status**: PASS (`.beads/vb-qi37.14.1/black-hat-review.md`)

| Defect | Fix Verified |
|--------|-------------|
| DEFECT-001: Exit code 3 → 2 for PRE-003 | FIXED: `app_impl.rs:1553` now returns `ValidationFailed` |
| DEFECT-002: JSON to stdout instead of stderr | FIXED: `app_impl.rs:5179-5187` now writes to stderr |

**Edge Cases Verified**: Empty input, OOB step ID, invalid postcard, slots/taint length mismatch, empty delta arrays.

---

## Machine Gate Evidence

| Gate | Result |
|------|--------|
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace` | PASS (No issues found) |
| `cargo test --workspace` | 10,962 passed |
| `cargo test --package vb_cli --test vb_qi37_14_1_run_step` | 25 passed |

**Classification**: BLOCK_LOCAL: None, BLOCK_REGRESSION: None, STATUS: PASS

---

## Gap Analysis

| Gap | Severity | Mitigation |
|-----|----------|------------|
| POST-005: `output_slot` structure not strictly validated in test | Low | Implementation at `app_impl.rs:1802-1817` correctly populates `output_slot` with `value` and `taint`. Test uses loose check (`has_output`). Black-hat review did not flag. |
| Kani BLOCKED_TOOLING | Medium | 4 Verus proofs cover same invariants; waived per machine gate. |

No HIGH or CRITICAL gaps. All acceptance criteria have corresponding test coverage.

---

## Summary

| Acceptance Criterion | Tests | Formal | Review | Gate |
|---------------------|-------|--------|--------|------|
| AC-1: Single-step execution | 2 | INV-005-CLI | CONFIRMED | PASS |
| AC-2: Delta reporting | 5 | - | CONFIRMED | PASS |
| AC-3: Durability gates | 3 | PRE-001-CLI | CONFIRMED | PASS |
| AC-4: Valid/invalid tests | 11 | - | CONFIRMED | PASS |

**Overall**: All acceptance criteria are satisfied with verifiable evidence.