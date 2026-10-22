# Test Review: vb-flvw3

**Reviewer:** local-3090-llama/qwen3.8-27b-uncensored
**Scope:** `crates/vb_runtime/src/shard/tests/chunk_015.rs` — `shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion`

## Verdict: APPROVE

## Test Quality

### Prior vs. New Coverage
The prior lookup-only test was a "fake guard" — it verified ticket lookup but not the actual negative path through the shard's command handler. This test restores real negative guard coverage by:
1. Getting a genuine pending AskTicket
2. Mutating it to an invalid state (wrong ask_step)
3. Submitting through the actual queue/tick cycle
4. Observing the exact expected error

### Coverage
- Invalid ticket rejection (ask_step mismatch)
- State preservation after rejection (timer count, completion counters)
- Journal immutability after rejection
- Valid completion path with original ticket
- Correct final counters (runs_completed=1, runs_failed=0, timer=0)

### Assertions
All assertions check observable state:
- Error type exact match (`RuntimeError::InvalidActionCompletion`)
- Numeric counters (pending_timer_count, runs_completed, runs_failed)
- Journal snapshot equality (logical event vectors)

No tautologies, no "doesn't throw" only checks, no length-grew assertions.

### Potential False Pass
The test uses different answer value types (Bool for invalid, I64 for valid). This could mask a different failure mode if the handler validated answer type before ask_step. However, inspection of `validate_ask_answer_request` confirms `validate_pending_ask_answer` (ticket lookup) runs first, making this safe. The guard being tested is the ask_step validation, not value type validation.

## Evidence

**Test execution:**
```
Command: cargo test -p vb_runtime --lib shard::tests::shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion -- --exact
Exit code: 0
Result: 1 passed; 0 failed
```

**Suite regression check:**
```
Command: cargo test -p vb_runtime --lib shard::tests
Exit code: 0
Result: 301 passed; 0 failed
```

Evidence captured via immutable runner at `.beads/vb-flvw3/evidence/`.
