# Review Handoff: Run-Attempt Ticket Authority (vb-xvqwl)

## Review Status

Complete. Both code-review.md and test-review.md written. Decision: APPROVED for the run-attempt ticket authority source delta.

## What Was Reviewed

- ActionTicket struct change (7→8 fields, added run_attempt)
- Idempotency key computation updated to include run_attempt
- Ticket issuance paths (handle_do, execute_do) propagate state.run_attempt
- Ticket validation (validate_ticket_attempt) rejects stale tickets before side effects
- Checkpoint capture increments run_attempt on failure
- Recovery (handle_recover, RecoverRunCommand::for_retry) restores incremented run_attempt
- AskTicket generation tracking via boundary_seq (independent of run_attempt)
- Compatibility: old encoded tickets without run_attempt will fail to deserialize (breaking wire change)

## What Was Not Introduced by This Change (Known Issues B/C/D)

- B: Automatic terminal recovery
- C: Strict retry sync barrier
- D: Raw checkpoint durable chain binding

These are being repaired by DeepSeek vb-hz9v8.

## Independent Tests Run

- durable_retry_run_attempt_authority: 4/4 pass (new tests)
- durable_retry_execution: 2/2 pass (updated existing tests)
- durable_retry_red_phase: 9/9 pass
- ticket_authority_matrix: 2/2 pass

## Findings

No defects found in the run-attempt ticket authority source delta. All production issuance, validation, recovery, and executor paths correctly use the incremented run_attempt. No hardcoded run_attempt=1 on production retry paths. Per-action attempt counter is independent and correctly bounded.
