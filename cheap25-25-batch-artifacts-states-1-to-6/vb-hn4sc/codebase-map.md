# Codebase Map — vb-hn4sc

bead_id: vb-hn4sc
bead_title: Storage: enforce byte-budget limits in queued group commits (P1 bug)
phase: 2 (explore)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
source_checkout: /home/lewis/src/velvet-ballistics
scout_role: read-only discovery (no production edits)
captured_at: 2026-07-01T15:25:00Z

## Bead Statement

> QueuedWriter group_commit currently accepts unbounded byte payloads. Add
> byte-budget enforcement per group commit and reject oversize groups with a
> typed error.

In this codebase "QueuedWriter" is `vb_storage::queue::JournalWriterQueue`
and "group commit" is `JournalWriterQueue::flush_batch` (the function that
atomically drains up to `batch_size` queued events into a single
`fjall::OwnedWriteBatch` and commits it). The bead asks for a byte-budget
gate analogous to the one already enforced by `JournalWriteBatch::append_event`
(see `crates/vb_storage/src/batch/append_event.rs:86-102` and the existing
typed error `JournalError::JournalBatchBytesExceeded`).

## Scope Summary

Primary touched crate: **vb_storage**. The change is bounded inside the
storage crate and only touches the queued-writer path. Production callers
(`vb_cli::ipc_serve`, `vb_runtime::journal::QueuedStorageRuntimeJournal`,
`vb_runtime::journal::RuntimeJournalConfig::shared_journal`) already accept
a `JournalWriterQueue` from the outside and rely on its public surface;
therefore any signature change must remain source-compatible or have a
backwards-compatible companion constructor.

## Touched / Suspected Files

### Primary (must change)

| Path | Role | Why |
|---|---|---|
| `crates/vb_storage/src/queue/writer.rs` | `JournalWriterQueue` impl | Holds `enqueue_journaled`, `enqueue_strict`, `flush_batch`, `drain_all`, `shutdown`. Today the `_limits: StorageLimits` parameter at line 54 is **ignored** (`_limits`). This is the queue-side gap. |
| `crates/vb_storage/src/queue/writer/stage.rs` | `stage_queued_event` helper | Re-encodes each event inside the per-flush OwnedWriteBatch (see `encode_record` call at lines 61-67). The encoded `value.len()` is the natural per-event byte accounting source. |
| `crates/vb_storage/src/types.rs` | `StorageLimits`, `JournalWriterFlushReport`, `JournalWriterQueueProfileCounts` | `StorageLimits` only carries per-event `max_journal_event_payload_bytes` (line 12). It must grow a batch-level `max_journal_batch_bytes` (or `byte_budget`) field that defaults to the `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` already defined at `crates/vb_storage/src/batch/types.rs:10`. |
| `crates/vb_storage/src/error/mod.rs` | `JournalError::JournalBatchBytesExceeded { attempted, limit }` | Variant already exists (line 41) with diagnostic code `JOURNAL_BATCH_BYTES_EXCEEDED_CODE = 0x4022`. Reuse it; do not introduce a parallel `QueuedBatchBytesExceeded`. |
| `crates/vb_storage/src/error/codes.rs` | `JournalError::diagnostic_code` / `symbolic_code` | Already maps `JournalBatchBytesExceeded` to `0x4022` and `JOURNAL_BATCH_BYTES_EXCEEDED`. No new codes needed. |
| `crates/vb_storage/src/queue/writer_contract.rs` | Pure predicates (`enqueue_allowed`, `strict_batch_remove_decision`, etc.) | Optional: extend `StorageQueueDecisionState` (line 18) with `byte_budget: u64` + `accumulated_bytes: u64` so the new decision can be unit-pure and Verus-compatible. |
| `crates/vb_storage/src/queue/mod.rs` | Module wiring | Currently `mod writer;` (loads `writer.rs`) and inside `writer.rs` `mod stage;` resolves to `writer/stage.rs` via the Rust 2018 "module file coexists with submodule directory" rule. No change required, but the wiring is fragile and worth verifying in CI. |
| `crates/vb_storage/src/queue/tests.rs` | Queue unit tests (1169 lines, ~60+ tests) | Add coverage: `flush_batch_rejects_oversize_byte_budget`, `drain_all_returns_first_oversize_error`, `enqueue_at_capacity_bytes_succeeds_when_event_fits`, `flush_batch_partial_drain_then_reject`, plus parity check that `JournalWriteBatch` and `JournalWriterQueue` emit identical error variants for the same oversize event. |

### Pattern Sources (read-only)

| Path | Role | Why |
|---|---|---|
| `crates/vb_storage/src/batch/types.rs` | `JournalWriteBatch` impl | Defines `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576`, `staged_bytes: u64`, `byte_limit: Option<u64>`. Direct path already enforces byte budget; queued path is the gap. |
| `crates/vb_storage/src/batch/append_event.rs` | `JournalWriteBatch::append_event` | Lines 86-102 show the canonical checked_add overflow pattern + `JournalError::JournalBatchBytesExceeded` raise. Replicate this exact contract on the queued side. |
| `crates/vb_storage/src/batch/t_byte_accounting_part1.rs` … `t_byte_accounting_part4.rs` | Existing byte-accounting tests | Pattern reference for the new queued-byte tests (constructor invariants, checked_add overflow, exact-fit acceptance, over-limit rejection, no-state-mutation on rejection). |
| `crates/vb_storage/src/constants.rs` | `MAX_BATCH_COUNT = 10_000`, `MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576`, `RECORD_HEADER_BYTES = 60` | Bound the worst-case per-event encoded size to `60 + 1_048_576 = 1_048_636` bytes — already covered by the Kani bridge harness `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event`. The new byte-budget gate must accept at least one max-size event. |
| `crates/vb_storage/src/kani_vb_vzcuf_ps007.rs` | Kani bridge harness | Already proves `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` matches the `vb_core::max_journal_batch_bytes` policy and that `max_encoded <= limit`. New harness needed for the queued path's per-flush arithmetic. |
| `crates/vb_storage/src/kani_vb_vzcuf_ps006.rs` | `check_byte_limit_arithmetic_safe` | Reference pattern for byte-limit Kani proof (cfg(kani) module + checked_add assertions). |
| `crates/vb_storage/src/kani_vb_vzcuf_ps009.rs` | `check_staged_bytes_monotonic` | Reference pattern for "staged bytes never decrease" property. Apply the analogous invariant to the queued writer's per-flush accumulator. |

### Downstream Callers (must compile, semantics-preserving if possible)

| Path | Role | Note |
|---|---|---|
| `crates/vb_cli/src/ipc_serve.rs:18-25` | Constructs `JournalWriterQueue::new(1024, 64, StorageLimits::DEFAULT)` | Largest production caller; today passes `StorageLimits::DEFAULT`. Adding a `byte_budget` field to `StorageLimits` with a default of 1 MiB keeps this code working. |
| `crates/vb_runtime/src/journal/chunk_002.rs:357-415` | `QueuedStorageRuntimeJournal::{flush_batch,drain_all}` | Pass-through wrappers. The `Err(JournalError)` already propagates via `RuntimeError::from`. Verify `RuntimeError` carries a typed byte-budget variant or a fallback classification (see Open Questions). |
| `crates/vb_runtime/src/journal/chunk_001.rs:380-393` | `RuntimeJournalConfig::shared_journal` | Consumes `Arc<JournalWriterQueue>` built outside. No signature change needed if `JournalWriterQueue::new` keeps its current arity. |
| `crates/vb_runtime/src/journal/tests/chunk_001.rs:55-58` | `journal_queue` test helper | Pattern that callers will replicate; if a `byte_budget: u64` becomes a 4th positional arg, every test helper must update. |
| `crates/vb_runtime/src/journal/tests/chunk_002.rs:240, 243`, `chunk_003.rs:33,127,134`, `chunk_004.rs:932,939,946` | Runtime adapter tests | Assert specific `JournalWriterFlushReport { drained, written }` values; byte-budget rejection returns `Err`, so existing Ok-path tests stay green. New tests for Err path needed. |
| `crates/vb_runtime/tests/recovery_integration.rs:466-477, 531` | Recovery integration | Constructs queue, enqueues events, drains. No change needed if default byte budget ≥ current event sizes. |
| `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs:324,351` | Cross-crate contract test | Uses `JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)` and `drain_all`. No change needed for default-limit compatibility. |

### Tests / Proofs / Fuzz Currently Exercising The Gap

| Path | Test name / harness | Coverage |
|---|---|---|
| `crates/vb_storage/src/queue/tests.rs` | `flush_batch_rejects_same_batch_duplicate_key`, `flush_batch_across_calls_handles_idempotent_retry`, `shutdown_drains_all_pending_events`, ~60 others | Drain / enqueue / shutdown mechanics — **no byte-budget coverage**. |
| `crates/vb_storage/src/tests.rs` (lines 502, 855, 888, 4793-5000, 5843-5985) | `JournalWriterQueue::new` + `flush_batch`/`drain_all` exercised ~30 times | No byte budget checks. |
| `crates/vb_storage/src/edge_case_tests.rs:126,590,603,618` | Edge cases for queue | Capacity / batch_size edge cases only. |
| `crates/vb_storage/src/index_maintenance_tests.rs:46-545` | `drain_all` after enqueue | Index maintenance, not byte accounting. |
| `crates/vb_storage/src/proptest_storage.rs:231-284` | Disabled proptest (vb-b8i8f follow-up) | Listed for awareness; do not rely on this for proof closure. |
| `crates/vb_runtime/src/models/loom/journal_writer_queue.rs:58-130` | Loom model `JournalWriterQueue` (mock, **not** the real queue) | Mocks `pending: AtomicUsize`; does not test bytes. UNRELATED to this bead. |
| `crates/vb_storage/src/kani_vb_vzcuf_ps006.rs`, `ps007.rs`, `ps009.rs` | Byte-budget Kani harnesses | Cover `JournalWriteBatch`, **not** `JournalWriterQueue`. New `ps010`-style harness (or extend `ps009`) needed for queued path. |
| `crates/workspace_tests/tests/journal_batch_accounting_tests.rs` | Cross-crate byte accounting | Comment at line 48-51 documents: "JournalWriteBatch does not enforce byte limits directly. Byte budget enforcement happens at the runtime/budget layer via BudgetError::JournalBatchBytesExceeded. The batch only enforces count limits." This is misleading and contradicts the per-event byte check at `crates/vb_storage/src/batch/append_event.rs:86-102`; update that doc-comment as part of the change. |

## Public APIs Implicated

### Already present (reusable)

- `vb_storage::JournalWriterQueue::new(capacity: usize, batch_size: usize, limits: StorageLimits) -> Result<Self, JournalError>` — `crates/vb_storage/src/queue/writer.rs:40-48`. `_limits` parameter at line 54 is unused today.
- `vb_storage::JournalWriterQueue::enqueue_journaled(event: JournalEvent) -> Result<(), JournalError>` — line 67.
- `vb_storage::JournalWriterQueue::enqueue_strict(event: JournalEvent) -> Result<(), JournalError>` — line 72.
- `vb_storage::JournalWriterQueue::flush_batch(journal: &FjallJournal) -> Result<JournalWriterFlushReport, JournalError>` — line 152 (THE group commit). Currently drains up to `batch_size` events; must add byte-budget gate.
- `vb_storage::JournalWriterQueue::drain_all(journal: &FjallJournal) -> Result<JournalWriterFlushReport, JournalError>` — line 237.
- `vb_storage::JournalWriterQueue::shutdown(journal: &FjallJournal) -> Result<JournalWriterFlushReport, JournalError>` — line 266.
- `vb_storage::JournalWriterQueue::pending_profile_counts() -> Result<JournalWriterQueueProfileCounts, JournalError>` — line 94.
- `vb_storage::JournalWriterQueue::probe_accepting_writes() -> Result<(), JournalError>` — line 120.
- `vb_storage::JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }` — `crates/vb_storage/src/error/mod.rs:40-41`. Reuse.
- `vb_storage::StorageLimits::DEFAULT` — `crates/vb_storage/src/types.rs:15-20`. Only carries `max_journal_event_payload_bytes`.
- `vb_storage::JournalWriterFlushReport { drained: usize, written: usize }` — `crates/vb_storage/src/types.rs:165-171`. No change; byte-budget rejection is `Err`, not a different `Ok` variant.
- `vb_storage::JournalWriterQueueProfileCounts { journaled: usize, strict: usize }` — `crates/vb_storage/src/types.rs:156-162`. Possibly extend with `pending_bytes: u64` for observability (optional, see Open Questions).

### Likely new (or extend)

- `vb_storage::StorageLimits` — add `pub max_journal_batch_bytes: u64` with `DEFAULT` set to `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (1_048_576). Source-compatible if added with a constructor / `Default`.
- `vb_storage::JournalWriterQueue::with_contracts(...)` — wire `_limits` to a `byte_budget: u64` field on `JournalWriterQueueState` (or on `JournalWriterQueue` itself).
- `vb_storage::batch::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` — already public, `1_048_576`. Re-export or reference from the queued path so both paths share the constant.

### Not in scope

- `vb_runtime::action_queue.rs`, `vb_runtime::shard::command_queue_*`, `vb_runtime::engine::StepBudget` — distinct bounded queues; not the same code path as the journal writer.
- `vb_storage::queue::BatchBuilder` (`crates/vb_storage/src/queue/batch.rs`) — different type for non-queued callers. Not affected, but its `cap` field stays event-count-only.
- `crates/vb_storage/src/queue/writer_contract.rs` Verus route — only touched if proof-plan includes a Verus spec for the new byte-budget gate (currently no Verus spec covers the queued path).

## Existing Contracts / Invariants Relevant to the Change

- **Atomicity (master §49 Crash-Consistency Rule).** `flush_batch` stages every event into a single `fjall::OwnedWriteBatch` (line 194 of `writer.rs`); the batch either commits all-or-nothing. The new byte-budget gate MUST fire **before** `owned_batch.commit()` so a partial prefix is never observable. Concretely: at most N events pass the byte gate, those N are staged, then commit; the oversize event is rejected with `JournalBatchBytesExceeded` and the queue is left holding the rejected event plus any not-yet-staged events. Drained count = N, written count = N, queue still has events.
- **Idempotency (per-flush `staged_keys`).** `stage_queued_event` (`writer/stage.rs:38-43`) returns `DuplicateStagedKey` when the same `(run, seq)` is staged twice in the same flush. Byte-budget check must run **after** the staged_keys gate (so duplicate detection still fires first, preserving test `flush_batch_rejects_same_batch_duplicate_key`) and **before** `owned_batch.insert` (so no partial insert happens on rejection).
- **Durability profile selection (writer.rs:163-171).** `has_strict` is computed by walking up to `batch_size` items; SyncAll is forced iff any staged event is Strict. The byte-budget gate must respect this: a partial batch of mixed profiles that fits in the byte budget is fine.
- **No unwrap / no panic / no unchecked index (Holzman Rust).** `writer.rs` and `writer/stage.rs` use `checked_add`/`get` patterns already; the new gate must extend this discipline.
- **Per-event payload cap (existing).** `encode_record(..., MAX_JOURNAL_EVENT_PAYLOAD_BYTES, ...)` at `writer/stage.rs:66` already returns `PayloadTooLarge` for an oversize single event; the new gate handles the *sum* across multiple events.

## Risk Tags

| Tag | Why |
|---|---|
| `persistence` | The byte gate changes what gets durably written per group commit. Must remain all-or-nothing. |
| `public-api` | `StorageLimits` field addition; `JournalWriterQueue` signature wiring (limits parameter goes from ignored to enforced). |
| `contract-parity` | Direct `JournalWriteBatch` path already enforces the budget; queued path must match (same error variant, same accounting basis: encoded record length, not raw payload length — see `t_byte_accounting_part1.rs:accounting_uses_full_encoded_length_not_payload_length`). |
| `migration` | Production callers (`vb_cli::ipc_serve`, `RuntimeJournalConfig::shared_journal`) currently pass `StorageLimits::DEFAULT`. New field must default to the same 1 MiB value used by `JournalWriteBatch` to avoid silent payload-size regressions for already-deployed callers. |
| `concurrency` | `JournalWriterQueue::state: Mutex<JournalWriterQueueState>`; byte accumulator must be mutated under the same lock to avoid races between `enqueue_*` writers and `flush_batch`. |
| `error-classification` | The new error path is `JournalBatchBytesExceeded`, which already maps to `JOURNAL_BATCH_BYTES_EXCEEDED` (0x4022). Runtime propagation in `vb_runtime::journal::chunk_002.rs:406` uses `RuntimeError::from(JournalError)` — verify `RuntimeError::From<JournalError>` handles `JournalBatchBytesExceeded` and surfaces it as a budget-exhaustion signal, not a generic write error. UNKNOWN until verified. |

## Open Questions / Unknowns

1. **Enforce at enqueue or at flush_batch?** Bead text says "byte-budget enforcement per group commit" → naturally read as `flush_batch` time. Alternative: per-event enqueue rejection using `staged_bytes + event_byte_size > limit`. Decision affects the failure semantics (callers can retry after a partial drain vs. must drop the offending event). RECOMMENDATION: enforce at `flush_batch`, returning the first oversize event's byte count; leave already-enqueued events in the queue so the next flush can re-attempt (with the oversize event still failing until dropped). This matches the spirit of "reject oversize groups".
2. **What byte-size basis?** Already established by `JournalWriteBatch::append_event`: full encoded record length (`encode_record` output), not raw payload. Reuse this exact basis for parity (see `t_byte_accounting_part1.rs:accounting_uses_full_encoded_length_not_payload_length`).
3. **`StorageLimits` field naming.** Could be `max_journal_batch_bytes`, `byte_budget`, or `group_commit_byte_limit`. RECOMMENDATION: `max_journal_batch_bytes` to match `max_journal_event_payload_bytes` and the existing `vb_core::max_journal_batch_bytes` Kani-bridged value at `kani_vb_vzcuf_ps007.rs:42`.
4. **Default value.** Must equal `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (`1_048_576`) at `crates/vb_storage/src/batch/types.rs:10`. Cross-check `vb_core::max_journal_batch_bytes` default via `kani_vb_vzcuf_ps007::check_default_batch_byte_limit` (already PASS per existing evidence).
5. **Per-event size caching.** Option A: re-encode during `flush_batch` (re-uses existing `encode_record` call in `stage_queued_event`). Option B: encode once at `enqueue_*` and store the byte count on `QueuedJournalEvent`. Option B avoids double-encoding for events that are re-flushed across multiple `drain_all` iterations, but adds 8 bytes per queued event. RECOMMENDATION: Option A — single encode per flush, memory-neutral, no queue-state drift.
6. **`JournalWriterQueueProfileCounts` extension.** Optional observability field `pending_bytes: u64`; not required for the byte gate itself but useful for callers that want to back-pressure before enqueueing. UNKNOWN whether needed; defer unless a caller asks.
7. **RuntimeError propagation.** `JournalError::JournalBatchBytesExceeded` propagates through `RuntimeError::from`. Whether `RuntimeError` exposes a typed `BudgetExhausted` variant for this case is UNKNOWN; must verify before claiming the wire path is honest.
8. **Loom model.** `vb_runtime::models::loom::journal_writer_queue.rs` is a mock (local `AtomicUsize`); it does NOT exercise `vb_storage::queue::JournalWriterQueue`. A real Loom schedule-exploration harness for the new byte accumulator is OUT OF SCOPE for this bead (per Holzman Rust scope discipline).
9. **Module wiring.** `crates/vb_storage/src/queue/writer.rs:14` declares `mod stage;` which resolves to `writer/stage.rs` via the Rust 2018 "module file coexists with subdirectory" rule. Verified by the parent (`/home/lewis/src/velvet-ballistics`) and isolated workspace having identical layout. Worth re-verifying on a fresh `cargo check -p vb_storage` as a CI smoke before editing.

## Verification Plan (Recommended for Downstream States)

- **Unit (State 7).** Add at least four tests to `crates/vb_storage/src/queue/tests.rs`:
  1. `flush_batch_rejects_when_total_encoded_bytes_exceed_limit`
  2. `flush_batch_accepts_exact_fit_byte_budget`
  3. `flush_batch_at_capacity_partial_drain_then_oversize_rejected_no_commit`
  4. `enqueue_does_not_enforce_byte_budget_only_flush_does`
  Plus a parity test that `JournalWriteBatch` and `JournalWriterQueue` return identical error variants for the same `(attempted, limit)` value.
- **Kani (State 6).** Add a harness at `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs` (new file) that proves the per-flush accumulator arithmetic is checked_add-safe and that `attempted > limit ⟹ JournalBatchBytesExceeded`. Wire into the existing `kani-vb-vzcuf` feature gate.
- **Proptest (State 7).** Re-enable `proptest_storage.rs` per `vb-b8i8f` follow-up, OR add a small `proptest` block at the bottom of `queue/tests.rs` that generates random event sequences within the byte budget.
- **Manual QA (State 8).** CLI: start `vb_cli ipc-serve`, enqueue events whose total encoded size is just under, at, and over `1_048_576`, observe `JOURNAL_BATCH_BYTES_EXCEEDED` diagnostic code (0x4022) on the third case.
- **moon ci (State 11).** Confirm green after the change: `tmp/test-output.tmp`, fresh `--force` rerun per `vb-scxh` precedent.

## Excluded Paths (Out of Scope for This Bead)

- `crates/vb_runtime/src/shard/command_queue_*`, `crates/vb_runtime/src/action_queue.rs` — distinct bounded queues, not the journal writer.
- `crates/vb_storage/src/recovery/*` — replay does not invoke the writer queue.
- `crates/vb_storage/src/queue/BatchBuilder` (`crates/vb_storage/src/queue/batch.rs`) — different API for non-queued callers; no byte accounting today and bead does not ask for it.
- `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` — mock, not production.
- `crates/vb_compile/src/expr_lexer/tests/adversarial.rs` — unrelated "byte budget" for string literal parsing, not journal bytes.
- `crates/vb_storage/src/batch/types.rs` `JournalWriteBatch` itself — already enforces the gate; only the constant reference (`DEFAULT_JOURNAL_BATCH_BYTE_LIMIT`) is borrowed.

## Downstream Owners

- **rust-contract:** Own the `StorageLimits` extension (`max_journal_batch_bytes`) and decide whether `flush_batch`'s new error variant is `JournalBatchBytesExceeded` (recommended) or a parallel `QueuedBatchBytesExceeded` (NOT recommended — duplicates diagnostic codes).
- **proof-planner:** Plan Kani harness for queued-byte arithmetic and (optionally) a Verus spec extension for `writer_contract.rs::StorageQueueDecisionState`.
- **test-writer:** Author unit + parity tests in `queue/tests.rs`; enable `proptest_storage.rs` if budget allows.
- **holzman-rust:** Implement `JournalWriterQueue::flush_batch` byte gate using `checked_add`, replicate `JournalWriteBatch::append_event` pattern exactly; preserve all-or-nothing atomicity.
- **black-hat-reviewer:** Verify contract parity (direct batch vs. queued path emit the same error variant for the same oversize scenario).
- **truth-serum / evidence-packaging:** Capture raw `cargo test`, `cargo kani --harness check_queued_byte_budget_invariants -p vb_storage`, and `moon ci --force --summary normal` outputs in `.beads/vb-hn4sc/`.

## Anti-Hallucination Checks Performed

- Verified `JournalBatchBytesExceeded` variant and `JOURNAL_BATCH_BYTES_EXCEEDED_CODE` exist by reading `crates/vb_storage/src/error/mod.rs:40-41` and `crates/vb_storage/src/error/codes.rs:74,172,251`.
- Verified `_limits: StorageLimits` is currently unused at `crates/vb_storage/src/queue/writer.rs:54`.
- Verified `JournalWriterQueue::flush_batch` signature and call site at `crates/vb_storage/src/queue/writer.rs:152-231` and downstream wrappers `crates/vb_storage/src/public_api.rs:42-47`, `crates/vb_runtime/src/journal/chunk_002.rs:404-415`.
- Verified `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` at `crates/vb_storage/src/batch/types.rs:10`.
- Verified `MAX_BATCH_COUNT = 10_000`, `MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576`, `RECORD_HEADER_BYTES = 60` at `crates/vb_storage/src/constants.rs:88,100,84`.
- Verified `StorageLimits::DEFAULT` shape at `crates/vb_storage/src/types.rs:15-20` (only carries per-event cap).
- Verified `JournalWriterQueue::new` callers across `vb_cli`, `vb_runtime`, `vb_storage` tests, and `workspace_tests` via `rg 'JournalWriterQueue::new'` — all pass `StorageLimits::DEFAULT` (no caller currently customizes byte limits, so default-compat is sufficient).
- Verified no kani/proptest harness covers the queued path byte budget; no `vb-hn4sc` artifacts pre-existed in `.beads/vb-hn4sc/`.
- Verified module wiring `mod stage;` in `writer.rs:14` resolves to `writer/stage.rs` (Rust 2018 file-and-subdirectory co-existence rule); flagged as a CI smoke candidate.
- UNKNOWN (explicitly): RuntimeError classification of `JournalBatchBytesExceeded` (item 7 in Open Questions); requires reading `crates/vb_runtime/src/error/conversions.rs` or equivalent in State 3.
- UNKNOWN (explicitly): Whether existing `flush_batch` callers in `vb_runtime` already dequeue events in a retry loop that would mask a byte-budget rejection — flagged for `test-writer`.
