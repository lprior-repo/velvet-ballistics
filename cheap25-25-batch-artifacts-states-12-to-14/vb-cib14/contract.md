# Contract: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Acceptance Contract

Downstream states must implement and verify the following behavior-affecting requirements.

### C1 — Resumed Maps to RunResumed

- `StorageRuntimeJournal::storage_event(RuntimeJournalEvent::Resumed { run, timestamp }, seq)` MUST return `Ok(JournalEvent::RunResumed { run, seq, timestamp: convert_resume_timestamp(timestamp, run)? })`.
- The new arm lives in `boundary_storage_event` (mirroring `WaitScheduled → WaitScheduledEvent`).
- The mapper MUST NOT silently rewrite `Resumed` as `JournalEvent::RunFailedEvent`.

### C2 — Timestamp Conversion Is Total And Explicit

- The conversion `u64 → DateTime<Utc>` MUST be performed via `i64::try_from(timestamp_u64)` followed by `DateTime::<Utc>::from_timestamp(i64_secs, 0)`.
- On `i64::try_from` failure, the mapper MUST return `Err(RuntimeError::ResumeTimestampOverflow { run, timestamp })`.
- On `from_timestamp` returning `None` (far-future `i64_secs`), the mapper MUST also return `Err(ResumeTimestampOverflow { run, timestamp })`.
- The conversion MUST NOT use `as i64`, `unwrap`, `expect`, modular wrap, silent clamp, or panic.

### C3 — Storage Dispatch Totality (Paired With vb-edvbj)

- After this fix, `StorageRuntimeJournal::storage_event` is exhaustive over `RuntimeJournalEvent`. Each of the 16 variants reaches an explicit arm.
- The catch-all `Ok(JournalEvent::RunFailedEvent { .. })` at `chunk_002.rs:298–302` is NOT removed by this bead; that is vb-edvbj's responsibility.
- Once vb-edvbj removes the catch-all, this fix must already be in place. The two beads are STRONG-coupled for release.
- The compile-time exhaustive-match enforcement is the structural guard against future variants falling through.

### C4 — Single-Clone Invariant Preserved

- `StorageRuntimeJournal::storage_event` continues to call `clone_for_dispatch(&event)` exactly once per invocation, regardless of the variant.
- `STORAGE_EVENT_CLONE_COUNT` (test-only counter) increases by exactly 1 for a `Resumed` dispatch.
- The existing regression test `storage_event_clones_the_event_exactly_once_per_dispatch` at `crates/vb_runtime/src/journal/tests/chunk_002.rs:410–493` is extended to cover a `Resumed` arm sample.

### C5 — Recovery/Replay Classifies RunResumed As Active

- After this fix, a resumed run's journal contains exactly one `JournalEvent::RunResumed { run, seq, timestamp }`.
- `incident.rs::lifecycle_state(JournalEvent::RunResumed)` returns `LifecycleState::Active` (already correct at `crates/vb_storage/src/journal/incident.rs:203`).
- `recovery/hydrate.rs::is_in_flight_or_completed(JournalEvent::RunResumed)` returns `Ok(false)` (already correct at `crates/vb_storage/src/recovery/hydrate.rs:754`).
- The user-visible symptom (a resumed run reported as `Failed`) is removed.

### C6 — Seq And RunId Pass-Through

- `seq` is the per-run `EventSeq` supplied by the shard; the mapper MUST pass it through unchanged.
- `run` is the `RunId` carried by `RuntimeJournalEvent::Resumed`; the mapper MUST pass it through unchanged.
- The mapper MUST NOT derive `seq` from `timestamp`, increment `seq`, or zero `run`.

### C7 — Public Error Surface Adds ResumeTimestampOverflow

- A new `RuntimeError` variant `ResumeTimestampOverflow { run: RunId, timestamp: u64 }` is added.
- The variant MUST carry both `run` and the original `timestamp: u64` for diagnostics.
- The variant MUST NOT be a unit variant.
- `RuntimeError` may need a `#[non_exhaustive]` attribute if it does not already have one; the implementation agent must verify and add it.
- Existing `ResumeError::JournalAppendFailedWithSource` already propagates this error from the journal append path; no shard-side change is required.

## Verus Mirror Binding

- Strong/Weak production binding: the existing Verus mirror at `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:616-624` declares `RunResumed { run: u64, seq: EventSeq, timestamp: u64 }`. The production shape `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }` is unchanged. The mirror stays accurate.
- Drift gate: `scripts/check-verus-production-binding.sh` continues to pass after this fix because no `JournalEvent` shape changes; only the runtime-side mapper adds an arm.
- Mirror references at lines 715, 748, 792, 839 (mirror `run_id`, `seq`, and `is_valid`) are unchanged.

## TLA+-Owned Clauses

- None. The existing resume FSM refinement obligation (RRO-TLA-RESUME-001) at `verification/tla/rust-refinement-obligations.jsonl:6` is unchanged by this fix. The shard-side `handle_resume` → `append_resumed_event` → `apply(RuntimeEvent::Resume)` sequence is the TLA+ refinement target; the mapper is downstream of that and does not affect the FSM.

## Verus-Owned Clauses

- C1, C2, C6, C7 — mapper function correctness, conversion totality, and error variant shape. The mirror at `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` covers the `JournalEvent::RunResumed` shape; a new spec for the mapper arm is expected.

## Kani-Owned Clauses

- C2 — exhaustive harness of `u64 → i64 → DateTime<Utc>` over the `u64` range (or a bounded subset covering the realistic Unix timestamp range plus overflow sentinels).
- C3 — bounded proof that `storage_event` is exhaustive over a generated `RuntimeJournalEvent` (no fallthrough). The 16-variant enumeration test at `chunk_004.rs:1077-1090` is the behavior anchor; a Kani harness should bound-check the dispatch.
- C4 — bounded proof that `STORAGE_EVENT_CLONE_COUNT == 1` after one `storage_event(Resumed, _)` call.

## Flux-RS-Owned Clauses

- None required; the mapper is structurally simple and the conversion is total over `u64`.

## Property-Test-Owned Clauses

- C1, C2 — proptest over `(timestamp: u64, seq: EventSeq, run: RunId)` to assert that `storage_event(Resumed { run, timestamp }, seq)` produces either `Ok(RunResumed { run, seq, DateTime::from_timestamp(...) })` or `Err(ResumeTimestampOverflow { run, timestamp })` — never any other variant.
- C6 — proptest that `seq` is pass-through: `mapped_event.seq() == seq` for all inputs.

## Non-Goals

- No implementation in State 3.
- No behavior tests or verifier harnesses in State 3.
- No change to `RuntimeJournalEvent::Resumed` signature (still `u64` seconds since UNIX epoch).
- No change to `JournalEvent::RunResumed` shape (still `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }`).
- No change to `Shard::handle_resume` or `Shard::append_resumed_event` logic.
- No removal of the `RunFailedEvent` catch-all (vb-edvbj's responsibility).
- No change to `RecordKind` family or codec validation.
- No new `JournalEvent` variant.

## Coupled Bead

- `vb-edvbj` — deletes the `Ok(JournalEvent::RunFailedEvent { .. })` catch-all at `chunk_002.rs:298–302`. STRONG-coupled release dependency. vb-cib14 must land before (or simultaneously with) vb-edvbj so that the dispatch remains total after the catch-all is removed.

## Bridge Pointers for Later States

- Public API change surface: `crates/vb_runtime/src/error.rs` (new `RuntimeError::ResumeTimestampOverflow` variant; `#[non_exhaustive]` if needed).
- Mapper site: `crates/vb_runtime/src/journal/chunk_002.rs` (`boundary_storage_event` arm for `Resumed`; conversion helper).
- Shard append: `crates/vb_runtime/src/journal/chunk_002.rs::StorageRuntimeJournal::append_sequenced` (unchanged; propagation via `?`).
- Recovery/replay: `crates/vb_storage/src/journal/incident.rs:203`, `crates/vb_storage/src/recovery/hydrate.rs:754`, `crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126` (unchanged; already correct).
- Verus mirror reference: `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:616-624` (unchanged shape; binder for `JournalEvent::RunResumed`).
- Refinement harness target: `verification/tla/rust-refinement-obligations.jsonl:6` (RRO-TLA-RESUME-001; the new mapper arm adds a source ref to `shard/lifecycle/chunk_001.rs:291-367`).
- Test anchor: `crates/vb_runtime/src/journal/tests/chunk_002.rs:410-493` (single-clone regression test; extend with a `Resumed` sample).
- Pattern test anchor: `crates/vb_runtime/src/journal/tests/chunk_002.rs:150-195` (`re_009_wait_resolved_maps_to_dedicated_journal_event`) — same shape as the new `Resumed` regression test.
- Registered workspace target: `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:225` (CLI/storage parity; unchanged but continues to pass).