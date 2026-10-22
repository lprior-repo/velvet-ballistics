# Code Review: Run-Attempt Ticket Authority (vb-xvqwl)

**Frozen SHA:** 9954545d3f816ec360b9ecd07ea07dfd77a27767
**Baseline SHA:** 423d8ae8b667d82b4d36e57d29d7ef2aede08bdf
**Branch:** codex/vb-wop2u-local5090-ticket-authority
**Reviewer:** Local RTX5090 Qwen3.8-27B via OMP high reasoning

## Summary

This change adds run-attempt authority to action tickets to fix the stale ticket counterexample where old tickets from attempt 1 are accepted on attempt 2 after a durable run failure and explicit retry. The fix adds `run_attempt` to `ActionTicket` and propagates it through all issuance, validation, and recovery paths.

## Decision

APPROVED

The implementation correctly addresses the cold Runtime/Fjall stale ticket counterexample. The run_attempt is tracked, incremented on retry, included in ticket issuance and idempotency key computation, and validated before accepting any ticket side effects.

## Evidence

### Positive: Run attempt is validated before any ticket side effect

**Path:** `crates/vb_runtime/src/shard/helpers.rs` (validate_ticket_attempt)
```rust
// Run attempt authority: ticket must match current run attempt.
let expected = state.run_attempt.get();
if ticket.run_attempt != expected {
    return Err(RuntimeError::StaleAttempt {
        incoming: ticket.run_attempt,
        current: expected,
    });
}
```

This is the first validation check in the function, so no side effects can occur with a stale ticket.

### Positive: Checkpoint carries incremented run attempt

**Path:** `crates/vb_runtime/src/shard/transitions.rs` (capture_retry_checkpoint)
```rust
let next_attempt = match state
    .run_attempt
    .get()
    .checked_add(1)
    .and_then(core::num::NonZeroU16::new)
{
    Some(attempt) => attempt,
    None => return Ok(()),
};
```

The next checkpoint's run_attempt is incremented by 1 before capture. This is then persisted and restored during retry.

### Positive: Recovery restores run attempt from checkpoint

**Path:** `crates/vb_runtime/src/run_checkpoint.rs` (restore)
```rust
let state = RunState {
    registered_execution: crate::shard::types::RegisteredExecutionMode::ReconcileOnly,
    frame,
    run_attempt: self.run_attempt,
    ...
};
```

The restored RunState carries the incremented run_attempt. All subsequent tickets issued during the retry use this incremented value.

### Positive: Idempotency key includes run attempt

**Path:** `crates/vb_core/src/action/ticket.rs` (compute_action_idempotency_key)
```rust
pub fn compute_action_idempotency_key(run: RunId, run_attempt: u16, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let attempt_part = u128::from(run_attempt);
    ...
}
```

Different run attempts produce different idempotency keys, preventing replay of completions from previous attempts.

### Positive: AskTicket uses boundary_seq for generation tracking

AskTicket does not need an explicit run_attempt field because it uses `boundary_seq` (journal event sequence number) for generation tracking. On recovery, the journal sequence is re-seeded, so old ask answers are rejected with a different boundary_seq.

## Verified Risks

### Risk: No hardcoded run_attempt=1 on production retry paths

All production paths correctly pass `state.run_attempt` through the call chain. The only hardcoded `1` is in `drive_with_actions`, which is a test utility.

### Risk: Run attempt never overflows

The increment uses `checked_add(1)` with `NonZeroU16::new` for overflow detection. This is safe for all practical retry counts.

### Risk: Per-action attempt counter is independent of run attempt

Verified: After a durable run retry, run_attempt advances but per-action attempt resets to 1. This is correct because per-action retry is a separate bounded retry policy within a single run attempt.

## Known Issues (Not Introduced by This Change)

These are pre-existing blockers being repaired by DeepSeek vb-hz9v8:
- **B:** Automatic terminal recovery
- **C:** Strict retry sync barrier  
- **D:** Raw checkpoint durable chain binding

## Compatibility Notes

ActionTicket struct layout changed from 7 to 8 fields. Old encoded tickets without run_attempt will fail to deserialize (serde default values not used). This is a breaking wire change that requires version bump or migration for IPC/storage compatibility. All production issuance and validation paths have been updated.
