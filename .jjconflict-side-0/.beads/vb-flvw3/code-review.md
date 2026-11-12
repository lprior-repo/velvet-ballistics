# Code Review: vb-flvw3

**Reviewer:** local-3090-llama/qwen3.8-27b-uncensored
**Scope:** `crates/vb_runtime/src/shard/tests/chunk_015.rs` — `shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion`

## Verdict: APPROVE

## Analysis

The test correctly exercises the ask_step validation guard in the shard's ask-answer handling path.

### Correctness
- Obtains a real pending AskTicket via `shard.lookup_pending_ask_ticket(run, step 2)`
- Mutates `ask_step` to 99 (nonexistent step) by constructing a new `AskTicket` with the mutated field (struct fields are `pub(crate)`, no API change)
- Submits the invalid ticket through the real queue path (`shard.enqueue` + `shard.tick`)
- Asserts the exact error: `RuntimeError::InvalidActionCompletion`

### Guard Validation Order
The handler's `validate_ask_answer_request` calls `validate_pending_ask_answer` which performs ticket lookup by `(run, ask_step)`. This fails for step 99 before any answer value processing. The different answer value types (Bool vs I64) between invalid and valid answers do not cause a false pass because step validation occurs first.

### State Isolation
- Validates `pending_timer_count()` remains 1 after invalid answer
- Validates `runs_completed` remains 0
- Uses `journal.snapshot()` (returns `Vec<RuntimeJournalEvent>`, logical events not raw bytes) to prove no journal mutation
- Completes with valid ticket: runs_completed=1, runs_failed=0, timer=0

### No Production API Changes
All APIs used (`lookup_pending_ask_ticket`, `enqueue`, `tick`, `pending_timer_count`, `counters`, `snapshot`) are pre-existing. No new casts, unsafe, or unchecked indexing.

## Evidence

**Specific test:**
```
Command: cargo test -p vb_runtime --lib shard::tests::shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion -- --exact
Exit code: 0
Result: 1 passed; 0 failed; 1937 filtered out
```

**Full shard suite (301 tests):**
```
Command: cargo test -p vb_runtime --lib shard::tests
Exit code: 0
Result: 301 passed; 0 failed; 0 ignored; 1637 filtered out
```

All evidence captured via immutable runner at `.beads/vb-flvw3/evidence/`.
