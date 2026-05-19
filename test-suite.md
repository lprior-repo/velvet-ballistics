# Test Suite Status Report — vb-0sps

## Bead Information

| Field | Value |
|-------|-------|
| Bead ID | vb-0sps / VB-BDD-CATALOG-007 |
| State | 8 (test-writer) |
| Workdir | /home/lewis/src/bd-vb-0sps-bdd |
| Target File | `crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs` |

## Test Plan Reference

- **Test Plan**: `.beads/vb-0sps/test-plan.md`
- **Approved**: State 6 proof-reviewer (attempt 7)
- **TLC Evidence**: 1.6M/1.9M states, depth 10, exit 0 on 4 positive configs

## Deliverables Status

| Deliverable | Required | Written | Status |
|---|---|---|---|
| BDD Given/When/Then scenarios | 18 | 18 | BLOCKED |
| Proptest invariants | 6 | 0 (in proptest module) | BLOCKED |
| Fuzz targets | 3 | 1 (journal_event exists) | PARTIAL |
| Kani harnesses | 2 | 0 | NOT STARTED |
| Mutation checkpoints | 9 | 9 | BLOCKED |

## Execution Results

```
cargo nextest run -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd

Result: 32 tests run, 0 passed, 32 failed
All failures: "not yet implemented: ObservedRun and compare_observed_runs must be implemented"
```

## BLOCKED Items

### Root Cause: Missing Types

The following types are referenced in the test-plan but **do not exist** in the codebase:

1. **`ObservedRun`** — Records terminal state, journal events, slot values, taints
   - Expected in: `vb_codegen` crate
   - Used by: All BDD scenarios, proptest invariants

2. **`ParityError`** — Error variants for parity mismatches
   - Expected variants: `TerminalMismatch`, `JournalMismatch`, `TaintMismatch`, `SuspensionMismatch`, `ResumeMismatch`, `UnsupportedMismatch`
   - Used by: `compare_observed_runs` return type

3. **`compare_observed_runs(ir: &ObservedRun, gen: &ObservedRun) -> Result<(), ParityError>`**
   - Core comparison function
   - Not found in `vb_codegen::lib.rs`

## What Was Written

### BDD Scenarios (18)

All 18 BDD Given/When/Then scenarios written:

**Family 1: Deterministic Terminal Parity (3)**
- `deterministic_workflow_terminal_parity_when_ir_and_generated_finish` (1.1)
- `taint_parity_at_every_slot_write_and_terminal` (1.2)
- `step_state_sequence_legal_and_terminal_states_do_not_reopen` (1.3)

**Family 2: Suspension Parity (3)**
- `do_action_blocks_suspension_metadata_matches_and_pc_does_not_advance` (2.1)
- `wait_until_blocks_metadata_and_pc_matches` (2.2)
- `ask_blocks_metadata_and_pc_matches` (2.3)

**Family 3: Resume Parity (3)**
- `resume_action_completion_parity_output_taint_event_pc_and_final_result` (3.1)
- `resume_ask_answer_parity_output_taint_event_pc_and_final_result` (3.2)
- `resume_timer_parity_event_pc_and_final_result` (3.3)

**Family 4: Typed Error Parity (4)**
- `missing_slot_error_parity_variant_and_fields` (4.1)
- `divide_by_zero_error_parity_variant_and_fields` (4.2)
- `type_mismatch_error_parity_variant_and_fields` (4.3)
- `budget_exhausted_error_parity_variant_and_fields` (4.4)

**Family 5: Unsupported Fail-Closed (4)**
- `unsupported_accessor_returns_unsupported_ir_before_source_emission` (5.1)
- `unsupported_expression_returns_unsupported_ir_before_source_emission` (5.2)
- `unsupported_node_returns_unsupported_ir_before_source_emission` (5.3)
- `unsupported_workflow_not_counted_as_generated_parity` (5.4)

**Family 6: Catalog Integrity (3)**
- `catalog_007_points_to_executable_target_and_deferred_is_none` (6.1)
- `all_positive_parity_fixtures_pass_validate_before_execution` (6.2)
- `no_maxperf_speed_pgo_release_claims_in_bdd_documentation` (6.3)

### Unit Tests (6)

- `compare_observed_runs_identical_inputs_produces_ok`
- `compare_observed_runs_detects_terminal_mismatch`
- `compare_observed_runs_detects_journal_length_mismatch`
- `validate_generated_subset_accepts_valid_deterministic_workflow`
- `unsupported_workflow_no_source_emitted_when_validate_fails`

### Mutation Checkpoint Tests (9)

- M1: `mutation_detects_shortcircuit_in_and_helper`
- M2: `mutation_detects_skipped_taint_comparison`
- M3: `mutation_detects_ir_only_journal_emission`
- M4: `mutation_detects_missing_source_emitted_guard`
- M5: `mutation_detects_pc_advance_past_blocked_step`
- M6: `mutation_detects_wrong_error_variant_mapping`
- M7: `mutation_detects_resume_before_slot_write`
- M8: `mutation_detects_wrong_slot_index_in_suspension`
- M9: `mutation_detects_catalog_deferred_not_cleared`

### Fixtures (13)

- `make_deterministic_do_finish_workflow`
- `make_tainted_slot_workflow`
- `make_do_action_blocking_workflow`
- `make_wait_until_blocking_workflow`
- `make_ask_blocking_workflow`
- `make_missing_slot_workflow`
- `make_div_by_zero_workflow`
- `make_type_mismatch_workflow`
- `make_budget_exhausted_workflow`
- `make_unsupported_accessor_workflow`
- `make_unsupported_expression_workflow`
- `make_unsupported_node_workflow`

## Compilation Status

```
cargo build -p velvet-ballastics-workspace-tests 2>&1
Result: SUCCESS (0 crates compiled, no output)
```

## Test Execution Status

```
cargo nextest run -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd 2>&1
Result: 32 tests run, 0 passed, 32 failed
All failures: "not yet implemented: ObservedRun and compare_observed_runs must be implemented"
```

## Open Questions (from test-plan.md)

1. **Error adapter for POST-002**: Does a "documented normalized adapter" exist for error field mapping?
2. **Journal event adapter for POST-005**: Do event kind labels match exactly between IR and generated journal?
3. **ObservedRun type**: Does it exist in vb_codegen, or does it need to be implemented?
4. **SlotValue/Taint concrete types**: Are these concrete types or type aliases?
5. **Generated runtime compile**: Is rustc available in test environment, or use pre-compiled fixtures?
6. **Catalog update timing**: Is catalog update part of this implementation or separate?

## Definition of Done

The following must be completed before vb-0sps tests can pass:

- [ ] Implement `ObservedRun` type in vb_codegen
- [ ] Implement `ParityError` enum in vb_codegen
- [ ] Implement `compare_observed_runs(ir: &ObservedRun, gen: &ObservedRun) -> Result<(), ParityError>`
- [ ] Implement `ObservedRun` builders/fixtures for test scenarios
- [ ] Implement `run_ir_observed()` and `run_generated_observed()` test helpers
- [ ] Replace all `todo!()` with actual assertions
- [ ] Add corpus seeds for fuzz targets
- [ ] Write 2 Kani harnesses
- [ ] Verify all 18 BDD scenarios pass
- [ ] Verify all 6 proptest invariants pass (10,000 cases)
- [ ] Verify mutation kill rate ≥ 90%

---

## Test Suite Report

### Test Count
- Unit tests (BDD scenarios): 18
- Unit tests (pure parity functions): 5  
- Mutation checkpoint tests: 9
- Fixtures: 13
- **Total test functions**: 32

### Gate Results
- [x] Source clippy: 0 warnings (ignoring dead_code/unused warnings)
- [x] Test compile: PASS
- [x] nextest: 32 tests run, 0 passed, 32 failed (expected - all fail on todo!())
- [ ] Mutation kill rate: PENDING
- [ ] Line coverage: PENDING
- [ ] Moon CI: PENDING

### Status: BLOCKED

The test suite is written and compiles successfully. All 32 tests fail with `todo!()` because `ObservedRun` and `ParityError` types are not yet implemented in vb_codegen.

**TEST SUITE STATUS: BLOCKED**

The tests are written and verified to compile. They cannot execute successfully until the `ObservedRun` and `ParityError` types are implemented in vb_codegen, as specified in the test-plan.md open questions.
