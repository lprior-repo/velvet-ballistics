# vb-ymb0f Independent Code Review Findings

**Reviewed:** `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`
**Candidate:** 1e12ef61 (HEAD, codex/vb-16jfp-cli-candidate)
**Base:** e72b905de
**Reviewer:** codex/vb-ymb0f-code-review (local 3090, new context)
**Date:** 2026-10-21

## Verdict: APPROVED

All six previously rejected test cases now use genuine captured ActionTickets and exact typed errors. No panic/unwrap/expect patterns. All 18 tests compile and pass.

## Detailed Findings

### 1. Fake ActionTicket Helper — FIXED (CRITICAL)
- **Previous:** `action_ticket(run, action)` helper fabricated tickets with hardcoded `step: StepIdx::ZERO`, `seq: SeqNo::ZERO`, `attempt: 1` — not the genuine runtime-issued tickets.
- **Now:** Helper removed entirely. All six tests capture genuine tickets via `runtime.lookup_pending_action_ticket(run, 0)` BEFORE cancel/kill.
- **Tests affected:** HP-3, HP-4, C4-complete, C4-fail, C4-stale, K5. ✓

### 2. Genuine Ticket Identity Verification — ADDED (CRITICAL)
- All six tests assert `lookup_pending_action_ticket(run, 0).is_none()` AFTER cancel/kill, proving the pending action was genuinely removed from the shard's index.
- This verifies the fix isn't just using a different fake ticket — the real one exists and is removed. ✓

### 3. Exact Typed Error Verification — FIXED (CRITICAL)
- **Completion after terminal (HP-4, C4-complete, C4-stale, K5):** Assert exact `Err(RuntimeError::RunNotFound)` — run moved to `terminal_runs` after cancel/kill. ✓
- **Failure after terminal (HP-3, C4-fail):** Assert exact `Err(RuntimeError::InvalidActionCompletion)` — fail_action on cancelled run. ✓
- Previous vague `is_err()` checks eliminated.

### 4. Journal Non-Mutation Invariants — ADDED (CRITICAL)
- All six tests capture journal state before stale action attempt.
- All six assert journal length unchanged after (full snapshot equality for C4-stale and K5, length for others).
- C4-stale additionally verifies counter non-mutation (`runs_failed`, `runs_completed`). ✓

### 5. Padding Contract Capability Declarations — FIXED (CRITICAL)
- `action_contract` now accepts optional `required: Option<Capability>` parameter.
- Padding contracts (IDs 0-6) pass `None` → empty `required_capabilities`.
- Real action contract (ID 7) passes `Some(required_capability(current))`.
- Fixes `AdmissionCapabilityDenied` at submit — all tests reach runtime logic. ✓

### 6. Code Quality Attributes
- No `panic!`, `unwrap()`, `expect()`, or `.ok()` calls in the test file. ✓
- No `dbg!` or `todo!` or `unimplemented!`. ✓
- Imports cleaned: removed unused `ActionTicket` and `SeqNo`. ✓

## Test Execution Evidence

```
PROPTEST_CASES=256 cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests

running 18 tests
test kill_missing_run_returns_run_not_found_error ... ok
test ec1_terminal_cancelled_state_does_not_regress ... ok
test action_failure_after_cancel_returns_error ... ok
test action_completion_after_cancel_returns_error ... ok
test hp4_action_after_cancel_returns_error ... ok
test cancel_missing_run_returns_run_not_found_error ... ok
test second_cancel_after_first_cancel_retains_one_event ... ok
test cancel_after_kill_is_idempotent ... ok
test inv1_terminal_never_regresses_after_cancel ... ok
test inv1_completed_run_terminal_never_regresses ... ok
test action_completion_after_kill_returns_error ... ok
test stale_action_after_cancel_does_not_mutate_state ... ok
test hp3_cancel_action_suspended_run_removes_pending_action ... ok
test kill_live_run_produces_runkilled_journal_event ... ok
test second_kill_after_first_kill_produces_no_extra_event ... ok
test hp1_cancel_running_run_transitions_to_cancelled ... ok
test cancel_terminal_run_produces_no_side_effects ... ok
test kill_on_cancelled_run_is_idempotent ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Disposition

**APPROVED** — All findings from previous rejection (fake tickets, vague error checks, missing journal invariants, padding capability contract) are fixed with genuine captured tickets and exact typed errors verified by compilation and execution.
