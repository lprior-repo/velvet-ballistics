# Workflow Model — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Scanner Workflow (Legal States and Transitions)

The trim scanners in `crates/vb_storage/src/trimming/logic.rs` follow a
strict read-validate-interpret state machine. The contract pins the legal
states and the transitions between them.

### State Machine — Per Raw Key Observation

```
                +-----------------+
                |  InitialState   |  (scanner set up, prefix cursor positioned)
                +-----------------+
                        |
                        v
                +-----------------+
                | ObservingKey    |  (item.key() succeeded; len check pending)
                +-----------------+
                        |
            +-----------+-----------+
            |                       |
            v                       v
   key.len() ==             key.len() !=
   MAX_*_KEY_LEN            MAX_*_KEY_LEN
            |                       |
            v                       v
   +-----------------+     +-----------------------+
   |  DecodeKey      |     | AbortAndError         |
   +-----------------+     +-----------------------+
            |                       |
            v                       v
   decode OK ->                 return
   accumulate /                 Err(TrimError::
   continue loop                IncompleteTrim { .. })
            |
            v
   decode Err -> AbortAndError
   (prefix collision,
    reserved seq sentinel,
    invalid run id)
```

### States

| State | Description | Terminal? |
|-------|-------------|-----------|
| `InitialState` | Scanner opened; `prefix_key` materialised; LSM cursor positioned. | No |
| `ObservingKey` | Iterator returned a `(key, _)`; `key.len()` not yet checked. | No |
| `DecodeKey` | Length verified; `decode_storage_key` invoked (snapshot path only). | No |
| `Accumulate` | Key is canonical; value-side work proceeds (decode event payload for trim helpers, parse seq for trim loop, etc.). | No |
| `AbortAndError` | Non-canonical key OR decode failure. Returns `Err` to caller. | **Yes** |

### Guards

| Guard | Predicate | On failure |
|-------|-----------|------------|
| `G-LengthEqCap` | `key.len() == MAX_TRIM_KEY_LEN` (or `MAX_SNAPSHOT_KEY_LEN`) | Abort with `TrimError::IncompleteTrim { deleted_count }` |
| `G-PrefixMatch` | `decode_storage_key(&key)` returns the expected variant (`RunSnapshot` for snapshot path) | Abort with `TrimError::IncompleteTrim { deleted_count: 0 }` |
| `G-RunIdNonZero` | `key.run.get() != 0` | Caught by `KeyDecodeError::InvalidRunId`; abort with `IncompleteTrim { deleted_count: 0 }` |
| `G-SeqNotSentinel` | `key.seq.get() != u64::MAX` | Caught by `KeyDecodeError::ReservedSeqSentinel`; abort with `IncompleteTrim { deleted_count: 0 }` |
| `G-SliceBounds` | `key.get(9..17).is_some()` (only meaningful when `G-LengthEqCap` already passed) | Defensive — should be unreachable; abort with `IncompleteTrim { deleted_count: 0 }` |

### Transitions

| # | From | Guard | To | Side effect |
|---|------|-------|-----|-------------|
| T1 | `InitialState` | iterator yields first key | `ObservingKey` | none |
| T2 | `ObservingKey` | `G-LengthEqCap` | `DecodeKey` (snapshot path) or `Accumulate` (event path) | none |
| T3 | `ObservingKey` | `!G-LengthEqCap` | `AbortAndError` | none |
| T4 | `DecodeKey` | `G-PrefixMatch` ∧ `G-RunIdNonZero` ∧ `G-SeqNotSentinel` | `Accumulate` | none (or `Ok(Some(seq))` for `latest_durable_snapshot_seq`) |
| T5 | `DecodeKey` | `!G-PrefixMatch` ∨ `!G-RunIdNonZero` ∨ `!G-SeqNotSentinel` | `AbortAndError` | none |
| T6 | `Accumulate` | loop body completes | back to T1 if iterator has more keys; else terminal `Ok` | `batch.remove(...)` + `deleted_count.saturating_add(1)` for trim path; `count.saturating_add(1)` for count path |
| T7 | any | Fjall backend error (`item.key()`, `batch.commit()`) | `AbortAndError` | none |

### Terminal Outcomes

| Outcome | Return type | Caller-facing meaning |
|---------|-------------|-----------------------|
| `Ok(Some(seq))` | `TrimResult<Option<EventSeq>>` | Snapshot found at `seq`; trim can proceed. |
| `Ok(None)` | `TrimResult<Option<EventSeq>>` | No snapshot exists for the run. |
| `Ok(TrimmedRunResult { Trimmed, .. })` | `TrimResult<TrimmedRunResult>` | Events were deleted; LSM batch committed. |
| `Ok(TrimmedRunResult { NoOp, .. })` | `TrimResult<TrimmedRunResult>` | No events eligible under cutoff. |
| `Ok(count)` | `Result<u64, JournalError>` | Count returned to `trim_eligibility_diagnostic`. |
| `Err(TrimError::IncompleteTrim { deleted_count })` | `TrimError` | Fail-closed abort; batch NOT committed. |

---

## Workflow Invariants

### WF-INV-1 — Abort-on-First-Bad-Key

The scanner must abort the moment it observes any non-canonical key. The
`break` / `return` semantics in `trimming/logic.rs:36-38`, `:77-79`, and
`:222-226` enforce this. **No skip-and-continue branch is permitted.**

### WF-INV-2 — No Mutation on Err

`trim_events_for_run` performs `batch.commit()` ONLY in the success path
(`trimming/logic.rs:105`). The `Err` path returns early, leaving the LSM
batch in the pending state, which is dropped without commit. `F-fjall
contract:` Fjall's `WriteBatch` is automatically discarded when dropped
without commit.

### WF-INV-3 — Counter Preserved on Abort

`deleted_count` (or `count`) is a `u64` incremented via
`saturating_add(1)`. The abort returns the partial counter so callers know
how far the scan progressed. This is the value surfaced as
`TrimError::IncompleteTrim { deleted_count }`.

### WF-INV-4 — Diagnostic Code Stable Across Chain

```
TrimError::IncompleteTrim { .. }           -> diagnostic_code() = 0x4102
JournalError::Trim(Box::TrimError::Incompl) -> diagnostic_code() = 0x4102 (via codes.rs:167)
```

The chain `trim_eligibility_diagnostic -> count_trimmable_events -> Err(JournalError::Trim(...))`
preserves `0x4102`. Test `error_code_tests.rs:~244` verifies this.

---

## Idempotence

- `latest_durable_snapshot_seq` is **idempotent**: read-only; safe to call
  repeatedly; result depends only on the Fjall state.
- `trim_events_for_run` is **idempotent** under successful commit: a second
  call sees no eligible events below the same cutoff and returns `NoOp`.
  Under abort, the LSM batch is dropped, so a second call re-scans and
  re-aborts (still safe; the fail-closed property is preserved across
  retries).
- `count_trimmable_events` is **idempotent**: read-only.

## Cancellation / Shutdown

- No async surface; scanners are synchronous.
- Cancellation is implicit: the caller can drop the `FjallJournal` at any
  point. The next operation will observe a closed handle and surface
  `TrimError::Fjall(_)` or `JournalError::Fjall(_)`.
- No background task or worker is owned by the scanner; no shutdown path
  is in scope.

## Retry Semantics

- All three scanners are safe to retry. The fail-closed property means
  each retry hits the same abort if the underlying row remains malformed.
- The planner must not introduce retry logic inside the scanner (would
  mask the abort).

---

## Hazard Cross-Reference

| Hazard class | Workflow impact | Where |
|--------------|-----------------|-------|
| Temporal (recovery from wrong snapshot) | High — overlong key masquerading as durable snapshot would cause wrong-event trimming. The contract fixes this. | `trimming/logic.rs:36-46` |
| Parser/codec | The length check is the parser boundary; without it, `decode_storage_key` would also reject (KeyLengthMismatch) but the wrong error would surface. | `trimming/logic.rs:36, 77, 222` |
| Persistence | Fjall raw-key read at `item.key()`; cap enforced at scanner level. | `trimming/logic.rs:75, 218` |
| Public API | Error variant + diagnostic code stability. | `trimming/mod.rs:51-54`, `error/codes.rs:62` |
| Numeric/cap refinement | All three constant-equalities are compile-time. | `constants.rs:74` + new aliases |
| Concurrency | None — synchronous, single-threaded scan over a snapshot. | n/a |
| Unsafe / provenance | None — `#![forbid(unsafe_code)]`. | n/a |
| Performance | Negligible — `key.len()` is `O(1)` for `fjall::UserKey`. | constant-time check |
| Release / API | The aliases change from `pub(crate)` literals to `pub(crate)` named caps; no public API breakage. | `constants.rs:74-79` |

For the full hazard analysis, see `hazard-analysis.md`.

---

## Forbidden Workflow Paths

| Forbidden path | Why |
|----------------|-----|
| Truncate a long key to 17 bytes and proceed | Would silently lose information about a corrupted row and produce wrong trim results. |
| Pad a short key to 17 bytes and proceed | Would fabricate data; even more dangerous than truncate. |
| Skip the malformed row and continue | Would silently drop the row and break the fail-closed invariant. |
| Log a warning and proceed | Holzmann-Rust forbids `warnings.rs`-style "log and continue" on parser boundary errors; this is an explicit typed error. |
| Spawn a worker to clean up the malformed row | Out of scope; the scanner surfaces the error and the caller decides. |
| Retry the decoder with a different cap | Would mask the bug; the cap is invariant. |

---

## Workflow Diagram (Trim Path)

```
  caller
    |
    v
  trim_eligibility_diagnostic(policy)
    |
    +---> run_headers()
    |
    +---> for header in headers:
    |       |
    |       +---> latest_durable_snapshot_seq(run)      <-- G-LengthEqCap (MAX_SNAPSHOT_KEY_LEN)
    |       |       |
    |       |       +-> Ok(Some(seq))    -> safe_point = seq
    |       |       +-> Ok(None)         -> push Blocked(NoDurableSnapshot), continue
    |       |       +-> Err(IncompleteTrim) -> return Err(JournalError::from(TrimError))  <-- FAIL CLOSED
    |       |
    |       +---> check_retention_policy(run, policy)   <-- separate policy guard
    |       |
    |       +---> count_trimmable_events(run, safe_point)  <-- G-LengthEqCap (MAX_TRIM_KEY_LEN)
    |               |
    |               +-> Ok(count)    -> push Eligible { .. count .. }
    |               +-> Err(JournalError::Trim(IncompleteTrim)) -> return  <-- FAIL CLOSED
    |
    +---> Ok(TrimDiagnostic { .. })
```

The diagram shows the two fail-closed edges that this contract binds. Both
arise from the length check at the same three sites. The contract commits to
the **`MAX_*_KEY_LEN` named-cap replacement** at all three sites.

END OF WORKFLOW MODEL.