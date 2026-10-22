# Rust Contract: Existing Test Fixture Migration to run_attempt Semantics

## Problem

vb-wop2u (LOCAL5090) changed production API for run-attempt ticket authority:
- `ActionTicket` gained `run_attempt: u16` field
- `compute_action_idempotency_key` signature changed to `(run, run_attempt, seq, action)`
- `issue_action_ticket` gained `run_attempt` parameter
- Various drive/issue functions now require run_attempt propagation

Existing test fixtures, benches, fuzz targets, and cfg(test) helpers were partially
updated by a previous LOCAL5090 snapshot but have 148+ compile errors. This bead
completes the mechanical migration of test-only code to match the new signatures.

## Scope

OWNED: All test fixture code, integration tests, benches, fuzz targets, and cfg(test)
helper functions that construct ActionTicket values, call compute_action_idempotency_key,
call issue_action_ticket, or call drive/issue functions requiring run_attempt.

NOT OWNED: Production functions, structs, or behavior. Not durable_retry_execution.rs or
durable_retry_run_attempt_authority.rs (LOCAL5090 owns). Not compiler files (repaired separately).

## Migration Semantics

1. **First-run fixtures**: Use `run_attempt: 1` (initial durable run attempt).
2. **Retry fixtures**: Use appropriate higher values (2, 3) when testing explicit retry paths.
3. **Never**: Blindly use `ticket.attempt` as `run_attempt`; they have distinct meanings.
4. **Never**: Force `run_attempt: 1` in tests intentionally modeling later run attempts.
5. **Preserve**: Proptest/fuzz arbitrary inputs where structural diversity is needed.
6. **Preserve**: All assertions, expected errors, and test semantics. Do not weaken.

## Verification

1. `cargo check --tests --workspace` compiles clean (production-owned failures reported separately)
2. Relevant test suites run and report actual pass/fail counts
3. No fixture semantics weakened or silently changed
