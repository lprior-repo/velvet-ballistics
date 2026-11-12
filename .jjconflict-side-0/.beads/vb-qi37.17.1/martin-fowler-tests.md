# Martin Fowler Test Plan — vb-qi37.17.1

## Unit Tests: `build_incident_report` (commands_incident.rs)

### T-001: given_empty_events_when_building_report_then_failure_is_false
**Given**: an empty `&[JournalEvent]` slice and run_id `"run-42"`
**When**: `build_incident_report("run-42", &[])` is called
**Then**:
- `report.failure_found == false`
- `report.failure_code == ""`
- `report.failed_at_step == None`
- `report.side_effects.is_empty()`
- `report.repair_hints.is_empty()`

### T-002: given_step_started_then_run_failed_when_building_report_then_failure_is_true_with_step
**Given**: `[StepStarted{step:1}, RunFailedEvent{..}]` and run_id `"run-7"`
**When**: `build_incident_report("run-7", events)` is called
**Then**:
- `report.failure_found == true`
- `report.failure_code == "RunFailed"`
- `report.failed_at_step == Some(1)`
- `report.side_effects.is_empty()`

### T-003: given_action_completed_then_run_failed_when_building_report_then_side_effect_has_confirmed_certainty
**Given**: `[StepStarted{step:2}, ActionCompletedEvent{step:2, action:"deploy"}, RunFailedEvent{..}]`
**When**: `build_incident_report("run-x", events)` is called
**Then**:
- `report.side_effects.len() == 1`
- `report.side_effects[0]["certainty"] == "confirmed"`
- `report.side_effects[0]["step"] == 2`
- `report.failure_found == true`
- `report.failure_code == "RunFailed"`

### T-004: given_action_failed_then_run_failed_when_building_report_then_side_effect_has_failed_certainty
**Given**: `[StepStarted{step:3}, ActionFailedEvent{step:3, action:"build"}, RunFailedEvent{..}]`
**When**: `build_incident_report("run-y", events)` is called
**Then**:
- `report.side_effects.len() == 1`
- `report.side_effects[0]["certainty"] == "failed"`
- `report.side_effects[0]["step"] == 3`

### T-005: given_mixed_actions_then_run_failed_when_building_report_then_both_side_effects_present
**Given**: `[StepStarted{step:1}, ActionCompletedEvent{step:1, action:"init"}, StepStarted{step:2}, ActionFailedEvent{step:2, action:"deploy"}, RunFailedEvent{..}]`
**When**: `build_incident_report("run-z", events)` is called
**Then**:
- `report.side_effects.len() == 2`
- `report.side_effects[0]["certainty"] == "confirmed"`
- `report.side_effects[1]["certainty"] == "failed"`
- `report.failed_at_step == Some(2)` (last StepStarted before failure)

### T-006: given_step_started_then_run_cancelled_when_building_report_then_cancelled_code
**Given**: `[StepStarted{step:5}, RunCancelled{..}]` and run_id `"run-cancel"`
**When**: `build_incident_report("run-cancel", events)` is called
**Then**:
- `report.failure_found == true`
- `report.failure_code == "RunCancelled"`
- `report.failed_at_step == Some(5)`

### T-007: given_multiple_step_started_when_building_report_then_last_step_before_failure
**Given**: `[StepStarted{step:1}, ActionCompletedEvent{..}, StepStarted{step:4}, ActionCompletedEvent{..}, StepStarted{step:7}, RunFailedEvent{..}]`
**When**: `build_incident_report("run-multi", events)` is called
**Then**:
- `report.failed_at_step == Some(7)`
- `report.side_effects.len() == 2`

### T-008: given_unknown_event_variants_when_building_report_then_no_panic
**Given**: A slice containing `JournalEvent` variants not explicitly matched (via `JournalEvent` construction that exercises unhandled variants)
**When**: `build_incident_report("run-unknown", events)` is called
**Then**:
- Returns `IncidentReport` without panicking
- `report.failure_found == false`

## Unit Tests: `build_repair_hints` (commands_incident.rs)

### T-009: given_run_failed_with_no_side_effects_when_building_hints_then_single_hint
**Given**: `failure_code="RunFailed"`, `side_effects=&[]`, `failed_at_step=None`
**When**: `build_repair_hints("RunFailed", &[], None)` is called
**Then**:
- `hints.len() == 1`
- `hints[0]` contains `"investigate step output and engine logs"`

### T-010: given_run_failed_with_side_effects_and_step_when_building_hints_then_three_hints
**Given**: `failure_code="RunFailed"`, `side_effects=[value]`, `failed_at_step=Some(3)`
**When**: `build_repair_hints("RunFailed", side_effects, Some(3))` is called
**Then**:
- `hints.len() == 3`
- hint 0: `"investigate step output"`
- hint 1: `"review side effects"`
- hint 2: `"consider retry from step 3"`

### T-011: given_run_cancelled_with_no_side_effects_when_building_hints_then_single_hint
**Given**: `failure_code="RunCancelled"`, `side_effects=&[]`, `failed_at_step=None`
**When**: `build_repair_hints("RunCancelled", &[], None)` is called
**Then**:
- `hints.len() == 1`
- `hints[0]` contains `"run was cancelled"`

### T-012: given_run_cancelled_with_side_effects_when_building_hints_then_two_hints
**Given**: `failure_code="RunCancelled"`, `side_effects=[value]`, `failed_at_step=Some(1)`
**When**: `build_repair_hints("RunCancelled", side_effects, Some(1))` is called
**Then**:
- `hints.len() == 2`
- hint 0: `"run was cancelled"`
- hint 1: `"review completed side effects for partial cleanup"`

### T-013: given_unknown_failure_code_when_building_hints_then_empty_hints
**Given**: `failure_code="SomethingElse"`, `side_effects=[value]`, `failed_at_step=Some(1)`
**When**: `build_repair_hints("SomethingElse", side_effects, Some(1))` is called
**Then**:
- `hints.is_empty()`

## Integration Tests: `cmd_incident` (app_impl.rs)

### T-014: given_failed_run_when_cmd_incident_then_json_output_has_failure_code
**Given**: A FjallJournal database containing events for run `"run-1"`:
- `RunAccepted`, `StepStarted{step:1}`, `ActionCompletedEvent{step:1, action:"init"}`, `RunFailedEvent`
**When**: `cmd_incident("run-1", db_path, OutputFormat::Json)` is called
**Then**:
- Stdout contains valid JSON
- JSON contains `"failure_code": "RunFailed"`
- JSON contains `"failure_found": true`
- Exit code is `CliExitCode::Success`

### T-015: given_nonexistent_run_when_cmd_incident_then_structured_error_no_stack_trace
**Given**: A FjallJournal database that does NOT contain events for run `"nonexistent"`
**When**: `cmd_incident("nonexistent", db_path, OutputFormat::Json)` is called
**Then**:
- Stdout contains JSON with `"success": false` and `"error"` field
- No stack trace or `std::backtrace` text in output
- Exit code is `CliExitCode::StorageError`

### T-016: given_successful_run_when_cmd_incident_then_not_an_incident_exit_code
**Given**: A FjallJournal database containing events for run `"run-ok"`:
- `RunAccepted`, `StepStarted{step:1}`, `ActionCompletedEvent{step:1, action:"deploy"}`, `RunFinished`
**When**: `cmd_incident("run-ok", db_path, OutputFormat::Json)` is called
**Then**:
- Stdout contains JSON with `"failure_found": false`
- Exit code is `CliExitCode::StorageError` (run succeeded, not an incident)

---

**Written by**: rust-contract agent
**Bead**: vb-qi37.17.1
**Date**: 2026-05-17
