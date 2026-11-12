# vb-wop2u Production Half — LOCAL RTX5090 Qwen3.8-27B Handoff

## Status

Production durable run-attempt ticket authority is complete. The core issue — old `ActionTicket`s from a failed first attempt being replayed against a retried second attempt — is fixed by binding `run_attempt` to the ticket and validating it before any journal mutation.

## What Was Done

### Production Code
The previous 5090 iteration (codex checkpoint) already implemented the complete production code:
- `ActionTicket.run_attempt: u16` field added (`crates/vb_core/src/action/ticket.rs`)
- `compute_action_idempotency_key(run, run_attempt, seq, action)` now includes run_attempt
- `issue_action_ticket` accepts run_attempt parameter
- Engine dispatch passes `state.run_attempt.get()` to `execute_do`
- Retry tickets preserve `original_ticket.run_attempt` (per-action retry keeps same run attempt)
- `validate_ticket_attempt` in shard helpers validates `ticket.run_attempt == state.run_attempt` with `StaleAttempt { incoming, current }` rejection
- Validation runs in preflight BEFORE any journal append in both completion and failure paths
- Recovery path (`handle_recover`) restores checkpoint's run_attempt into restored RunState
- AskTicket uses `boundary_seq` (durable per-ask sequence) for authority — no run_attempt field needed; `validate_pending_ask_answer` compares full ticket including boundary_seq

No additional production code changes were required beyond the existing codex checkpoint work. All production code compiles cleanly.

### New Permanent Regression Test
Created `crates/vb_runtime/tests/durable_retry_run_attempt_authority.rs` with 4 genuine end-to-end tests:
1. **stale_old_ticket_rejected_after_durable_retry** — Produces actual failed run, recovers via `recover_retry`, replays old ticket (run_attempt=1) against new pending ticket (run_attempt=2). Asserts exact `StaleAttempt` rejection with expected/found values. Verifies runtime is unchanged (no counter mutation). Also verifies old failure ticket is rejected. New ticket completion succeeds.
2. **run_attempt_advances_but_per_action_attempt_does_not** — Confirms run_attempt increments (1→2) while per-action attempt resets (stays at 1) on run retry.
3. **future_run_attempt_ticket_rejected** — Fabricates ticket with run_attempt=3 (future). Asserts rejection.
4. **zero_run_attempt_ticket_rejected** — Fabricates ticket with run_attempt=0. Asserts rejection.

All 4 tests pass with the real Runtime, real FjallJournal, and real durable checkpoint producer.

### Updated Existing Test
Updated `crates/vb_runtime/tests/durable_retry_execution.rs` line 239 to assert `retry_ticket.run_attempt == 2` after durable retry, confirming the new ticket authority.

## Verification

```bash
# Production compilation
cargo check --workspace  # PASS (exit 0, 12.7s)

# New regression tests
cargo test --package vb_runtime --test durable_retry_run_attempt_authority -- --nocapture
# PASS: 4/4 tests
# - stale_old_ticket_rejected_after_durable_retry
# - run_attempt_advances_but_per_action_attempt_does_not
# - future_run_attempt_ticket_rejected
# - zero_run_attempt_ticket_rejected

# Updated existing durable retry test
cargo test --package vb_runtime --test durable_retry_execution -- --nocapture
# PASS: 2/2 tests (durable_retry_reopens_and_completes_failed_step, checkpoint_roundtrips_full_retry_authority)
```

## Pending Blockers (LOCAL 3090 vb-3gthe Scope)

The existing test fixtures across the workspace use the old ActionTicket layout without `run_attempt`. These are in the 3090 workspace's scope to fix. Summary of failures from `cargo check --workspace --tests`:

**crates/vb_runtime (many errors):**
- `src/action/tests.rs`: 5 missing `run_attempt` fields in ticket initializers
- `src/action_queue.rs` (test module): 1 missing `attempt` field
- `src/engine/action.rs` tests: 16+ missing run_attempt argument to `compute_idempotency_key`
- `src/engine/action.rs` tests: execute_do call missing state_run_attempt arg
- Many shard lifecycle test files need run_attempt in ticket fixtures

**crates/vb_core:**
- Test modules with ticket initializers

**crates/vb_storage:**
- `src/codec/tests.rs`: `codec_action_ticket` helper missing run_attempt

**crates/workspace_tests:**
- `vb_test_runtime_ipc_resource_behavior.rs`: 3 missing run_attempt fields

Total: ~148 test compilation errors, all in `cfg(test)` modules or test files. No production code errors.

## No Changes Needed Beyond Production

The production ticket authority mechanism is complete. The fix is the run_attempt field bound to the ticket, included in the idempotency key, validated before mutation. No new lint hacks, no unsafe, no unwrap/expect/panic, no feature flag hacks.

## Evidence Files

- `.beads/vb-wop2u/evidence/wop2u_check_new_test.*` — production compile
- `.beads/vb-wop2u/evidence/wop2u_test_run_attempt_authority.*` — new regression tests (4/4 pass)
- `.beads/vb-wop2u/evidence/wop2u_test_existing_retry.*` — updated existing test (2/2 pass)
- `.beads/vb-wop2u/evidence/wop2u_check_production.*` — production-only compile
- `.beads/vb-wop2u/evidence/wop2u_check_storage_tests.*` — workspace test errors (3090 scope)
- `.beads/vb-wop2u/evidence/wop2u_check_runtime_tests.*` — runtime test errors (3090 scope)