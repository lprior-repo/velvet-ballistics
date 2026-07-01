# Codebase Map — vb-cib14

- bead_id: vb-cib14
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
- jj_workspace: cheap25-vb-cib14
- captured_at: 2026-07-01
- controller: femdation
- bead_summary: RuntimeJournalEvent::Resumed falls through to a catch-all in
  `StorageRuntimeJournal::storage_event` that rewrites the resume event as a
  synthetic `JournalEvent::RunFailedEvent`. The real `JournalEvent::RunResumed`
  mapping is missing. Once vb-edvbj deletes the catch-all, this path becomes a
  hard compile/dispatch failure. The fix wires `Resumed` to the
  `JournalEvent::RunResumed` arm.

## Hot spot — the catch-all bug

- `crates/vb_runtime/src/journal/chunk_002.rs`
  - `StorageRuntimeJournal::storage_event` (lines 270–303)
    - Top-level match (lines 274–293) dispatches to `run_storage_event`,
      `action_storage_event`, or `boundary_storage_event` via a `_` arm at
      line 293 (the `_` arm IS the catch-all being deleted in vb-edvbj).
    - After dispatch, the `if let Some(storage_event) = result { … }` (lines
      295–297) returns the mapped event only if the dispatch arm produced
      `Some(...)`. Otherwise it falls through to a *second* catch-all at
      lines 298–302:
      ```rust
      Ok(JournalEvent::RunFailedEvent {
          run: event.run_id(),
          seq,
          attempt: 1,
      })
      ```
      This silently rewrites any unmatched `RuntimeJournalEvent` (including
      `Resumed`) as a `RunFailedEvent`. That is the P0 bug.
  - `boundary_storage_event` (lines 193–268): the `Resumed { .. }` arm at
    line 266 returns `Ok(None)`, which is what causes `Resumed` to fall
    through to the synthetic `RunFailedEvent`.
  - `run_storage_event` (lines 41–103): `Resumed` is correctly listed in the
    no-op catch-all (line 101), but `storage_event` never routes `Resumed`
    here, it routes via `boundary_storage_event` due to the `_ =>` arm.
  - `action_storage_event` (lines 105–191): same situation, `Resumed`
    appears in the no-op catch-all (line 189).

## Source-of-truth for the resume event

- `crates/vb_runtime/src/journal/chunk_001.rs`
  - `RuntimeJournalEvent::Resumed` (lines 188–194): defined as
    `{ run: RunId, timestamp: u64 }`.
  - `RuntimeJournalEvent::run_id()` (lines 200–224): `Resumed { run, .. }`
    arm at line 218.

- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
  - `Shard::handle_resume` (lines 307–331): public API entry point for
    resume; emits the `Resumed` journal event through `append_resumed_event`.
  - `Shard::append_resumed_event` (lines 344–358): constructs
    `RuntimeJournalEvent::Resumed { run, timestamp }` where `timestamp` is
    a `u64` seconds-since-epoch from `current_timestamp()` (defined in
    `shard/lifecycle/chunk_003.rs:24–28`).
  - `Shard::validate_run_exists` (lines 333–338),
    `Shard::get_runtime_state_or_running` (lines 340–342),
    `Shard::is_run_tracked` (lines 360–362),
    `Shard::observe_resume_drive_result` (lines 364–373),
    `Shard::restore_resumable_after_drive_failure` (lines 375–384): the
    surrounding resume-state plumbing. These do NOT need to change for
    this bead.

- `crates/vb_runtime/src/shard/transitions.rs`
  - `Shard::apply` (lines 50–76): `RuntimeEvent::Resume` (line 55) inserts
    `RuntimeState::Resuming`; `RuntimeEvent::ResumeRollback` (line 58)
    inserts `RuntimeState::Resumable`. Resume FSM plumbing, not directly
    part of this bead.

- `crates/vb_runtime/src/shard/types.rs`
  - `RuntimeState::{Resumable, Resuming}` (lines 750–764) and
    `RuntimeEvent::{Resume, ResumeRollback}` (lines 778–797).
  - `ResumeStatus::{Resumed, AlreadyRunning}` (lines 822–830).
  - `ResumeResult` (lines 833–841): includes `pub timestamp: u64`.
  - `ResumeError::{RunIdNotFound, NotResumable, IncompleteHydration,
    JournalAppendFailed, JournalAppendFailedWithSource, StructuredOutputFailed}`
    (lines 846–873).

## Target storage event

- `crates/vb_storage/src/events.rs`
  - `JournalEvent::RunResumed` (lines 289–297):
    `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }`.
  - `JournalEvent::record_kind()` (lines 401–429): `RunResumed` maps to
    `RecordKind::RunResumed` at line 424.
  - `JournalEvent::run_id()` (lines 336–363) and `JournalEvent::seq()`
    (lines 369–397): both include `RunResumed`.

## Recovery/replay expectations

- `crates/vb_storage/src/journal/incident.rs:203` —
  `JournalEvent::RunResumed { .. } => LifecycleState::Active`. If the
  runtime's `Resumed` is rewritten as `RunFailedEvent` (the current bug),
  recovery/replay marks the run as `LifecycleState::Failed` instead of
  `Active`. This is the user-visible failure.

- `crates/vb_storage/src/recovery/hydrate.rs:754` —
  `JournalEvent::RunResumed { .. } => Ok(false)` in
  `is_in_flight_or_completed` style projection.

- `crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126` —
  `RunResumed` is explicitly listed in the recovery observation classifier.

- `crates/vb_storage/src/recovery/replay/summary/apply.rs:79–81` —
  `RunResumed` is one of the lifecycle events without a sequence number
  history concern at the summary level.

- `crates/vb_storage/src/recovery/recovery_unit_tests.rs:762,1208` —
  unit tests already exercise `JournalEvent::RunResumed` as a distinct
  variant.

- `crates/vb_storage/src/journal/regression_tests_vb_1rqz7.rs:160,523` —
  regression test scaffolding for `RunResumed` event round-trip.

## CLI surface that already uses RunResumed correctly

- `crates/vb_cli/src/lifecycle.rs:150–220` — `resume()` writes
  `JournalEvent::RunResumed { run, seq: next_seq, timestamp: Utc::now() }`
  directly to the journal (no runtime shard involvement). It demonstrates
  the expected seq + `DateTime<Utc>` shape. Tests in
  `crates/vb_cli/tests/lifecycle_integration.rs:289–334` (`resume_succeeds_when_bead_is_cancelled`)
  assert `JournalEvent::RunResumed` is appended.

- `crates/vb_cli/src/commands_journal.rs:264`,
  `crates/vb_cli/src/status.rs:417`,
  `crates/vb_cli/src/commands_diff.rs:205,236`,
  `crates/vb_cli/src/events.rs:171` — match `JournalEvent::RunResumed`
  arms; these already assume the variant exists.

- `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:225` —
  maps `JournalEvent::RunResumed` to `"RunResumed"`.

## Existing tests covering the runtime resume path

- `crates/vb_runtime/tests/durable_resume_red_phase.rs`
  - Line 332–349 `resume_post001_journal_appended_before_success`: asserts
    that `RuntimeJournalEvent::Resumed` appears in a `VolatileRuntimeJournal`
    snapshot after `handle_resume`. Does NOT exercise
    `StorageRuntimeJournal::storage_event`.
  - Line 345, 476: matches against `RuntimeJournalEvent::Resumed { run, .. }`.

- `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`
  - Lines 17–28 file-level docstring documents the resume error-propagation
    contract.
  - Lines 192, 250, 265: resume `Resumed` append failure scenarios.

- `crates/vb_runtime/src/journal/tests/chunk_004.rs`
  - Line 155 `runtime_journal_event_resumed_has_correct_timestamp`: only
    tests `run_id()` on the runtime event; does NOT exercise
    `StorageRuntimeJournal::storage_event`.
  - Line 1077 `RuntimeJournalEvent::Resumed` is part of the
    "all-16-variants" run_id enumeration.
  - Line 1090 `assert_eq!(events.len(), 16, "all 16 event variants must be covered");`.

- `crates/vb_runtime/src/journal/tests/chunk_002.rs`
  - `storage_runtime_journal_maps_action_wait_and_ask_events`
    (lines 1–145): exercises boundary events through
    `StorageRuntimeJournal::append_sequenced`, but does NOT include
    `Resumed`.
  - `re_009_wait_resolved_maps_to_dedicated_journal_event`
    (lines 150–195): regression test for `WaitResolved` mapping. Sets the
    pattern for a `Resumed` regression test.
  - `storage_event_clones_the_event_exactly_once_per_dispatch`
    (lines 410–493): clone-counting regression test. Exercises one variant
    from each of the three dispatch arms but does not cover `Resumed`.

- `crates/vb_runtime/src/shard/tests/chunk_004.rs:153`,
  `chunk_006.rs:62`, `chunk_009.rs:171`, `chunk_013.rs:305`,
  `chunk_016.rs:173`: shard-level resume tests; per
  `verification/tla/rust-refinement-obligations.jsonl` (RRO-TLA-RESUME-001),
  these are the behavior-test references for the resume state-machine
  refinement obligation.

## Verification / proof artifacts

- `verification/tla/rust-refinement-obligations.jsonl:6` —
  RRO-TLA-RESUME-001 ties `Shard::handle_resume/append_resumed_event` to
  `specs/ResumeStateMachine.tla`. Source refs cite
  `shard/lifecycle/chunk_001.rs:291-367`. `mapping_status: partial`
  because the refinement harness is empty.
- `verification/tla/proof-to-rust-map.md:47` — same obligation table.
- `verification/verus/extern_storage_kind_family.rs:200,235,370,417,449,487,492,525,579,611` —
  mirror of `JournalEvent::RunResumed` for Verus spec coverage.
- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:366,616,617,715,748,792,839` —
  Verus mirror of `RunResumed`.
- `verification/verus/vb-vzcuf-PS-003.rs:136`,
  `verification/verus/storage_kind_family.rs:492,525,579,611` — additional
  Verus spec mentions.

## Adjacent files / context

- `crates/vb_runtime/src/journal.rs` — `journal.rs:1–14` includes the
  three journal chunks plus the `tests` module.
- `crates/vb_runtime/src/journal/chunk_003.rs` — `QueuedStorageRuntimeJournal`
  also calls `StorageRuntimeJournal::storage_event(event, seq)?` at line 12,
  so the same catch-all bug also affects the queued adapter.
- `crates/vb_runtime/src/shard/mod.rs` — re-exports `ResumeError,
  ResumeResult, ResumeStatus` at line 24.
- `crates/vb_runtime/src/shard/lifecycle.rs` — pulls in
  `lifecycle/{chunk_001.rs,chunk_002.rs,chunk_003.rs,chunk_004.rs}` and
  the `lifecycle_tests/*` test modules.
- `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:24–28` —
  `current_timestamp()` returns `u64` seconds; this is the source of the
  `Resumed.timestamp` value that needs to be converted to
  `DateTime<Utc>` when mapped to `JournalEvent::RunResumed`.
- `crates/vb_runtime/Cargo.toml:9` — `chrono.workspace = true` (chrono is
  already a direct dependency of `vb_runtime`).
- `crates/vb_storage/src/events.rs:5` — `use chrono::{DateTime, Utc};`
  (chrono is already imported in `vb_storage`).

## Risk surface

- Risk that the fix accidentally drops the `EventSeq` if `Resumed` is
  routed through `run_storage_event` or `action_storage_event` rather than
  receiving a dedicated arm in `storage_event`. Both `run_storage_event`
  and `action_storage_event` discard the seq in their catch-all `None`
  paths (they return `Option<JournalEvent>`, not
  `RuntimeResult<JournalEvent>`), so any `Resumed` mapping must live in
  `storage_event` itself OR `boundary_storage_event` must return
  `Ok(Some(JournalEvent::RunResumed { .. }))` rather than `Ok(None)`.
- Risk that the `u64` timestamp from `current_timestamp()` overflows
  `chrono::DateTime<Utc>::from_timestamp` (i64 seconds). For realistic
  Unix timestamps (currently ≈ 1.7e9) this is not a concern, but a typed
  conversion path is required because the existing source is `u64` and
  `DateTime<Utc>::from_timestamp` takes `i64`. Need
  `i64::try_from(timestamp)` with explicit overflow handling.
- Risk that adding a `Resumed` arm under `run_storage_event` or
  `action_storage_event` breaks the single-clone invariant asserted by
  `storage_event_clones_the_event_exactly_once_per_dispatch` at
  `crates/vb_runtime/src/journal/tests/chunk_002.rs:410–493`. The fix
  should keep the dispatch-on-`&event` + single `clone_for_dispatch`
  shape.
- Risk that the change is silently neutralized by the
  `JournalEvent::RunResumed { .. } => Ok(false)` arm in
  `crates/vb_storage/src/recovery/hydrate.rs:754` if hydrate's
  classification depends on whether the event was stored as `Resumed` or
  as the bug's `RunFailedEvent`. The hydrate function currently treats
  `RunResumed` as NOT in-flight-or-completed, which is the correct
  classification; the bug causes a `RunFailedEvent` write which would
  flip it to "completed".
- Risk of double-emit if `append_resumed_event` is called twice for the
  same run after a rollback. Already handled at the runtime-state level
  (`Resuming` is the in-flight state); the storage mapping change does not
  affect that.

## Open questions for downstream stages

- Where should the new `Resumed => RunResumed` arm live? Two reasonable
  options: (a) add an explicit `Resumed` arm to `boundary_storage_event`
  returning `Ok(Some(JournalEvent::RunResumed { run, seq, timestamp }))`,
  or (b) add an explicit `Resumed` arm directly in `storage_event` after
  the dispatch. Option (a) is consistent with the existing pattern (e.g.
  `WaitScheduled => WaitScheduledEvent`). Option (b) is more localized.
- Is `seq` for `RunResumed` always derived from the per-run sequence
  owned by the shard, or can it come from the runtime event? Per
  `crates/vb_storage/src/events.rs:367–368` ("Lifecycle events
  (RunResumed, RunRetried, RunAnswered) now carry sequence numbers"),
  yes — `RunResumed` carries the shard-owned `EventSeq`.
- Should `u64` seconds-since-epoch be `chrono::DateTime<Utc>::from_timestamp`
  with explicit `i64::try_from`, or should the runtime event signature
  change to carry a `DateTime<Utc>`? Existing `RuntimeJournalEvent::Resumed`
  uses `u64`, so the conversion belongs in the storage mapper, not in
  the runtime event definition.
- Does any existing behavior test rely on `Resumed` being silently
  rewritten as `RunFailedEvent`? None found in `crates/vb_runtime/tests/`
  or `crates/workspace_tests/tests/`. The only related tests are the
  `lifecycle_integration.rs` resume test which already uses
  `vb_cli::lifecycle::resume` (writes `RunResumed` directly) and the
  `durable_resume_red_phase.rs` POST-001 test which uses
  `VolatileRuntimeJournal` (no storage dispatch). So no test depends on
  the buggy catch-all behavior.
- Is `Verus` mirror at
  `verification/verus/extern_storage_kind_family.rs:370` and
  `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:617`
  in sync with the production `JournalEvent::RunResumed` shape?
  Production has `{ run, seq, timestamp }`; mirrors match. Drift risk is
  low.

## Open questions for the bead description

- "wire Resumed to the actual resume hydration path in the runtime shard
  module" — the runtime shard module already has the hydration path
  (`handle_resume` → `append_resumed_event`). The real fix is the
  *storage dispatcher* in
  `crates/vb_runtime/src/journal/chunk_002.rs`, not the runtime shard.
  The fix must produce a `JournalEvent::RunResumed` so that
  `crates/vb_storage/src/journal/incident.rs:203` classifies the event as
  `LifecycleState::Active` instead of the current bug producing
  `LifecycleState::Failed` via the synthetic `RunFailedEvent`. If the
  bead really means "wire Resumed to the hydration path", then this
  may be a documentation mismatch and the implementation agent should
  confirm with the controller before changing shard code.
- vb-edvbj ("the catch-all being deleted") has not been located in any
  artifact directory in `/home/lewis/src/isoloated/`. The reference may
  be in the source `velvet-ballistics` repo or in the dispatch
  controller. The visible catch-all is in
  `crates/vb_runtime/src/journal/chunk_002.rs:298–302`. If vb-edvbj is
  the bead that deletes this synthetic `RunFailedEvent` fallback, the
  current vb-cib14 fix MUST be in place first or downstream code will
  fail to compile (no exhaustive match arm for `Resumed` in the
  dispatchers).
