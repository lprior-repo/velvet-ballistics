bead_id: vb-qi37.16.5
bead_title: lifecycle replay and journal persistence controls
phase: State 14 final manual QA
updated_at: 2026-05-12T03:14:12Z

# Manual QA Final

STATUS: PASS

## Workspace Guard

All commands were run with workdir `/home/lewis/src/Velvet-ballistics-vb-qi37-16-5-go`. Source checkout `/home/lewis/src/Velvet-ballistics` was not touched.

## Commands and Verbatim Outcomes

### 1. Lifecycle integration suite

Command:
```bash
cargo test --package velvet_ballistics --test lifecycle_integration -- --test-threads=1
```

Exit: `0`

Stdout excerpt:
```text
running 43 tests
test answer_returns_duplicate_request_when_called_twice ... ok
test answer_returns_invalid_transition_when_bead_is_active ... ok
test answer_returns_invalid_transition_when_bead_is_cancelled ... ok
test answer_returns_invalid_transition_when_bead_is_completed ... ok
test answer_returns_invalid_transition_when_bead_is_failed ... ok
test answer_returns_invalid_transition_when_bead_is_pending ... ok
test answer_returns_stale_request_when_not_in_waiting_answer_state ... ok
test answer_succeeds_when_bead_is_waiting_answer ... ok
test cancel_returns_duplicate_request_when_called_twice ... ok
test cancel_returns_invalid_transition_when_bead_is_completed ... ok
test cancel_returns_invalid_transition_when_bead_is_failed ... ok
test cancel_returns_invalid_transition_when_bead_is_pending ... ok
test cancel_returns_stale_request_when_state_already_advanced ... ok
test cancel_succeeds_when_bead_is_active ... ok
test cancel_succeeds_when_bead_is_waiting_answer ... ok
test duplicate_request_error_includes_structured_diagnostics ... ok
test each_successful_command_appends_exactly_one_event ... ok
test invalid_transition_error_includes_structured_diagnostics ... ok
test lifecycle_command_returns_journal_write_failure_on_io_error ... ok
test lifecycle_command_returns_storage_unavailable_when_not_connected ... ok
test no_state_has_self_loop_transition ... ok
test replay_from_empty_journal_produces_valid_initial_state ... ok
test replay_full_journal_reconstructs_bit_identical_state ... ok
test replay_with_malformed_event_returns_replay_corruption ... ok
test replay_with_missing_event_returns_replay_corruption ... ok
test resume_returns_duplicate_request_when_called_twice ... ok
test resume_returns_invalid_transition_when_bead_is_active ... ok
test resume_returns_invalid_transition_when_bead_is_completed ... ok
test resume_returns_invalid_transition_when_bead_is_failed ... ok
test resume_returns_invalid_transition_when_bead_is_pending ... ok
test resume_returns_invalid_transition_when_bead_is_waiting_answer ... ok
test resume_returns_stale_request_when_not_in_cancelled_state ... ok
test resume_succeeds_when_bead_is_cancelled ... ok
test retry_returns_duplicate_request_when_called_twice ... ok
test retry_returns_invalid_transition_when_bead_is_active ... ok
test retry_returns_invalid_transition_when_bead_is_cancelled ... ok
test retry_returns_invalid_transition_when_bead_is_completed ... ok
test retry_returns_invalid_transition_when_bead_is_pending ... ok
test retry_returns_invalid_transition_when_bead_is_waiting_answer ... ok
test retry_returns_stale_request_when_not_in_failed_state ... ok
test retry_succeeds_when_bead_is_failed ... ok
test stale_request_error_includes_structured_diagnostics ... ok
test valid_transition_graph_contains_all_expected_edges ... ok

test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
```

### 2. CLI help discovery

Command:
```bash
target/debug/velvet-ballistics --help
```

Exit: `0`

Stdout excerpt:
```text
velvet-ballistics - compiled workflow runtime

commands:
  replay     <run_id> --db <path> [--json|--jsonl]     Replay a run from journal
  retry      <run_id> --db <path> [--json|--jsonl]     Retry a failed run from last successful step
  resume     <run_id> --db <path> [--json|--jsonl]     Resume a suspended run
  answer     <run_id> --step <N> --value-file <file> --db <path> [--json|--jsonl]  Answer a suspended step
```

### 3. CLI version

Command:
```bash
target/debug/velvet-ballistics version
```

Exit: `0`

Stdout:
```text
velvet-ballistics 0.1.0
```

### 4. Doctor smoke

Command:
```bash
target/debug/velvet-ballistics doctor --db /tmp/velvet-final-qa-vb-qi37-16-5-db
```

Exit: `0`

Stdout:
```text
doctor: trim eligibility — 0 total, 0 eligible, 0 blocked, 0 events trimmable
doctor: all checks passed
```

## Decision

Final manual QA passed. Lifecycle integration controls and CLI discovery/version/doctor smoke are operational after State13 with no code changes.
