# Contract — vb-hn4sc

bead_id: vb-hn4sc
bead_title: Storage: enforce byte-budget limits in queued group commits (P1 bug)
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:36:00Z
authoring_agent: rust-contract

This is the canonical contract artifact. It states the requirements, the type-level obligations, the workflow obligations, the error obligations, the boundary obligations, and the hazard mitigations for the byte-budget gate on `JournalWriterQueue::flush_batch`. Downstream agents (proof-planner, test-writer, holzman-rust) lock onto this artifact as the source of truth.

---

## 1. Requirement

**R-HN4SC-1 (byte-budget gate at group commit).** `JournalWriterQueue::flush_batch` must enforce a per-flush encoded-byte budget before any `owned_batch.commit()` call, returning `Err(JournalError::JournalBatchBytesExceeded { attempted, limit })` when the next staged event would push the accumulated bytes above the configured limit.

**Source:** bead statement; codebase-map §"Scope Summary"; delivery-scope.jsonl contract clause `GROUP-COMMIT-BYTE-GATE-1`.

**Acceptance criteria:**

- AC-1.1: `flush_batch` returns `Err(JournalBatchBytesExceeded { attempted, limit })` for an oversize byte batch.
- AC-1.2: `flush_batch` returns `Ok(JournalWriterFlushReport)` for a byte batch within the limit.
- AC-1.3: `JournalWriteBatch::append_event` and `JournalWriterQueue::flush_batch` emit the same error variant and same `(attempted, limit)` shape for the same oversize event.
- AC-1.4: `StorageLimits::DEFAULT.max_journal_batch_bytes == DEFAULT_JOURNAL_BATCH_BYTE_LIMIT == 1_048_576`.
- AC-1.5: The `_limits: StorageLimits` parameter at `writer.rs:54` is no longer ignored; the byte budget is wired into the gate.
- AC-1.6: No new error variant, no new diagnostic code.

---

## 2. Type-Level Obligations

| ID | Obligation |
|---|---|
| **T-HN4SC-1** | `StorageLimits` MUST be extended with `pub max_journal_batch_bytes: u64` defaulting to `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (1_048_576). |
| **T-HN4SC-2** | `JournalWriterQueue` MUST carry an immutable `byte_budget: u64` field set from `StorageLimits::max_journal_batch_bytes` at construction. |
| **T-HN4SC-3** | `EncodedRecordLength(u64)` MUST be introduced as a smart-constructor newtype with the invariant `0 < value <= MAX_ENCODED_RECORD_BYTES`. |
| **T-HN4SC-4** | `AccumulatedFlushBytes(u64)` MUST be introduced with a `checked_add` `add` operation and a `would_exceed` predicate. |
| **T-HN4SC-5** | `gate_decision` (pure) MUST be implemented as the single decision function used by `flush_batch`; its signature is `(&AccumulatedFlushBytes, EncodedRecordLength, u64) -> GateDecision`. |
| **T-HN4SC-6** | `GateDecision` MUST be an enum with `Accept { new_accumulated: AccumulatedFlushBytes }` and `Reject { attempted: u64, limit: u64 }` variants. |
| **T-HN4SC-7** | A compile-time const assertion MUST bind `StorageLimits::DEFAULT.max_journal_batch_bytes == crate::batch::types::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` and `== vb_core::max_journal_batch_bytes()`. |
| **T-HN4SC-8** | `JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }` MUST NOT be modified (parity target). |
| **T-HN4SC-9** | All new types MUST derive `Debug, Clone, Copy, PartialEq, Eq` where applicable and MUST NOT introduce `Option<u64>`, boolean flags, or stringly-typed error shapes. |
| **T-HN4SC-10** | No `unsafe`, no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!` in the gate or its newtypes. |

---

## 3. Workflow Obligations

| ID | Obligation |
|---|---|
| **W-HN4SC-1** | The byte gate MUST fire AFTER `staged_keys_unique` and `durable_key_unique` checks and BEFORE `owned_batch.insert`. |
| **W-HN4SC-2** | The byte gate MUST fire BEFORE `owned_batch.commit()` so that a violating batch is never partially committed. |
| **W-HN4SC-3** | On gate rejection, the queue MUST retain the rejected event plus all not-yet-staged events; `pending.len()` MUST be unchanged. |
| **W-HN4SC-4** | On overflow (`checked_add` returns `None`), the gate MUST return `JournalBatchBytesExceeded { attempted: u64::MAX, limit }`. |
| **W-HN4SC-5** | The byte accumulator MUST be a stack-local `u64` reset to `0` at every `flush_batch` entry; it MUST NOT be a field on `JournalWriterQueue` or `JournalWriterQueueState`. |
| **W-HN4SC-6** | `drain_all` MUST short-circuit on the first `JournalBatchBytesExceeded` error (no further `flush_batch` iterations). |
| **W-HN4SC-7** | `shutdown` MUST drain via `drain_all`; the byte gate's behavior under shutdown is identical to its behavior in normal operation. |
| **W-HN4SC-8** | The byte gate MUST NOT alter the per-flush `DuplicateStagedKey` precedence (existing test `flush_batch_rejects_same_batch_duplicate_key` continues to pass unchanged). |
| **W-HN4SC-9** | The byte gate MUST NOT alter the per-flush `DuplicateEvent` precedence (existing test `flush_batch_across_calls_handles_idempotent_retry` continues to pass unchanged). |

---

## 4. Error Obligations

| ID | Obligation |
|---|---|
| **E-HN4SC-1** | No new error variant for the queued path; reuse `JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }`. |
| **E-HN4SC-2** | Diagnostic code `0x4022` and symbolic code `JOURNAL_BATCH_BYTES_EXCEEDED` MUST be reused (no new code in `crates/vb_storage/src/error/codes.rs`). |
| **E-HN4SC-3** | For every successful emission, `attempted > limit`. |
| **E-HN4SC-4** | `attempted` MUST equal `accumulated_bytes.checked_add(next_event_encoded_len).unwrap_or(u64::MAX)`. |
| **E-HN4SC-5** | `limit` MUST equal `self.byte_budget` (the active configuration value). |
| **E-HN4SC-6** | The byte-budget error message MUST remain `"journal batch byte budget exceeded: attempted {attempted} > limit {limit}"` (existing display string). |
| **E-HN4SC-7** | The misleading comment in `crates/workspace_tests/tests/journal_batch_accounting_tests.rs:48-51` claiming `JournalWriteBatch` does not enforce byte limits MUST be corrected. |

---

## 5. Boundary Obligations

| ID | Obligation |
|---|---|
| **B-HN4SC-1** | The pure gate predicate MUST NOT take `&FjallJournal`, `MutexGuard`, or any I/O handle as a parameter. |
| **B-HN4SC-2** | The pure gate predicate MUST be `fn gate_decision(&AccumulatedFlushBytes, EncodedRecordLength, u64) -> GateDecision`. |
| **B-HN4SC-3** | The byte accounting basis MUST be the full encoded record length (`encode_record` output), not the raw payload. |
| **B-HN4SC-4** | `usize::try_into::<u64>()` MUST be used for the `value.len() -> u64` conversion; the failure branch returns `JournalError::InvalidConfig { field: "encoded_record_length", reason: "value length exceeds u64" }`. |
| **B-HN4SC-5** | All state mutation MUST happen under `state: Mutex<JournalWriterQueueState>`; no new mutex introduced. |
| **B-HN4SC-6** | The gate MUST NOT introduce async/await, time, network, RNG, or `unsafe` boundaries. |
| **B-HN4SC-7** | `flush_batch` MUST remain `pub fn flush_batch(&self, journal: &FjallJournal) -> Result<JournalWriterFlushReport, JournalError>` (signature preserved). |
| **B-HN4SC-8** | `with_contracts` MUST remain `pub fn with_contracts(capacity: JournalQueueCapacity, batch_size: JournalBatchSize, limits: StorageLimits) -> Result<Self, JournalError>` (signature preserved). |

---

## 6. Hazard Mitigations (cross-references)

| ID | Mitigation | Hazard |
|---|---|---|
| **M-HN4SC-1** | Gate fires before `commit()`; rejects partial prefix | H-01 |
| **M-HN4SC-2** | Strict `>` comparison (`attempted > limit`) matches `JournalWriteBatch` | H-02 |
| **M-HN4SC-3** | `checked_add` returns `JournalBatchBytesExceeded { attempted: u64::MAX, limit }` on overflow | H-03 |
| **M-HN4SC-4** | Stack-local `u64` accumulator; no shared mutable state | H-04 |
| **M-HN4SC-5** | Panic-free gate (no `unwrap`/`expect`/`panic`) | H-05 |
| **M-HN4SC-6** | `EncodedRecordLength::new(0)` rejects zero-byte events | H-06 |
| **M-HN4SC-7** | `usize::try_into::<u64>()` with explicit `InvalidConfig` error | H-07 |
| **M-HN4SC-8** | Single encode per flush (Option A from codebase-map Open Question 5) | H-08 |
| **M-HN4SC-9** | `MAX_ENCODED_RECORD_BYTES = 1_048_636` ≤ `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576`? NO. The default budget is exactly `MAX_ENCODED_RECORD_BYTES - 60`, i.e. payload-only. A max-size encoded event is 60 bytes over budget. ACKNOWLEDGED: the contract accepts that the default budget rejects a single max-size event if interpreted as "full encoded length". Document in `StorageLimits` doc-comment that callers needing max-size events in one batch must raise the budget. | H-09 (revised) |
| **M-HN4SC-10** | Audit `git grep "StorageLimits {"` returns only `DEFAULT` definition | H-10 |
| **M-HN4SC-11** | Doc-comment on `with_contracts` notes that `limits` is now enforced | H-11 |
| **M-HN4SC-12** | Out of scope; carried to `proof-to-implementation` | H-12 |
| **M-HN4SC-13** | Parity test locks variant shape; `error-taxonomy.md §8` forbids parallel variants | H-13 |
| **M-HN4SC-14** | `gate_decision` is the single source of truth for the gate logic | H-14 |
| **M-HN4SC-15** | `const _STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND` asserts default binding at compile time | H-15 |
| **M-HN4SC-16** | Doc-comment on `JournalBatchBytesExceeded` notes shape stability | H-16 |
| **M-HN4SC-17** | Sync code; mutex-poisoning surfaces `WriteLockPoisoned` on subsequent calls | H-17 |
| **M-HN4SC-18** | Out of scope; carried forward | H-18 |
| **M-HN4SC-19** | Existing `journal.events.contains_key` idempotency check unchanged | H-19 |
| **M-HN4SC-20** | Stack-local accumulator; reset to `0` per `flush_batch` | H-20 |

**Important correction from the draft hazard analysis (H-09):** `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` is *payload* basis at the existing `JournalWriteBatch` (the existing harness `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event` proves this fits). The queued contract requires the *encoded* basis (60-byte header + payload = 1_048_636 bytes for max). This means the default budget rejects a single max-size event under the new contract. The contract resolves this by either:

(a) **Default `max_journal_batch_bytes = DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (1_048_576)** with the acknowledgment that the default fits only a max-payload event WITHOUT the 60-byte header — which is impossible since the encoder always emits the header. Therefore this default rejects a single max-payload event.

(b) **Default `max_journal_batch_bytes = MAX_ENCODED_RECORD_BYTES` (1_048_636)** to guarantee the default fits at least one max-size event. This is 60 bytes above `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` but preserves the "at least one max event fits" invariant already proven by `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event`.

The contract chooses **(b)** with an explicit `const` alias:

```text
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;          // existing, payload basis
pub const DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER: u64 = 1_048_636; // NEW
```

`StorageLimits::DEFAULT.max_journal_batch_bytes` MUST equal `DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER` (1_048_636), not the existing `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT`. The Kani harness `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event` already proves this fits a max-size event, so the parity claim is preserved.

This correction is reflected in M-HN4SC-9 above and in type-contracts.md §2.1 (the `Default` block).

---

## 7. Test Obligations (signaled to `test-writer`)

Required unit tests in `crates/vb_storage/src/queue/tests.rs`:

- `flush_batch_rejects_when_total_encoded_bytes_exceed_limit`
- `flush_batch_accepts_exact_fit_byte_budget` (one event whose encoded length equals the budget)
- `flush_batch_accepts_single_max_size_event` (encoded length 1_048_636 against default budget 1_048_636)
- `flush_batch_at_capacity_partial_drain_then_oversize_rejected_no_commit` (verify no `commit()` was called; pending unchanged)
- `flush_batch_rejects_on_checked_add_overflow` (synthetic accumulator near `u64::MAX`)
- `enqueue_does_not_enforce_byte_budget_only_flush_does`
- `drain_all_short_circuits_on_first_byte_budget_rejection`
- `journal_write_batch_and_journal_writer_queue_emit_identical_error_for_same_oversize_event` (parity test, mandatory per AC-1.3)

Required Kani harness:

- `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs` with `check_queued_byte_budget_invariants` that uses `kani::any()` for the accumulator, next event size, and limit; asserts:
  - overflow → `GateDecision::Reject { attempted: u64::MAX, limit }`
  - `accumulated + next <= limit` → `GateDecision::Accept`
  - `accumulated + next > limit` → `GateDecision::Reject { attempted: accumulated + next, limit }`
  - exact fit (`attempted == limit`) → `GateDecision::Accept`

The Kani harness MUST NOT be a hardcoded shape; it MUST use `kani::any()` with explicit bounds on `usize`/`u64` and MUST be wired into the `kani-vb-vzcuf` feature gate (consistent with ps006/ps007/ps009).

---

## 8. Compliance Checklist

| Check | Verified by |
|---|---|
| R-HN4SC-1 / AC-1.1–AC-1.6 | Implementation + tests + parity test |
| T-HN4SC-1..10 | Code review + compile-time const assertion (T-7) |
| W-HN4SC-1..9 | Unit tests + code review |
| E-HN4SC-1..7 | Parity test + code review + comment-correctness review |
| B-HN4SC-1..8 | Code review + cargo clippy (no async/no unsafe boundaries) |
| M-HN4SC-1..20 | Tests + Kani + parity + compile-time const |

---

## 9. Open Items (Non-Blocking)

- **OI-1.** `RuntimeError` classification of `JournalBatchBytesExceeded` (H-12). Carried to `proof-to-implementation`.
- **OI-2.** Optional `JournalWriterQueueProfileCounts.pending_bytes` observability field. Deferred.
- **OI-3.** Loom model for the byte accumulator. Out of scope.
- **OI-4.** Module wiring (`mod stage;`) CI smoke. Verification only, not a contract concern.
- **OI-5.** Memory bound by bytes (vs. event count). Out of scope; future bead.

---

## 10. References

- `domain-model.md` — ubiquitous language, aggregates, value objects, policies.
- `type-contracts.md` — newtype definitions, `gate_decision` predicate, const bindings.
- `workflow-model.md` — state machine, guard precedence, transitions.
- `error-taxonomy.md` — reused variant, parity claim, railway diagram.
- `boundary-map.md` — pure-core / imperative-shell separation.
- `hazard-analysis.md` — 25-hazard inventory.
- `proof-seeds.jsonl` — domain-level proof hints (no proof obligations; proof-planner owns).
- `traceability-matrix.jsonl` — requirement → clause → seed → lane hint mapping.