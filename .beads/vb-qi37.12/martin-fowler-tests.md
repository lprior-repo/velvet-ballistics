# Martin Fowler Test Plan: vb-qi37.12

This is a specification artifact only. It does not implement tests.

## Happy Path Tests
- `given_strict_journal_persist_succeeds_when_runtime_mutates_then_success_ack_and_journal_evidence_exist`
- `given_valid_compiler_inputs_when_validation_runs_then_no_errors_are_accumulated_or_discarded`

## Error Path Tests
- `given_strict_journal_persist_failure_when_runtime_mutates_then_typed_storage_error_reaches_caller`
- `given_process_lock_write_failure_when_acquire_runs_then_typed_error_or_explicit_best_effort_contract_is_observed`
- `given_engine_drive_error_when_apply_drive_result_handles_it_then_terminal_failure_retains_cause`
- `given_multiple_compiler_validation_errors_when_validation_runs_then_all_causes_remain_in_compile_errors`

## Edge Case Tests
- `given_absent_optional_slot_payload_when_accessor_runs_then_absence_is_distinct_from_corruption`
- `given_corrupt_slot_payload_when_recovery_decodes_then_typed_corruption_error_is_returned`
- `given_drop_time_persist_failure_when_no_explicit_close_was_called_then_no_success_durability_claim_is_made`

## Contract Verification Tests
- `given_scoped_production_fallible_sites_when_inventory_runs_then_each_site_has_classification`
- `given_best_effort_discard_on_release_critical_path_when_gate_runs_then_gate_rejects`
- `given_source_contains_ignored_result_when_static_gate_runs_then_unclassified_discard_fails`

## Given-When-Then Scenarios
### Scenario 1: Persist failure is never acknowledged as success
Given: a runtime mutation requires strict journal persistence.
When: the storage persist boundary fails.
Then: the caller receives a typed storage error and no success acknowledgement or externally visible success state is emitted.

### Scenario 2: Recovery corruption fails closed
Given: persisted event payload bytes are corrupt or truncated.
When: recovery or replay decodes the event on a recovery-critical path.
Then: recovery returns a typed corruption/replay error and does not hydrate an empty successful frame.

### Scenario 3: Explicit discard exceptions are narrow
Given: a production path uses a best-effort discard.
When: the static inventory gate classifies the site.
Then: the site is accepted only if it has a typed discard API, rationale, operation name, and non-critical scope.
