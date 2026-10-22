# Code Review — vb-4qog3 Independent Review of vb-hz9v8 B/C/D Fixes

REVIEWED SHA: `0d348f5ae0aba7a9cce32d4ad0b990e66da07d2b`
BASELINE: `423d8ae8b667d82b4d36e57d29d7ef2aede08bdf`
VERDICT: **APPROVED**

Independent code review of the frozen vb-hz9v8 B/C/D implementation. This review
inspects the actual source, traces the implementation against the original
contract from vb-tf5lr, and verifies that each fix correctly addresses its
finding without introducing regressions.

Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u` and is NOT
in scope for this review.

---

## Finding B — CLI `retry` Silent Durability Downgrade (HIGH) — APPROVED

**Contract.** `cmd_retry` must use a strict append barrier (`shared_strict`) so
that retry-phase evidence is persisted with a `SyncAll` barrier, never weakening
a previously-strict run's durability.

**Implementation.** `crates/velvet_ballistics/src/run_ops.rs:177`:
```rust
let shared = vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::new(journal));
```
Correctly constructs the retry runtime with `shared_strict` instead of the
previous `shared_journaled`.

**Verified Path.** Traced the durability chain:
- `shared_strict` → `StorageRuntimeJournal { profile: DurabilityProfile::Strict }`
- `append_storage_event` → `self.journal.append_strict(event)` (line 41, chunk_002.rs)
- `JournalWriteBatch::strict().commit()` → `fjall::PersistMode::SyncAll` (commit.rs:8)

**Comment Quality.** Lines 172-176 in `run_ops.rs` correctly document the
rationale: the original per-run append mode is not recorded, so strict is the
fail-safe default that never weakens and only upgrades journaled retry phases.

**No Regressions.** The admission policy (`RuntimePolicy::Strict`) is unchanged
for both `--durability strict` and `--durability journaled` runs. Only the
durability barrier is affected.

---

## Finding C — Automatic Cold Recovery Silently Resumes Failed Run (CRITICAL) — APPROVED

**Contract.** The automatic recovery classifier must treat a run as `CannotResume`
when its latest terminal evidence belongs to its current attempt, where "current
attempt" excludes `RetryCheckpoint` events (which name the authorized next attempt,
not a started one).

**Implementation.** `crates/vb_runtime/src/recovery/full.rs:125-141`:
```rust
pub(crate) fn run_is_currently_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    let mut current_attempt = 1u16;
    for event in events {
        if matches!(event, vb_storage::JournalEvent::RetryCheckpoint { .. }) {
            continue;  // Skip RetryCheckpoint — names next, not current
        }
        if let Some(attempt) = event.attempt() && attempt > current_attempt {
            current_attempt = attempt;
        }
    }
    events.iter().rev().any(|event| {
        vb_storage::recovery::is_terminal_event(event)
            && event.attempt().unwrap_or(1) == current_attempt
    })
}
```

**Correctness Analysis.**
- Correctly excludes `RetryCheckpoint` from current-attempt computation (line 128)
- Correctly computes current attempt as max over non-checkpoint events (lines 131-134)
- Correctly scans for terminal event at current attempt (lines 137-140)
- Handles the case where a run failed at N, has a checkpoint for N+1, but nothing
  beyond that — it correctly identifies this as terminal (not resumable)

**Usage.** `crates/vb_runtime/src/runtime.rs:873`:
```rust
if crate::recovery::run_is_currently_terminal(&context.prefix.events) {
    return Ok(RuntimeRecoveryProduct::cannot_resume_terminal(
        context.seed.summary()
    ));
}
```
Correctly returns `CannotResume` before building a resumable product.

**No Over-Rejection.** The implementation correctly preserves the case where a
run was explicitly retried and is genuinely active (pending action or Ask
receipt-reconciliation): those have `StepStarted(attempt=N+1)` events that set
`current_attempt` to N+1, making the old `RunFailedEvent(attempt=N)` stale.

---

## Finding D — Raw `recover_retry` Trusts Caller-Supplied Authority (MEDIUM) — APPROVED

**Contract.** Before enqueueing `ShardCommand::Recover`, `Runtime::recover_retry`
must bind the supplied checkpoint to the durable chain, verifying:
1. Durable checkpoint exists for the run
2. Blake3 payload digest matches
3. Decoded durable checkpoint equals supplied checkpoint
4. Run identity matches
5. `checkpoint.prior_seq + 1 == durable.seq`
6. `durable.seq + 1 == next_seq`
7. Checkpoint is at journal head (not already consumed)
8. Preceding event is `RunFailedEvent` at `checkpoint.run_attempt - 1`

**Implementation.** `crates/vb_runtime/src/runtime.rs:453-593` —
`validate_retry_checkpoint_against_durable_chain`. All 8 checks are implemented:

- Check 1: Lines 466-486 — finds latest `RetryCheckpoint` for run, fails closed if absent
- Check 2: Lines 489-494 — computes observed digest, compares to durable digest
- Check 3: Lines 497-506 — decodes durable payload, requires equality with supplied checkpoint
- Check 4: Lines 510-514 — verifies `durable_run == run`
- Check 5: Lines 522-535 — verifies `prior_seq + 1 == durable.seq` with checked arithmetic
- Check 6: Lines 537-549 — verifies `durable.seq + 1 == next_seq` with checked arithmetic
- Check 7: Lines 555-563 — verifies checkpoint is journal head (`events.last().seq == durable.seq`)
- Check 8: Lines 567-591 — verifies preceding event is `RunFailedEvent` at `run_attempt - 1`

**Correctness Analysis.**
- All arithmetic uses `checked_add`/`checked_sub` with fail-closed errors (overflow/underflow)
- No state or journal mutation occurs on validation failure (validation happens before enqueue)
- The journal head check (check 7) correctly detects already-consumed checkpoints, which
  a stale `next_seq` alone could not detect
- The value equality check (check 3) is a strong defense-in-depth that catches any tampering
  of the checkpoint contents, even if individual fields pass their specific checks

**Invocation.** `crates/vb_runtime/src/runtime.rs:436`:
```rust
self.validate_retry_checkpoint_against_durable_chain(run, &checkpoint, next_seq)?;
```
Called before `RecoverRunCommand::for_retry` and shard enqueue. Correct placement.

---

## Cross-Cutting Quality

**No Unsafe/Unwrap/Expect/Panic.** Verified throughout all changed files. Error
handling uses proper `RuntimeError::Recovery { error }` construction.

**Documentation.** All new functions have clear doc comments explaining purpose,
rationale, and edge cases. The `run_is_currently_terminal` function (line 115-123)
has an especially thorough explanation of the RetryCheckpoint attempt semantics.

**Testing.** Independent adversarial tests confirm all rejection paths and the
valid happy path. No production code changes by reviewer.

**Verus/Flux/Kani Impact.** No Verus spec changes required — the fix does not alter
the Verus-bound `vb_storage::RecoveryCannotResumeState` type or its drift-gated
mirrors. The `terminal_state` reason is carried by a private reason field on the
`vb_runtime` `NonResumableRecoveryProduct`.

---

## Conclusion

All three findings (B, C, D) are correctly and completely fixed. The implementation
matches the contract from vb-hz9v8/rust-contract.md, which in turn correctly
addresses the original findings from vb-tf5lr. No regressions introduced.

**VERDICT: APPROVED**