# Martin Fowler Test Plan — vb-oaom: cli: Add runtime ai context packet command

## Happy Path Tests

- `test_ai_context_for_valid_run_includes_state_diagnostics_and_next_commands` — Given a run with journal events and a valid run_id, When `ai-context` is invoked, Then the packet includes `journal_event_trail`, `suggested_next_cli_commands`, and `workflow` fields.
- `test_json_output_includes_citations_and_redaction_status` — Given a run with taint-annotated slots, When `ai-context` is invoked with `--json`, Then secret-tainted slots show `[REDACTED]` and clean slots show their string representation.
- `test_ai_context_packet_schema_fields` — Given a valid run, When `ai-context` is invoked, Then the output JSON object contains `schema_version`, `kind: "AiContextPacket"`, `run_id`, `workflow`, `journal_event_trail`, `action_contracts`, `trace_ring_snapshot`, and `suggested_next_cli_commands`.
- `test_ai_context_workflow_field_contains_digest` — Given a run with a known compiled workflow, When `ai-context` is invoked, Then `workflow.digest` is a 64-character hex string.
- `test_ai_context_workflow_field_contains_compiled_ir` — Given a run with a compiled workflow, When `ai-context` is invoked, Then `workflow.compiled_ir.available` is `true` and `compiled_ir.name` is present.
- `test_action_contracts_inferred_from_do_nodes` — Given a compiled workflow with `Do` nodes, When `ai-context` is invoked, Then `action_contracts` contains the action IDs from those nodes.
- `test_suggested_ai_commands_for_finished_run` — Given a finished run, When `ai-context` is invoked, Then `suggested_next_cli_commands` contains `replay` as the final recommended command.
- `test_suggested_ai_commands_for_running_run` — Given a running run, When `ai-context` is invoked, Then `suggested_next_cli_commands` contains `trace` and `resume`.
- `test_suggested_ai_commands_for_failed_run` — Given a failed run, When `ai-context` is invoked, Then `suggested_next_cli_commands` contains `incident` and `retry`.
- `test_jsonl_output_format` — Given valid arguments, When `ai-context` is invoked with `--jsonl`, Then output is valid JSON on a single line.

## Error Path Tests

- `test_unknown_run_returns_structured_not_found_diagnostic` — Given a run_id with no journal events, When `ai-context` is invoked, Then exit code is `ValidationFailed` and output JSON contains `"code": "RUN_NOT_FOUND"`.
- `test_invalid_run_id_returns_validation_failed` — Given a non-numeric run_id string, When `ai-context` is parsed, Then exit code is `ValidationFailed`.
- `test_parse_run_id_rejects_non_numeric` — Given run_id `"abc"`, When `parse_run_id` is called, Then it returns an error.
- `test_parse_run_id_rejects_overflow` — Given run_id `"18446744073709551616"` (u64::MAX + 1), When `parse_run_id` is called, Then it returns an error.
- `test_journal_open_failure_returns_storage_error` — Given `--db /nonexistent/path`, When `ai-context` is invoked, Then exit code is `StorageError` and output JSON contains `"error"` with the path.
- `test_handle_unopenable_db` — Given a path that cannot be opened as FjallJournal, When `handle` is called, Then `CliExitCode::StorageError` is returned.
- `test_journal_read_error_propagates` — Given a journal that returns an error on event read, When `handle` is called, Then a structured error JSON is output with the area that failed.
- `test_latest_snapshot_from_events_propagates_error` — Given snapshot lookup returns `JournalError::WriteLockPoisoned`, When `latest_snapshot_from_events` is called, Then the error is propagated.
- `test_ai_context_latest_snapshot_from_events_propagates_snapshot_lookup_error` — Existing test: `JournalError::WriteLockPoisoned` propagates from snapshot lookup through `latest_snapshot_from_events`.

## Edge Case Tests

- `test_tainted_payload_is_redacted_in_context_packet` — Given a slot that is `Secret`-tainted in the snapshot taint table, When `redacted_slot_value` is called, Then `[REDACTED]` is returned.
- `test_undecodable_slot_value_returns_undecoded` — Given a slot with malformed Postcard bytes, When `redacted_slot_value` is called, Then `[UNDECODED]` is returned.
- `test_redacted_slot_value_returns_redacted_for_secret` — Given `slot_is_secret_or_derived` returns `true` for taint level 2 (Secret), When `redacted_slot_value` is called, Then `[REDACTED]` is returned.
- `test_redacted_slot_value_returns_redacted_for_derived` — Given a slot with taint level 1 (DerivedFromSecret), When `redacted_slot_value` is called, Then `[REDACTED]` is returned.
- `test_suggested_commands_length_bounded` — Given any run status, When `suggested_ai_commands` is called, Then the result list has length ≤ 4.
- `test_suggested_commands_all_start_with_velvet_ballastics` — Given a valid run, When `suggested_ai_commands` is called, Then every string in the result starts with `"velvet-ballastics "`.
- `test_action_contracts_unique` — Given a workflow with duplicate action IDs in `Do` nodes, When `ai_action_contracts` is called, Then the result contains no duplicate action IDs.
- `test_workflow_digest_from_events_finds_run_accepted` — Given events containing `RunAccepted`, When `workflow_digest_from_events` is called, Then `Some(digest)` is returned.
- `test_workflow_digest_from_events_returns_none_for_no_run_accepted` — Given events with no `RunAccepted`, When `workflow_digest_from_events` is called, Then `None` is returned.

## Contract Verification Tests

- `test_precondition_run_id_parse_valid` — Verify PRE-001 by asserting `parse_run_id("123")` is `Ok(RunId::new(123))`.
- `test_precondition_run_id_parse_invalid` — Verify PRE-001 by asserting `parse_run_id("abc")` is `Err(...)`.
- `test_postcondition_redaction_always_string` — Verify POST-003 by asserting `redacted_slot_value` always returns a `Value::String`.
- `test_invariant_read_only_no_journal_write` — Verify INV-001 by asserting `handle` performs no `put_*` or `append` calls on the journal (checked by static analysis + mutation testing).
- `test_invariant_suggested_commands_bounded_length` — Verify INV-002 by property-testing `suggested_ai_commands` across all `RunStatus` variants.
- `test_invariant_all_suggested_commands_are_real_subcommands` — Verify INV-003 by checking each string in `suggested_ai_commands` against `VALID_COMMANDS`.

## Given-When-Then Scenarios

### Scenario 1: ai-context for a valid finished run includes state diagnostics and next commands

**Given**: a run with run_id `42` that has completed successfully (has `RunFinished` event), stored in journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 42 --db /tmp/test-db --json`

**Then**:
- Exit code is `SUCCESS`
- JSON output contains `kind: "AiContextPacket"`
- `journal_event_trail` is a non-empty array
- `suggested_next_cli_commands` includes `velvet-ballastics replay 42 --db /tmp/test-db --json`
- `workflow.compiled_ir.available` is `true`

### Scenario 2: json output includes citations and redaction status

**Given**: a run with one slot written that is `Secret`-tainted, stored in journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 42 --db /tmp/test-db --json`

**Then**:
- The slot value in `journal_event_trail` is `"[REDACTED]"`
- No raw secret bytes appear in the output
- `suggested_next_cli_commands` is a non-empty array of valid CLI invocations

### Scenario 3: unknown run returns structured not-found diagnostic

**Given**: run_id `99999` which does not exist in the journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 99999 --db /tmp/test-db --json`

**Then**:
- Exit code is `ValidationFailed`
- JSON output contains `"success": false` and `"code": "RUN_NOT_FOUND"`
- No `AiContextPacket` is emitted

### Scenario 4: tainted payload is redacted in context packet

**Given**: a run with a `Secret`-tainted slot value, stored in journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 42 --db /tmp/test-db --json`

**Then**:
- The redacted slot value in the packet is exactly `"[REDACTED]"`
- The redaction status for that slot is `Secret` or `DerivedFromSecret`
- No raw slot value bytes appear in the output

### Scenario 5: ai-context for a failed run suggests incident and retry

**Given**: a run with `RunFailedEvent` in its journal trail, stored in journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 42 --db /tmp/test-db --json`

**Then**:
- `suggested_next_cli_commands` contains `velvet-ballastics incident 42 --db /tmp/test-db --json`
- `suggested_next_cli_commands` contains `velvet-ballastics retry 42 --db /tmp/test-db --json`

### Scenario 6: ai-context for a running run suggests trace and resume

**Given**: a run with `RunAccepted` but no terminal event, stored in journal at `/tmp/test-db`

**When**: the operator runs `velvet-ballastics ai-context 42 --db /tmp/test-db --json`

**Then**:
- `suggested_next_cli_commands` contains `velvet-ballastics trace 42 --db /tmp/test-db --json`
- `suggested_next_cli_commands` contains `velvet-ballastics resume 42 --db /tmp/test-db --json`
