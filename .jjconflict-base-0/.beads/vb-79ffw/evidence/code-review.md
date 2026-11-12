# Code Review: vb-79ffw

**Reviewer:** Independent local3090 review
**Baseline:** af142d34 (frozen)
**Status:** APPROVED with findings

## Architecture Assessment

The run-attempt ticket authority mechanism is correctly architected:

1. **Ticket structure:** `ActionTicket` carries both `run_attempt` and `attempt` fields, clearly distinguishing run-level from action-level retries
2. **Key computation:** `compute_action_idempotency_key` binds the key to `run_attempt`, preventing stale key reuse
3. **Validation layer:** `validate_ticket_attempt` in `crates/vb_runtime/src/shard/helpers.rs` performs run-attempt authority checks
4. **Journal integration:** Ticket-bearing events (`ActionScheduledTicket`, `ActionCompletedEnvelope`) preserve the full ticket for replay

## Key Implementation Details

### Run-attempt authority check
```rust
// crates/vb_runtime/src/shard/helpers.rs:131-142
fn validate_ticket_attempt(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()> {
    let expected = state.run_attempt.get();
    if ticket.run_attempt != expected {
        return Err(RuntimeError::StaleAttempt {
            incoming: ticket.run_attempt,
            current: expected,
        });
    }
    // ... per-action attempt checks
}
```

### Idempotency key computation
```rust
// crates/vb_core/src/action/ticket.rs:48-65
pub fn compute_action_idempotency_key(
    run: RunId, run_attempt: u16, seq: SeqNo, action: ActionId,
) -> u128 {
    // Includes run_attempt in key derivation
}
```

## Design Findings

### Finding 1: Asymmetric validation (by design)
- `complete_action_with_output` validates the ticket synchronously before enqueuing
- `fail_action` enqueues without preflight validation; validation happens asynchronously in the shard

This is intentional per the existing `durable_retry_run_attempt_authority.rs` test. The completion path requires sync validation because output is provided inline; the failure path defers validation to the shard command loop.

### Finding 2: Schema version unchanged
- `CURRENT_SCHEMA_VERSION` remains 1 despite the `run_attempt` field addition
- This is correct because the field is added to the `ActionTicket` struct, not to the journal record envelope
- Journal record envelope compatibility is handled separately via schema-one shared envelope compatibility

## No Code Defects Found

The implementation is clean, follows existing patterns, and correctly implements the contract. No production code changes are required.
