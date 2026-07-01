# Error Taxonomy — vb-hn4sc

bead_id: vb-hn4sc
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:33:00Z
authoring_agent: rust-contract

The byte-budget gate does **not** introduce a new error variant. It reuses the existing `JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }` and its existing diagnostic code `JOURNAL_BATCH_BYTES_EXCEEDED_CODE = 0x4022`. This artifact documents the relevant error variants, the railway they form, and the parity claim between the direct (`JournalWriteBatch::append_event`) and queued (`JournalWriterQueue::flush_batch`) paths.

## 1. Reused Variant

```text
#[error("journal batch byte budget exceeded: attempted {attempted} > limit {limit}")]
JournalBatchBytesExceeded { attempted: u64, limit: u64 },
```

- **Location:** `crates/vb_storage/src/error/mod.rs:40-41` (existing, unchanged).
- **Diagnostic code:** `JOURNAL_BATCH_BYTES_EXCEEDED_CODE = 0x4022` (existing, unchanged).
- **Symbolic code:** `JOURNAL_BATCH_BYTES_EXCEEDED` (existing, unchanged).
- **Display invariant:** for every successful emission, `attempted > limit`.
  - Direct-path overflow case: `attempted = u64::MAX`, which is `> limit` for any realistic `limit`.
  - Queued-path overflow case: same.
  - Both paths compute `attempted` identically: `accumulated_bytes.checked_add(next_event_encoded_len).unwrap_or(u64::MAX)`.

## 2. Other Variants Affected by the New Gate

None. The gate adds a new *transition* but reuses the existing error type. The following variants continue to fire from the same call sites with the same semantics:

| Variant | From | Unchanged by gate? |
|---|---|---|
| `WriteLockPoisoned` | `Mutex::lock()` | yes |
| `QueueShutdown` | `enqueue_*` (only) | yes |
| `QueueFull` | `enqueue_*` | yes |
| `QueueCapacity` | `JournalQueueCapacity::try_from_usize` / `JournalBatchSize::try_from_usize` / `drain_all` bound | yes |
| `DuplicateStagedKey { run, seq }` | `stage_queued_event` HashSet guard | yes; precedence before byte gate |
| `DuplicateEvent { run, seq }` | `stage_queued_event` durable lookup | yes; precedence before byte gate |
| `Encode(_)` / `PostcardEncodeFailed(_)` | `encode_record` | yes |
| `PayloadTooLarge { len, max }` | `encode_record` payload cap | yes |
| `InvalidEvent` | `event.is_valid()` | yes |
| `Fjall(_)` | `owned_batch.commit()` / contains_key | yes; not reached on gate rejection |
| `SequenceOverflow` | `u64::try_from(value.len())` inside the existing `JournalWriteBatch::append_event` | yes; not used by queued gate (queued gate's `value.len()` is `usize` ≤ `1_048_636`, so `u64::try_from` is infallible; Holzman Rust still requires it, so the queued gate does `usize::try_into` returning `Result<_, JournalError>` shaped by `InvalidConfig { field: "encoded_record_length" }` to match the contract — see §3) |

## 3. New "Invalid Config" Sub-case (defensive only, never reachable in production)

The queued gate performs `u64::try_from(encoded_value.len())`. The bound guarantees this is infallible (`encoded_value.len() <= 1_048_636`), but the Holzman Rust rule "no unchecked conversion" requires the conversion be defended. The chosen shape is:

```text
Err(JournalError::InvalidConfig {
    field: "encoded_record_length",
    reason: "value length exceeds u64",
})
```

- **Reachable:** only if `encoded_value.len() > u64::MAX`, which is impossible on every platform Rust supports (usize is at most 64-bit on supported targets, and the encoded value is bounded by `MAX_ENCODED_RECORD_BYTES = 1_048_636`).
- **Defended in depth:** matches the discipline of `JournalWriteBatch::append_event` line 88 (`u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?`) but uses a more honest field name (`encoded_record_length`).

## 4. Railway Diagram (per-flush, byte gate introduced)

```
                 lock_acquired
                       |
                       v
                +-------------+
                | PreScan     |
                +-------------+
                       |
                       v
                +-------------+
                | ScanProfile |--(empty pending)--> Ok(report{0,0})
                +-------------+
                       |
                       v
                +-------------+
   per event --->| StageEvents |
                +-------------+
                  |   |   |
     next fits    |   |   |  next oversize
     in budget    |   |   v
                  |   |  +---------------+
                  |   |  | GateReject    |--> Err(JournalBatchBytesExceeded { attempted, limit })
                  |   |  +---------------+
                  |   |
                  |   |  checked_add
                  |   |  overflow
                  |   |   |
                  |   |   v
                  |   |  +---------------+
                  |   |  | GateOverflow  |--> Err(JournalBatchBytesExceeded { attempted: u64::MAX, limit })
                  |   |  +---------------+
                  |   |
                  |   v
                  | +-----------+
                  | | GateAccept|
                  | +-----------+
                  |    |
                  |    v (loop)
              duplicate staged_key
                  |
                  v
        Err(DuplicateStagedKey { run, seq })

        duplicate durable key
                  |
                  v
        Err(DuplicateEvent { run, seq })

        encode failure
                  |
                  v
        Err(Encode(_))

        payload too large
                  |
                  v
        Err(PayloadTooLarge { len, max })

        event invalid
                  |
                  v
        Err(InvalidEvent)

        conversion impossible
                  |
                  v
        Err(InvalidConfig { field: "encoded_record_length", reason: "..." })

                (all per-event guards passed)
                       |
                       v
                +-------------+
                |   Commit    |--> Err(Fjall(_)) on commit failure
                +-------------+
                       |
                       v
                +-------------+
                | DrainPending|--> Err(WriteLockPoisoned) on drain inconsistency
                +-------------+
                       |
                       v
                Ok(report{drained, written})
```

## 5. Parity Claim

For a given `(accumulated_bytes, next_event_encoded_len, limit)` triple:

| Path | Function | Returns |
|---|---|---|
| Direct | `JournalWriteBatch::append_event` | `Err(JournalBatchBytesExceeded { attempted, limit })` with `attempted = accumulated_bytes.checked_add(next_event_encoded_len).unwrap_or(u64::MAX)` |
| Queued | `JournalWriterQueue::flush_batch` (NEW) | `Err(JournalBatchBytesExceeded { attempted, limit })` with `attempted = accumulated_bytes.checked_add(next_event_encoded_len).unwrap_or(u64::MAX)` |

- Both error variants: identical.
- Both diagnostic codes: identical (`0x4022`).
- Both symbolic codes: identical (`JOURNAL_BATCH_BYTES_EXCEEDED`).
- Both overflow shapes: identical (`u64::MAX`).
- Both accounting bases: identical (full encoded record length, not raw payload).

A parity test in `crates/vb_storage/src/queue/tests.rs` will construct a `(JournalWriteBatch, JournalWriterQueue)` pair against the same `(event, limit)` and assert the two errors match on `(variant, attempted, limit, code, symbol)`.

## 6. RuntimeError Classification (open downstream concern)

The contract does NOT add a new variant to `vb_runtime::error::RuntimeError`. Propagation continues via `RuntimeError::from(JournalError)`. Whether the existing `From<JournalError> for RuntimeError` exposes a typed `BudgetExhausted`-style variant for `JournalBatchBytesExceeded` is verified in the implementation phase; the contract asserts the wire signal is the typed `JournalError` regardless of how `RuntimeError` chooses to label it. (Carried from codebase-map Open Question 7.)

## 7. CLI / IPC Wire Errors

`vb_cli::ipc_serve` propagates `JournalError` through its CLI error path. The diagnostic code `0x4022` is what the CLI emits to its caller. Adding the gate does not change this; it only causes the CLI to emit `0x4022` in the new byte-budget rejection path.

## 8. Forbidden / Out-of-Scope Variants

- **No `QueuedBatchBytesExceeded`.** Forbids any parallel variant.
- **No `ByteBudgetOverflow { ... }` separate from `JournalBatchBytesExceeded`.** Overflow rolls into the same variant.
- **No generic `InternalError(String)` for byte-budget rejection.** Always the typed variant.
- **No new diagnostic code.** `0x4022` is the only code the gate emits.