# Boundary Map — vb-hn4sc

bead_id: vb-hn4sc
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:34:00Z
authoring_agent: rust-contract

This artifact locates the new byte-budget gate on the functional-core / imperative-shell map, lists every boundary it crosses, and forbids I/O / time / randomness in the pure decision.

## 1. Functional Core vs Imperative Shell

### 1.1 Pure Core (no I/O, no time, no Fjall, no Mutex)

| Module | Type | Why pure |
|---|---|---|
| `crates/vb_storage/src/queue/writer_contract.rs` | `gate_decision` predicate (NEW; see type-contracts.md §4) | inputs: `AccumulatedFlushBytes`, `EncodedRecordLength`, `JournalBatchByteBudget`; outputs: `GateDecision` |
| `crates/vb_storage/src/types.rs` | `EncodedRecordLength`, `AccumulatedFlushBytes` newtypes | pure value-object constructors |
| `crates/vb_storage/src/batch/types.rs` | `JournalWriteBatch` byte-budget math (existing, parity target) | already pure; the queued gate mirrors this exactly |

### 1.2 Imperative Shell (holds state, locks, Fjall)

| Module | Type | Side effects |
|---|---|---|
| `crates/vb_storage/src/queue/writer.rs` | `JournalWriterQueue::flush_batch` | mutex lock; `OwnedWriteBatch` allocation; `commit`; `pop_front` drain |
| `crates/vb_storage/src/queue/writer/stage.rs` | `stage_queued_event` | Fjall `contains_key`; `encode_record`; `OwnedWriteBatch.insert`; index op |
| `crates/vb_storage/src/batch/append_event.rs` | `JournalWriteBatch::append_event` | Fjall `contains_key`; `OwnedWriteBatch.insert`; index op |

The byte-budget gate is implemented in the imperative shell (`flush_batch`) but delegates its *decision* to the pure `gate_decision` predicate in the core. This separation lets unit tests and proptest exercise the pure predicate without any Fjall harness, and lets a future Verus spec bind to the pure predicate without modeling the Fjall/Mutex state.

## 2. Boundary Crossing Diagram

```
                     +-----------------------+
                     |   Pure Core (Rust)    |
                     |   - gate_decision     |
                     |   - EncodedRecordLength
                     |   - AccumulatedFlushBytes
                     +-----------------------+
                                ^
                                |  (in-process function call; no IO)
                                |
+---------------------+    +-----+-----------------+    +----------------------+
| Rust async shell    |    | Imperative Shell       |    | Fjall storage shell  |
| (none for this bead)|    | - Mutex<state>         |    | - OwnedWriteBatch    |
|                     |    | - flush_batch loop     |    | - database.batch()   |
|                     |    | - stage_queued_event   |    | - contains_key       |
|                     |    | - encode_record        |    | - commit             |
|                     |    | - byte_gate_decide     |    | - PersistMode::SyncAll|
+---------------------+    +-----+-----------------+    +----------------------+
                                ^                              ^
                                |                              |
                          +-----+------+                +-------+-------+
                          | Lock guard |                | Storage IO   |
                          | (std::sync)|                | (fsync via   |
                          |  Mutex)    |                |  SyncAll)    |
                          +------------+                +--------------+
```

## 3. Boundaries Crossed by the Gate

### 3.1 Encoding boundary (parser)

- **Boundary:** `JournalEvent` (semantic) → `Vec<u8>` (encoded record) via `encode_record(MAGIC_JOURNAL_EVENT, kind, seq, event, MAX_PAYLOAD)`.
- **Where:** `crates/vb_storage/src/codec.rs::encode_record`, called from `stage_queued_event` at `writer/stage.rs:61-67`.
- **Gate interaction:** reads `value.len() -> usize`, then `usize::try_into::<u64>()` to obtain the per-event byte size for the gate. No new parsing introduced.

### 3.2 Lock boundary (concurrency)

- **Boundary:** `Mutex<JournalWriterQueueState>` lock acquisition at `flush_batch:156-159`.
- **Gate interaction:** the gate runs entirely under the lock; no separate lock is taken for the accumulator. The accumulator is a stack-local `u64`, not a queue field.
- **Holzman Rust rule:** no nested locks, no async under lock, no long-running work under lock. `encode_record` is pure and fast enough to run under lock.

### 3.3 Storage boundary (Fjall I/O)

- **Boundary:** `journal.database.batch()` at `writer.rs:194` → `OwnedWriteBatch` → `owned_batch.commit()` at `writer.rs:213`.
- **Gate interaction:** the gate fires BEFORE `commit()`. On rejection, `commit()` is NOT called. The `OwnedWriteBatch` is dropped (not aborted in the Fjall `aborted` sense — that flag is for `JournalWriteBatch` only) and its memory is reclaimed.

### 3.4 Time / clock boundary

- **Boundary:** none. The gate uses no time / clock / deadline.

### 3.5 Network boundary

- **Boundary:** none. The gate does not touch the network.

### 3.6 FFI / unsafe boundary

- **Boundary:** none. The gate is pure safe Rust. The crate `#![forbid(unsafe_code)]` declaration at `error/mod.rs:1` and similar in `types.rs:1` already forbids `unsafe` everywhere in the storage crate.

### 3.7 Randomness boundary

- **Boundary:** none. The gate uses no RNG.

## 4. Persistence Boundary (Crash Consistency)

The byte gate has a single, narrow persistence boundary: `owned_batch.commit()`. Master §49 mandates:

- A `flush_batch` either commits all-or-nothing.
- A process crash mid-flush leaves no durable-visible partial prefix.

The gate upholds this:

- **Gate fires before commit.** Rejection skips `commit()` entirely.
- **Gate fires after duplicate-key dedup.** No key is `insert`-ed into `OwnedWriteBatch` before the gate runs; the `staged_event_keys` HashSet is updated *after* `insert`, so a rejection leaves the HashSet unchanged as well.
- **No half-committed batches.** If the gate fires, `owned_batch` is dropped without commit. If the gate passes but `commit()` fails, the entire batch rolls back atomically (Fjall semantics).

## 5. Concurrency Boundary

### 5.1 Lock discipline

- `state: Mutex<JournalWriterQueueState>` is the single concurrency primitive.
- `enqueue_*`, `pending_profile_counts`, `probe_accepting_writes`, `flush_batch`, and `drain_all` all acquire the same lock.
- The byte accumulator is a stack-local `u64` inside `flush_batch`; no shared mutable state introduced.

### 5.2 Send/Sync

- `JournalWriterQueue` is `Send + Sync` today (because `Mutex<T>` is `Send + Sync` when `T: Send` and `VecDeque<QueuedJournalEvent>: Send`). Adding `byte_budget: u64` (a `Copy` type) preserves `Send + Sync` automatically.
- `EncodedRecordLength` and `AccumulatedFlushBytes` are `Copy` and therefore `Send + Sync` if they leak outside the module.

### 5.3 Loom

- A Loom model for the byte accumulator is OUT OF SCOPE for this bead (per codebase-map Open Question 8). The accumulator is a stack-local `u64` updated only under the existing lock; the Loom risk surface is identical to the existing `enqueue` + `flush_batch` interleaving, which is already exercised by tests.

## 6. Persistence + Replay Boundary

- **Replay:** `replay_journal` does not invoke the writer queue; the queue is for *forward* writes only.
- **Recovery:** `recovery_integration.rs:466-531` constructs the queue with `StorageLimits::DEFAULT` and drains. With the new field, default byte budget is `1 MiB`, which already accommodates all current test events (per codebase-map item §8 "No change needed if default byte budget ≥ current event sizes").
- **Module wiring (`mod stage;`):** Verified by the parent and isolated workspace layouts being identical. CI smoke (`cargo check -p vb_storage`) re-verifies after the change.

## 7. Public API Surface (boundary to callers)

| API | Module | Caller-facing change | Source-compat strategy |
|---|---|---|---|
| `JournalWriterQueue::new(capacity, batch_size, limits)` | `queue/writer.rs:40` | unchanged signature; behavior change: `_limits` is now wired into `byte_budget` and enforced | signature preserved |
| `JournalWriterQueue::with_contracts(capacity, batch_size, limits)` | `queue/writer.rs:51` | unchanged signature; `_limits` now used | signature preserved |
| `StorageLimits::DEFAULT` | `types.rs:17` | extended struct; `DEFAULT` constant populates the new field | source-compat: callers using `StorageLimits::DEFAULT` keep their previous behavior; explicit-struct-literal callers must add the field (none in the codebase today) |
| `StorageLimits { max_journal_event_payload_bytes, max_journal_batch_bytes }` | `types.rs:10` | new field | non-exhaustive-breaking: callers using `..Default::default()` continue to work |
| `JournalError::JournalBatchBytesExceeded` | `error/mod.rs:40` | unchanged | parity-preserved |
| `JournalWriterFlushReport` | `types.rs:166` | unchanged | no Ok-side change |

## 8. Test Boundary

- **Pure predicate tests** (no Fjall, no Mutex): exercise `gate_decision` directly. Suitable for unit tests and proptest.
- **Imperative tests** (with `temp_journal`): exercise `JournalWriterQueue::flush_batch` end-to-end. Suitable for the existing `tests.rs` test helpers.
- **Kani**: exercise the pure predicate's overflow and limit-exceeded branches with bounded symbolic inputs. New harness `kani_vb_vzcuf_ps010.rs`.
- **Mirror parity test** (required): one test that constructs both `JournalWriteBatch` and `JournalWriterQueue`, feeds the same `(event, limit)` to both, and asserts the returned errors match on `(variant, attempted, limit, code, symbol)`.

## 9. Forbidden Boundary Crossings

- **No async/await inside `flush_batch`.** The current function is sync; introducing async would force a Tokio runtime dependency in the storage crate.
- **No `Instant::now()` / `SystemTime::now()` inside the gate.** Time is not part of byte accounting.
- **No `rand` / `getrandom` inside the gate.** Byte accounting is deterministic.
- **No `unsafe` anywhere in the gate.** Forbids `transmute`, raw pointer arithmetic, etc.
- **No panicking math (`unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `dbg!`).** The gate uses `checked_add` and explicit `Result` propagation.
- **No Fjall handle passed into the pure predicate.** `gate_decision` is `(&AccumulatedFlushBytes, EncodedRecordLength, u64) -> GateDecision`; Fjall is not in its signature.