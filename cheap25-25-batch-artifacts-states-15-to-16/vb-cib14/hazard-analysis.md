# Hazard Analysis: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Temporal Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| vb-edvbj deletes the `RunFailedEvent` fallback before this fix lands | Compile-time error from match-exhaustiveness checker because `Resumed` still falls through to a synthetic `RunFailedEvent`. | Both beads must land in the same release; the new `Resumed` arm must be present before the catch-all is removed. |
| Two resume attempts in flight for the same run before the first `Resumed` is durable | Double-emit of `RunResumed` events; replay sees two resume events. | Pre-existing shard-level invariant: `RuntimeState::Resuming` is the in-flight state and blocks concurrent `apply(RuntimeEvent::Resume)` until the first one resolves or rolls back. This fix does not change that invariant. |
| Storage append latency between conversion success and durable write | The mapper returns `Ok(RunResumed)` before the durable append completes; an in-process crash between them leaves the journal without the event but in-memory state at `Resuming`. | Existing rollback path: `Shard::append_resumed_event` catches the error and rolls back via `RuntimeEvent::ResumeRollback`. The fix preserves this path. |
| `current_timestamp()` returns 0 when `SystemTime::now()` predates UNIX_EPOCH (unlikely but representable) | `RunResumed { timestamp: DateTime::<Utc>::from_timestamp(0, 0) }` would be a valid chrono value but semantically wrong. | This is a system-clock bug, not a mapper bug. The mapper must still emit the chrono conversion; an upstream wall-clock check belongs in `current_timestamp()`. |
| Far-future `u64` values (e.g. `u64::MAX`) | `DateTime::<Utc>::from_timestamp(i64::MAX, 0)` returns `None` for some values near the boundary. | The mapper must surface this as `Err(ResumeTimestampOverflow)` rather than unwrap. Realistic UNIX timestamps are well below this bound. |
| Boundary dispatcher reordering | If the `Resumed` arm is placed in `run_storage_event` or `action_storage_event` instead of `boundary_storage_event`, the seq is preserved (because they all pass `seq`) but the family classification is wrong. | Domain contract: `Resumed` is a boundary event (it crosses the runtime/storage boundary at a clean point in the resume FSM). The new arm belongs in `boundary_storage_event` to mirror the existing `WaitScheduled → WaitScheduledEvent` pattern. |

## Rust-Core Invariant Hazards

- **Single-clone regression:** adding a `Resumed` arm that consumes the cloned event twice (e.g. for both seq extraction and timestamp conversion) would break the `storage_event_clones_the_event_exactly_once_per_dispatch` invariant. The fix must destructure once and pass the parts forward.
- **Conversion dispatch on `&event` vs `event`:** the existing `storage_event` matches on `&event` to enable the single-clone pattern. The new `Resumed` arm must follow the same pattern; it must NOT re-match on the owned event inside the helper because that would require a second clone.
- **`seq` pass-through:** the mapper must NEVER derive `seq` from anything other than the `seq` parameter. It must not call `EventSeq::new(timestamp as u64)` or similar by mistake.
- **`RunId(0)` rejection:** if `run == RunId(0)`, the produced `RunResumed` would fail `JournalEvent::is_valid()`. This is a pre-existing invariant of `JournalEvent::is_valid()`; the mapper should not enforce it but must not silently rewrite `RunId(0)` as a different event.
- **Totality loss:** any new `RuntimeJournalEvent` variant added in the future must be added to all three dispatch helpers or the catch-all must be retained. Once vb-edvbj removes the catch-all, future variant additions become compile errors unless they are added to all three helpers — this is the desired enforcement.
- **`as i64` cast hazard:** `timestamp as i64` on a `u64 > i64::MAX` truncates silently on two's-complement targets. The mapper MUST use `i64::try_from(timestamp_u64)` and propagate the error.
- **`from_timestamp` Option hazard:** `DateTime::<Utc>::from_timestamp(secs, 0)` returns `Option<DateTime<Utc>>`. The mapper MUST handle the `None` case (the current realistic UNIX range never hits `None`, but far-future values can).

## Storage / Codec Hazards

- **Mirror drift:** the Verus mirror `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:616-624` declares `RunResumed { run: u64, seq: EventSeq, timestamp: u64 }`. The production `JournalEvent::RunResumed` is `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }`. The mirror uses `u64` as a placeholder for `DateTime<Utc>`. After this fix, the production shape is unchanged, so the mirror stays accurate. If the production shape changes (e.g. timestamp renamed), the mirror must be updated.
- **Recovery classifier symmetry:** `incident.rs::lifecycle_state` and `recovery/hydrate.rs::is_in_flight_or_completed` already classify `RunResumed` correctly. The bug was the producer; the classifiers were correct. Once the producer is fixed, the classifiers continue to classify correctly.
- **RecordKind stability:** `RecordKind::RunResumed` is a stable record kind. The fix does not change it. No `is_known_record_kind` or `validate_kind_family` change is required.
- **CLI parity:** `vb_cli::lifecycle::resume` writes `JournalEvent::RunResumed` directly with `Utc::now()` (lines 150–220). After this fix, the runtime-side resume path also produces `RunResumed` with the correct chrono shape. CLI tests (`vb_test_cli_storage_io_behavior.rs:225`) remain valid.

## Bounded-State Hazards

- **Replay sequence gap:** the mapper must pass `seq` through. A replay that observes a `RunResumed` with `seq != expected` will fail replay validation. The mapper's contract is pass-through, not derivation.
- **Capacity hazards:** the mapper itself does not allocate beyond the `DateTime<Utc>` (which is `Copy`). No new heap allocations are introduced.

## Concurrency / Scheduling Hazards

- **Shard-singlethreadedness:** the shard is single-threaded; no intra-shard locking is required for the mapper. The mapper is a pure function of `(event, seq)`.
- **Storage concurrency:** `StorageRuntimeJournal::storage_event` is called from the shard thread; Fjall appends go through `append_storage_event` which uses `append_journaled`/`append_strict`. No new concurrency surface is introduced.
- **Concurrent resume attempts:** already blocked at the runtime-state level (`RuntimeState::Resuming` blocks concurrent `apply(RuntimeEvent::Resume)`). This fix does not change that.

## Hostile / Invalid Input Hazards

- **Malicious timestamp:** an attacker who controls the `u64` value (currently they don't, because `current_timestamp()` is internal) could try `u64::MAX` to trigger overflow. The mapper surfaces this as `Err(ResumeTimestampOverflow { run, timestamp: u64::MAX })`. No panic, no clamp, no wrap.
- **Malicious seq:** an attacker who controls the `EventSeq` could try `EventSeq::MAX`. The mapper must not silently clamp this; it must pass it through. `JournalEvent::is_valid()` rejects `seq == EventSeq::MAX` later in the pipeline.
- **Boundary crossing of `chrono`:** `chrono::DateTime<Utc>` is a well-tested type. The mapper does not need to defend against malformed `DateTime<Utc>` values; only against the conversion inputs.

## Performance Hazards

- **Single-clone preservation:** the fix must not regress the single-clone invariant. The current code path clones once in `clone_for_dispatch` for all boundary events including `Resumed`. The fix preserves this shape.
- **Conversion cost:** `i64::try_from(u64)` is a single comparison + branch. `DateTime::<Utc>::from_timestamp(i64, 0)` is constant-time. The mapper introduces negligible overhead per resume event. Resumes are not in the hot path (they are operator-driven or retry-driven, not steady-state).
- **No new allocations:** the mapper produces a `DateTime<Utc>` (size: 8 bytes via chrono representation) inside the `JournalEvent::RunResumed` value. No heap allocations are introduced.

## Release / API Hazards

- **New `RuntimeError` variant:** adding `RuntimeError::ResumeTimestampOverflow { run, timestamp }` is a public runtime surface change. Existing callers that match on `RuntimeError` exhaustively (if any) will need to add the new arm. Tests must cover both the success and overflow paths.
- **Coupling with vb-edvbj:** STRONG-coupled release. Both beads must land together. If vb-edvbj lands first, the dispatch loses its fallback and `Resumed` becomes a compile error. If vb-cib14 lands first, the dispatch produces `RunResumed` but the fallback still exists for any other future variant (no harm).
- **Public API symmetry:** the existing `RuntimeError` enum is `non_exhaustive` (assumed based on Rust convention); if it is not `non_exhaustive`, adding a new variant requires a `#[non_exhaustive]` attribute. The implementation agent must verify and add the attribute if missing.
- **CLI/storage parity:** `vb_cli::lifecycle::resume` already writes `JournalEvent::RunResumed`. After this fix, both the runtime-driven and CLI-driven paths produce the same variant with the same shape. No CLI-side change is required.

## Remaining Illegal-State Risks

- **Existing `runtime_journal_event_resumed_has_correct_timestamp` test:** at `crates/vb_runtime/src/journal/tests/chunk_004.rs:152-157` it only tests `run_id()` on the runtime event, not the storage mapper. This test must be augmented (by the test-planner) with a new test that exercises `storage_event(RuntimeJournalEvent::Resumed { .. })` and asserts `JournalEvent::RunResumed` is produced with the correct `seq` and a chrono timestamp equal to `DateTime::<Utc>::from_timestamp(timestamp as i64, 0)`.
- **Existing 16-variant enumeration test:** `chunk_004.rs:1077-1090` enumerates all 16 variants of `RuntimeJournalEvent` to confirm `run_id()` covers them. After this fix, `storage_event` must cover all 16 variants explicitly. The test-planner should extend this enumeration to `storage_event` to enforce totality.
- **Future variants:** once vb-edvbj removes the catch-all, adding any new `RuntimeJournalEvent` variant becomes a compile-time error unless explicitly mapped. This is the desired enforcement but means future beads adding new variants must update the dispatch helpers.

## Coupled-Bead Hazards

- vb-edvbj deletes the `Ok(JournalEvent::RunFailedEvent { .. })` fallback at `chunk_002.rs:298–302`. If vb-edvbj lands first:
  - Compile-time error: match-exhaustiveness on the `_ =>` arm catches `Resumed`.
  - The error must surface during `moon run :lint-src` and the proof-writing phase.
- If vb-cib14 lands first without vb-edvbj:
  - Runtime correctly produces `RunResumed` for `Resumed`.
  - The fallback remains for any future variant.
  - No behavior regression; the bug is fixed.
- The STRONG-coupled release order is "vb-cib14 first, vb-edvbj second" so that the catch-all is only removed once the explicit mapping is in place.