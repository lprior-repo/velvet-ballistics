# Hazard Analysis — vb-hn4sc

bead_id: vb-hn4sc
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:35:00Z
authoring_agent: rust-contract

This artifact enumerates every hazard class relevant to the byte-budget gate on `JournalWriterQueue::flush_batch`. Each hazard names the risk, its trigger, the consequence, the mitigation, and the proof/test surface that proves the mitigation holds.

## 1. Hazard Inventory

| # | Class | Risk | Trigger | Consequence (if unmitigated) | Mitigation | Proof/Test Surface |
|---|---|---|---|---|---|---|
| H-01 | **Persistence** | Partial prefix committed | Bug in gate ordering: `owned_batch.insert` happens before gate fires | Master §49 atomicity violated; a process crash mid-flush leaves a partial durable record set | Gate fires *after* `staged_keys_unique` and `durable_key_unique` checks, *before* `owned_batch.insert`. On rejection, `owned_batch.commit()` is NOT called. | Unit tests `flush_batch_rejects_oversize_no_commit`; Kani harness `ps010::check_gate_fires_before_commit`; parity test `flush_batch_at_capacity_partial_drain_then_oversize_rejected_no_commit` |
| H-02 | **Persistence** | `owned_batch.commit()` invoked on a violating batch | Off-by-one or wrong comparison in the gate (`>=` vs `>`) | Bytes written exceed the configured limit; downstream memory pressure / fsync time blowups | Gate uses `>` (strictly greater than), mirroring `JournalWriteBatch::append_event` line 98. Exact-fit case (`attempted == limit`) is accepted. | Unit test `flush_batch_accepts_exact_fit_byte_budget`; Kani harness `ps010::check_exact_fit_accepted` |
| H-03 | **Rust-core invariant** | `u64::wrapping_add` overflow | Bug: `accumulated_bytes + next_event_encoded_len` uses `+` instead of `checked_add` | Accumulator wraps; gate silently accepts oversize batches; potentially unsafe arithmetic | `checked_add` only; overflow returns `JournalBatchBytesExceeded { attempted: u64::MAX, limit }`. Same pattern as `JournalWriteBatch::append_event:89-96`. | Kani harness `ps010::check_checked_add_overflow_rejected`; unit test `flush_batch_rejects_on_overflow` (with synthetic `accumulated_bytes = u64::MAX - 1`) |
| H-04 | **Concurrency** | Race between `enqueue_*` and `flush_batch` byte accumulator | Bug: accumulator lives on `JournalWriterQueue` as a shared field, accessed without the lock | Data race on `accumulated_bytes` | The accumulator is a stack-local `u64` inside `flush_batch`, never on the shared state. Verified by `git grep accumulated_bytes` showing only stack-local uses. | Code review; Kani model of lock interaction |
| H-05 | **Concurrency** | Mutex poisoning mid-gate | A panic inside the gate poisons the mutex; subsequent `enqueue_*`/`flush_batch` return `WriteLockPoisoned` | Queue is permanently unusable | The gate is panic-free: only `checked_add` and explicit `Result` propagation. The existing `WriteLockPoisoned` variant handles the post-poison state. | Existing test `flush_batch_returns_error_on_lock_poison` (analog); behavior unchanged |
| H-06 | **Hostile input** | Adversarial event size `value.len() == 0` | Caller enqueues an empty event (semantically invalid but not rejected by `event.is_valid()`) | Empty events contribute 0 to accumulator; not a hazard per se but defensible | `EncodedRecordLength::new(0)` returns `Err(InvalidConfig)`. The gate rejects before adding to the accumulator. | Unit test `flush_batch_rejects_zero_byte_event` |
| H-07 | **Hostile input** | Adversarial event size `value.len() > u64::MAX` | Impossible on supported platforms; defense in depth | Wrapping cast | `usize::try_into::<u64>()` is mandatory; failure path is `InvalidConfig { field: "encoded_record_length", reason: "value length exceeds u64" }`. | Kani harness bounds the conversion with `usize` ≤ 64-bit and `value.len() <= MAX_ENCODED_RECORD_BYTES` |
| H-08 | **Performance** | Re-encoding each queued event every flush | Event bytes re-computed in `flush_batch` via `encode_record`; for a queue at capacity draining across many `drain_all` iterations, encoding cost = `O(capacity / batch_size * batch_size) = O(capacity)` per drain | Slow drain on hot path; potential latency regression | Accepted by design (Open Question 5, Option A). The cost is bounded by `capacity` per drain; `capacity` is at most `MAX_BATCH_COUNT * batch_size` and `encode_record` is fast for `MAX_PAYLOAD = 1 MiB`. | Existing `encode_record` micro-bench; new micro-bench `flush_batch_throughput_at_default_limit` |
| H-09 | **Performance** | Cache-unfriendly 1 MiB events | A single 1 MiB event consumes the entire `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT`; the gate accepts it (exact fit) and rejects the next event | Worst case is one big event per flush; flush latency dominated by encode + commit | Accepted. `MAX_ENCODED_RECORD_BYTES = 1_048_636` is exactly the max-size event; the default budget fits exactly one. Documented in `StorageLimits::DEFAULT` doc-comment. | Behavior test `flush_batch_accepts_single_max_size_event` |
| H-10 | **API / Migration** | `StorageLimits::DEFAULT` extension breaks existing struct-literal callers | Callers write `StorageLimits { max_journal_event_payload_bytes: ... }` without the new field | Compile error in unrelated crates | Codebase map audit: no struct-literal `StorageLimits { ... }` exists outside `StorageLimits::DEFAULT` itself. All callers use `StorageLimits::DEFAULT`. | `git grep -n "StorageLimits {"` returns only the `DEFAULT` definition |
| H-11 | **API / Migration** | `JournalWriterQueue::with_contracts(capacity, batch_size, limits)` callers ignore `limits` | Old callers passed a placeholder `_limits`; they did not intend it to be enforced | Behavior change: previously queued events with total size > 1 MiB would commit silently | New gate enforces the limit; callers expecting "unlimited" must pass `StorageLimits { max_journal_batch_bytes: u64::MAX, .. }`. Documented in `with_contracts` doc-comment. | Audit of all `with_contracts` callers (production + tests); only `RuntimeJournalConfig::shared_journal` and `vb_cli::ipc_serve` in production, both use `StorageLimits::DEFAULT` |
| H-12 | **API / Migration** | RuntimeError mis-classification | `vb_runtime::journal::chunk_002.rs:406` propagates `JournalError` via `RuntimeError::from(JournalError)`. If `From<JournalError> for RuntimeError` does not have a typed `BudgetExhausted` variant, the byte-budget rejection is logged as a generic write error | Operators see "storage write failed" instead of "byte budget exceeded"; remediation is harder | Out of contract scope (codebase-map Open Question 7). The contract asserts the typed `JournalError` is the wire signal regardless of how `RuntimeError` labels it. | Verified by `proof-to-implementation` reading `crates/vb_runtime/src/error/conversions.rs` |
| H-13 | **Error classification** | New variant introduced for the queued path | A future contributor adds `QueuedBatchBytesExceeded { ... }` to bypass the direct-path code | Diagnostic codes drift; parity test fails; two error variants for the same domain concept | The contract explicitly forbids any new variant. `journal_batch_accounting_tests.rs:48-51` comment will be corrected to reflect that `JournalWriteBatch` DOES enforce byte limits (it always did). | Parity test asserts both paths emit `JournalError::JournalBatchBytesExceeded`; contract `error-taxonomy.md §8` forbids parallel variants |
| H-14 | **Refinement** | Pure predicate and imperative implementation drift | The `gate_decision` predicate is updated but `flush_batch`'s inline gate is not (or vice versa) | Pure tests pass but production is buggy | `gate_decision` is the *only* implementation of the gate; `flush_batch` calls it. If `flush_batch` ever grows its own gate logic, the contract forbids this. | Code review enforces single-source-of-truth; Kani harness binds to the pure predicate |
| H-15 | **Release / API** | `StorageLimits::DEFAULT` `const` drift | A future change adjusts the default without updating `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` | Storage limits and journal write limits disagree; parity test fails | Compile-time `const _STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND` block at the bottom of `types.rs` (mirrors `_INDEX_STATUS_STATE_EXHAUSTIVE`); fails to compile on drift. | Compiler enforces; explicit const assertion in type-contracts.md §5 |
| H-16 | **Release / API** | `JournalError::JournalBatchBytesExceeded` shape drift | Adding/removing fields on the variant | Parity test fails; downstream `From<JournalError>` conversion may break | The variant is `#[non_exhaustive]` in spirit (no derive, but the contract asserts the shape `{ attempted: u64, limit: u64 }` is stable for this bead). | Doc-comment on the variant asserts stability; parity test locks the shape |
| H-17 | **Concurrency** | Cancellation safety (sync code) | `JournalWriterQueue::flush_batch` is sync; a panic in a caller interrupts the call mid-flight | Not a true cancellation hazard in sync code; the Mutex surfaces the next call as `WriteLockPoisoned` | No change. Holzman Rust forbids `panic!` inside the gate. | Existing test patterns |
| H-18 | **Performance** | Memory pressure from holding 1 MiB events in queue | A high-throughput caller enqueues many near-max-size events | Queue memory grows; the byte gate ensures the *flush* batch is bounded, but the *queue* is not | Out of contract scope; `JournalQueueCapacity` (event count, not bytes) is the queue's memory bound. Carried forward as a future enhancement. | Existing memory test patterns; new test `queue_at_byte_capacity_below_event_capacity` |
| H-19 | **Temporal** | Idempotency on retried flushes | A retry of `flush_batch` after a transient error re-encodes events already on disk | Existing `journal.events.contains_key(key)` check inside `stage_queued_event` deduplicates; the byte gate does not interfere | No change to dedup logic. | Existing `flush_batch_across_calls_handles_idempotent_retry` |
| H-20 | **Temporal** | Stale accumulator across flushes | If the accumulator were stored on `JournalWriterQueueState`, an aborted flush could leave stale state | The accumulator is a stack-local `u64`, reset to `0` at every `flush_batch` entry. No persistence across calls. | Code review; the contract explicitly forbids storing the accumulator on the queue struct. | Review artifact |
| H-21 | **Parser/codec** | `encode_record` panics or returns malformed bytes | Bug in `encode_record` produces an unexpected `value.len()` | Gate computes a wrong size; either over-rejects or under-rejects | `encode_record` is the canonical encoder; the contract reuses it. No new encoding logic introduced. | Existing `encode_record` tests |
| H-22 | **Storage** | Fjall `OwnedWriteBatch::insert` fails silently on a violation | Fjall-specific behavior the gate cannot prevent | Out of contract scope; Fjall is trusted. | Existing storage tests. |
| H-23 | **Refinement** | Kani harness models a fake gate (vacuum) | The harness is hardcoded with a single shape and proves nothing | Kani proves the gate only on the hardcoded shape; the gate's behavior on adversarial inputs is unverified | The Kani harness exercises `gate_decision` with symbolic `accumulated_bytes`, `next.0`, and `limit` via `kani::any()`. Implements `kani::Arbitrary` for the newtypes OR uses `kani::any()` directly with explicit bounds. | Kani harness review |
| H-24 | **Concurrency** | Lock contention regression | Adding encode-call under the lock (already there today) is the same cost; the gate adds only an `if attempted > limit` check per event | Negligible | None needed. | Micro-bench on hot path |
| H-25 | **Performance** | Default budget too small | `1 MiB` is the default; a caller expects more | Caller passes a custom `StorageLimits { max_journal_batch_bytes: <larger>, .. }`. Documented. | No change. | Doc test |

## 2. Hazard Class Coverage Summary

| Class | Coverage |
|---|---|
| Temporal | H-19, H-20 covered |
| Concurrency | H-04, H-05, H-17, H-24 covered |
| Unsafe / provenance | Not applicable (`#![forbid(unsafe_code)]` in crate) |
| Hostile input | H-06, H-07 covered |
| Parser/codec | H-21 covered |
| Storage | H-22 covered |
| Performance | H-08, H-09, H-18, H-25 covered |
| API / migration | H-10, H-11, H-15, H-16 covered |
| Persistence | H-01, H-02 covered |
| Rust-core invariant | H-03 covered |
| Error classification | H-12, H-13 covered |
| Refinement | H-14, H-23 covered |
| Release | H-15, H-16 covered |

## 3. Top-Risk Hazards (must be addressed by proof/test obligations)

1. **H-01 (atomicity)** — required: unit test + Kani + parity test.
2. **H-03 (overflow)** — required: Kani + unit test.
3. **H-13 (variant drift)** — required: parity test (locks the variant shape).
4. **H-10 (struct-literal compat)** — required: compile-time audit (already done; recorded as a code-review check).
5. **H-23 (vacuum Kani)** — required: Kani harness uses `kani::any()` with explicit bounds, never a hardcoded shape.

## 4. Out-of-Scope Hazards (carried forward)

- H-12 (RuntimeError classification) — `proof-to-implementation` will resolve.
- H-18 (queue memory bound) — separate bead if needed.
- Loom model — separate bead if needed.