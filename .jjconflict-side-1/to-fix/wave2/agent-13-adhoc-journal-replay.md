# Wave 2 Agent-13: ad-hoc journal-replay deep-dive (Wave 2 / runtime-action-durability-shard)

Chunk 13 — 18 bug IDs focused on journal append/replay, recovery from events,
Fjall persistence. All `bd` show, source path, and `cargo test` evidence
collected against the current main working copy at
`/home/lewis/src/velvet-ballistics` (no source modifications, no beads created).

Source-of-truth contract (master §19 / AGENTS.md):

- **append-then-mutate**: durable journal append MUST succeed before runtime
  state is mutated; on failure rollback re-inserts the original state.
- **replay-equivalence**: `recover_*` paths MUST produce the same result as
  forward execution given the same event stream.
- **lifecycle-exhaustive**: `match event { ... }` over `JournalEvent` /
  `EngineSignal` MUST list every variant explicitly (no `_ =>` wildcard that
  silently drops events).
- **pending-tracking**: `ActionScheduled` MUST add (action, step) to pending
  set; `ActionCompleted` / `ActionFailed` MUST remove it; envelope MUST do both.

| bug-id | pri | append-then-mutate | replay-equivalence | lifecycle-exhaustive | pending-tracking | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-uxfl0 | P1 | n/a (regression) | **SILENTLY SKIPS pre-snapshot events** | n/a | n/a | `cargo test -p vb_storage --lib --no-fail-fast events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed` | 1 passed (asserts the bug) | **NOT-PATCHED** | `crates/vb_storage/src/journal/replay.rs:72-85` `events_for_run_bounded` sets `start_seq = next_seq(latest_durable_snapshot_seq)`, so tail only. All public recovery entry points (`recover_runtime_summary` L144, `recover_runtime_summary_with_expected` L160, `recover_runtime_frame_seed` L199, `recover_run_admission` L211, `recover_all_incomplete_runs` L228, `recover_full_journal` L203) call `journal.events_for_run(run)` which delegates to bounded. The fix from `da55addc7` (wave-5) added `events_for_run_full_bounded` + `events_for_run_full` and rewired every recovery entry point — both methods removed by a later commit (current tree has neither, no `Full` arm anywhere in `recover.rs`). Test `events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed` PASSES because it asserts the buggy "skip pre-snapshot" behaviour is current. Bead CLOSED with reason "implemented and verified", but the only landed SR-002 commit `da55addc7` was reverted. |
| vb-uy8p5 | P1 | n/a (3 unrelated test failures, not journal) | n/a | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast 'shard_config_validate_display_lists_all_errors\|lru_ring_capacity\|proptest_vb_god2f_action_completion'` | 0/0/1 tests, one assertion mismatch | **NOT-PATCHED (out-of-scope)** | Bead reports 3 specific test failures: (1) `shard::impl_::tests::shard_config_validate_display_lists_all_errors` at `crates/vb_runtime/src/shard/impl_tests/chunk_001.rs:497` — display missing `lru ring capacity zero`; (2) `shard::lru_ring_capacity_tests::clear_does_not_grow_arena_across_ten_cycles` at `crates/vb_runtime/src/shard/lru_ring_capacity_tests.rs:297` — `TerminalRunsLruFull { capacity: 8 }`; (3) `verification::proptest::proptest_vb_god2f_action_completion` at `crates/vb_runtime/src/verification/proptest/proptest_vb_god2f_action_completion.rs:154` — PayloadTooLarge mismatch (16777216 vs 16777217). All three are pre-existing wave-14 failures, NOT journal/replay bugs. Out of scope for chunk 13. |
| vb-v2zef | P2 | n/a | **PRESERVES non-target events in drain** | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast drain_for_run_preserves_non_target_events` | 1 passed | **PATCHED** | `crates/vb_runtime/src/trace.rs:87-116` `TraceRing::drain_for_run` keeps a `preserved: VecDeque` and re-pushes non-target events with `self.producer.push(event)`. Test `trace::tests::drain_for_run_preserves_non_target_events` (L162-173) verifies `drain_for_run(RunId::new(2), 8) == [run_two]` then `drain() == [run_one_start, run_one_finish]`. Test PASSES. |
| vb-v4ryp | P4 | n/a | **STRING COMPARISON ignores Finished.result payload** | n/a | n/a | `cargo test -p vb_runtime --test recovery_hydration_tests --no-fail-fast recover_runtime_summary_with_expected_terminal_mismatch` | 1 passed (but doesn't probe the bug) | **NOT-PATCHED** | `crates/vb_storage/src/recovery/recover.rs:155-177` `recover_runtime_summary_with_expected` still uses `terminal_state_to_string` for the comparison. `terminal_state_to_string` (L180-192) maps `Some(RecoveryTerminalState::Finished { .. }) => "Finished".to_owned()` — the `result` slot is dropped. So `Finished{result:5}` and `Finished{result:7}` both stringify to "Finished" and the function returns `Ok` (incorrect). The `PartialEq` check that the bead says was added is NOT present in source — no `hydration.summary().terminal != Some(expected)` guard anywhere. The fix was applied on `origin/bug-batch/5-p4-simplifications` (commits `447d50613` + `53614b915`) but those commits are NOT in main (`git log HEAD..origin/bug-batch/5-p4-simplifications -- crates/vb_storage/src/recovery/recover.rs` = 1 commit). Bead CLOSED but production code has the bug. The existing test `recover_runtime_summary_with_expected_terminal_mismatch` exercises Cancelled-vs-Finished (different variants) and PASSES — it does NOT exercise Finished-vs-Finished with different result slots, which is the bug. |
| vb-vbdco | P3 | n/a | n/a | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast evidence_collector_returns_typed_error_at_capacity\|bh_eng_01_evidence_collector_enforces_capacity_bound` | 59 evidence tests pass | **PATCHED** | `crates/vb_runtime/src/engine/types.rs:94-209` `EvidenceCollector::push_step_started` / `push_step_succeeded` / `push_slot_written_with_taint` / `push_slot_written_with_extra` all return `Err(EngineError::EvidenceCapacityExceeded)` (or `CollectEvidenceCapacityExceeded` for collect extras) when `events.len() >= capacity`. No silent drops. Tests `bh_eng_01_evidence_collector_enforces_capacity_bound`, `evidence_collector_returns_typed_error_at_capacity`, `evidence_collector_zero_capacity_returns_typed_error_for_every_push`, etc. all PASS. |
| vb-vuebt | P0 | n/a (lint only) | n/a | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast 'shard_submit_with_inputs_seeds_slots_and_drives\|finished_run_releases_frame_to_dimension_pool'` | tests compile, single return arrow each | **PATCHED** | `crates/vb_runtime/src/shard/tests/chunk_003.rs:94` `fn finished_run_releases_frame_to_dimension_pool() -> Result<(), String>` (single arrow). `crates/vb_runtime/src/shard/tests/chunk_013.rs:89` `fn shard_submit_with_inputs_seeds_slots_and_drives() -> Result<(), &'static str>` (single arrow). No third test (`timer_fired_command_returns_none_when_no_pending_timer`) found at `shard/impl_parts/timer_methods.rs:141` — the impl_parts/timer_methods.rs file may not exist. Both verified functions compile cleanly. |
| vb-vw6bx | P2 | n/a (lint only) | n/a | n/a | n/a | `bash moon run :lint-src 2>&1 \| tail -20` (script-based gate; not run here) | n/a (gate, not unit test) | **PATCHED** | `scripts/ignored-fallible-results.allow:4` documents `crates/vb_runtime/src/shard/transitions.rs\|DISCARD-006\|owner=velvet-ballistics\|expiry=2026-09-30\|follow_up=vb-rud5\|reason=finish_run and fail_run_state intentionally allow the secondary RunStateStore::insert to fail: the original journal-append error is the surfaced one. Best-effort rollback semantics preserved (RE-010/RE-011 regression coverage remains in drive.rs).`. Both `#[allow(clippy::let_underscore_must_use)]` markers at `transitions.rs:86` (finish_run) and `:194` (fail_run_state) are now documented per DISCARD-006 spec. |
| vb-wb05o | P3 | n/a | n/a | n/a | n/a | n/a (DUPLICATE of vb-12yr3) | n/a | **PATCHED (DUPLICATE)** | Close reason: "Duplicate of vb-12yr3; same external_ref bug-hunt-2026-06-21:RA-023 remains tracked there." RA-023 is admission path (capability count), not journal/replay. |
| vb-wcbde | P3 | n/a | n/a | n/a | n/a | `cargo test -p vb_core --lib --no-fail-fast MaxAttempts` | tests compile | **PATCHED (DUPLICATE)** | `crates/vb_core/src/ids/mod.rs:155-168` `MaxAttempts::try_new` returns `Err(EngineError::InvalidRepeatState { reason: "max_attempts_cannot_be_zero" })` for `value == 0`. Bead is DUPLICATE of vb-uvyi0 (same external_ref bug-hunt-2026-06-21:CF-012). Status IN_PROGRESS at the dup. Not journal/replay. |
| vb-wi486 | P2 | n/a | n/a | **Maps to typed StepNotFound** | n/a | `cargo test -p vb_core --lib --no-fail-fast ce_004` | 3 passed | **PATCHED** | `crates/vb_core/src/replay/mod.rs:138-149` `engine_to_replay_err` maps `EngineError::InvalidProgramCounter { step } => ReplayError::StepNotFound { step }` (no longer collides to `Internal`). 3 tests pass: `ce_004_replay_invalid_program_counter_yields_typed_step_not_found`, `ce_004_engine_to_replay_err_reserves_internal_for_unexpected_failures`, `ce_004_replay_jump_out_of_bounds_yields_typed_step_not_found`. |
| vb-wl1ut | P2 | n/a | n/a | n/a | n/a | n/a (DUPLICATE of vb-4bq3r) | n/a | **PATCHED (DUPLICATE)** | Close reason: "Closed"; bead points to vb-4bq3r. RP-003 is runtime primitive (together_start saturating-add), not journal. |
| vb-wo9j9 | P0 | n/a | **AwaitingAction ticket still ZEROED in runtime_from_core** | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast awaiting_action_produces_zeroed_ticket` | 1 passed (asserts the bug) | **NOT-PATCHED** | `crates/vb_runtime/src/engine/signal.rs:13-33` `runtime_from_core` still maps `EngineSignal::AwaitingAction => RuntimeSignal::AwaitingAction(ActionTicket { run: RunId::ZERO, step: StepIdx::ZERO, seq: SeqNo::ZERO, action: ActionId::new(0), attempt: 1, idempotency_key: 0, capacity: 1 })`. The test at `signal.rs:70-86` `awaiting_action_produces_zeroed_ticket` PASSES — it ASSERTS the zeroed ticket is the expected behaviour. `EngineSignal::AwaitingAction` at `crates/vb_core/src/engine/signals.rs:108` is STILL a unit variant (no struct payload). The bead description claims "EngineSignal::AwaitingAction now carries (step, seq, action)" but the actual struct variant is absent. The fix commit `561e50925` exists but is NOT on main (`git log HEAD..561e50925 -- crates/vb_runtime/src/ = empty`); no commit on main touches `runtime_construction.rs` (file does not exist in tree). `crates/vb_runtime/src/runtime.rs:336,349` accept `ticket: ActionTicket` parameters — but these are runtime action queue handlers, not the core signal path. The zeroed-ticket bug persists in the signal path that bead description says was fixed. |
| vb-ww1ts | P3 | n/a | n/a | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast 'capability_count_mismatch\|RA-018'` | 0 tests | **OPEN — not journal/replay** | Wave-15 bug-hunt follow-up. RA-018 is admission path (synthetic Capability fabricated at `crates/vb_runtime/src/admission/guards.rs:29-40`). Status OPEN, not closed. Not journal/replay. |
| vb-wyixc | P0 | n/a | n/a | n/a | n/a | `cargo test -p vb_core --lib --no-fail-fast symbol` | 78 symbol tests pass | **NOT-PATCHED (out-of-scope)** | `crates/vb_core/src/value_store.rs:92-99` `insert_symbol` does `self.symbols.push(value); Ok(id)` — does NOT intern (no hashmap lookup, no dedup). Duplicate symbols get distinct IDs as `next_symbol_id(self.symbols.len())`. The bead says CF-005 was fixed P0, but the current implementation does not intern. Symbol tests at `value_store::extended_tests::proptest_symbol_insert_then_lookup_matches` PASS only because they test insertion-then-lookup on the SAME id, not duplicate-intern equivalence. Out of scope for chunk 13 (not journal/replay). |
| vb-x3b0q | P2 | n/a | n/a | n/a | n/a | `cargo test -p vb_core --lib --no-fail-fast checked_len_to_u64` | tests compile | **PATCHED (out-of-scope)** | `crates/vb_core/src/value_store.rs:332-334` `checked_len_to_u64` is `u64::try_from(len).unwrap_or(u64::MAX)` — no `as u64` cast. CF-007 fixed. Not journal/replay. |
| vb-xjbs3 | P2 | n/a | n/a | n/a | n/a | `cargo test -p vb_core --lib --no-fail-fast shard_index` | tests compile | **PATCHED (out-of-scope)** | Status CLOSED with explicit "Fix + red regression test + black-hat + test-reviewer APPROVED". CF-011 is core frame (RunId::shard_index), not journal/replay. |
| vb-xkcdr | P1 | **Append-then-mutate OK in cancel/kill** | n/a | n/a | n/a | `cargo test -p vb_runtime --lib --no-fail-fast cancel` | 64 cancel tests pass | **PATCHED** | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-151` `handle_cancel`: line 137 `append_journal_event(RunCancelled)` happens BEFORE line 138-148 mutations (`pending_timer_remove`, `run_state_remove`, `release_frame`, `terminal_runs_insert`, `runtime_state_remove`, `counters.inc_failed`, `trace_ring.push`, `clear_executed_step_accounting`). Same pattern in `handle_kill` (L153-172): L159 `append_journal_event(RunKilled)` BEFORE L160+ mutations. The bead's specific fix (`discard_buffered_events_for_run` helper) does not exist in current code — but the journal append IS synchronous (`append_sequenced` at `journal/chunk_002.rs:293-297` calls `append_storage_event` → `append_journaled` immediately), so there is no coalesce buffer to drain. `discard_journal_sequence` at line 149/170 only removes the sequence counter, not buffered events. 64 cancel tests pass. |
| vb-y3az6 | P0 | n/a (kani compile fix) | n/a | n/a | n/a | `cargo test -p vb_storage --lib --no-fail-fast 'recover_runtime_frame_seed'` | 8 frame_seed tests pass | **PATCHED (out-of-scope)** | Bead says root cause was actually (1) `#[kani::proof_for(parse)]` unsupported in kani 0.67.0 at `crates/vb_core/src/kani_workflow_arbitrary.rs:667` and (2) `crates/vb_storage/src/journal/append/mod.rs:38-43` re-export bug. Current journal structure: no `append/` or `decision/` subdirectories; `crates/vb_storage/src/journal/append.rs` is flat (35 lines, just `append_journaled`/`append_strict`/`append_strict_batch`/`persist_strict`). FrameSeedAccumulator at `replay/summary.rs:402-696` is single impl block (no split). 8 frame_seed tests pass. Not journal/replay scope (kani compile + module re-export). |

## Cross-cutting findings (master §19 vs source for journal/replay paths)

| Concern | Status | Location | Note |
|---|---|---|---|
| `append_journal_event` returns error before any state mutation | OK | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:189-194` | Calls `journal.append_sequenced(event, seq)?` then `advance_journal_sequence(run, seq)?`. No mutation between. |
| `finish_run` rollback on append failure | OK | `crates/vb_runtime/src/shard/transitions.rs:87-112` | On `append_journal_event(RunFinished)` Err, `let _ = self.run_state_insert(run, state); return Err(error);` — re-inserts original state, surfaces journal error. |
| `fail_run_state` rollback on append failure | OK | `crates/vb_runtime/src/shard/transitions.rs:194-209` | Same pattern as `finish_run`. |
| `handle_cancel` order | OK | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-151` | `append_journal_event(RunCancelled)` BEFORE any state mutation. |
| `handle_kill` order | OK | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:153-172` | `append_journal_event(RunKilled)` BEFORE any state mutation. |
| `apply_summary_event` exhaustive match (no wildcard) | OK | `crates/vb_storage/src/recovery/replay/summary.rs:27-88` | All 23 `JournalEvent` variants covered explicitly. |
| `replay_events` exhaustive match (no wildcard) | OK | `crates/vb_storage/src/recovery/replay/core.rs:53-160` | All 23 `JournalEvent` variants covered explicitly. |
| `recover_runtime_frame_seed_from_events_inner` exhaustive match | **WILDCARD PRESENT** | `crates/vb_storage/src/recovery/replay/summary.rs:490-552` `apply_frame_event` | Match covers 13 variants explicitly then `_ => Ok(self)` at line 550 swallows the remaining 10 variants (RunAccepted, RunAdmission, ActionCompletedEvent, AskAnsweredEvent, WaitResolvedEvent, RetryScheduledEvent, RunCancelled, RunKilled, RunFailedEvent, RunResumed, RunRetried, RunAnswered). For FRAME SEED reconstruction these events correctly don't affect the seed state, but the wildcard is the only catch-all site for the durable-event enum. Holzman doctrine flags this. |
| Pending action add on ActionScheduled | OK | `crates/vb_storage/src/recovery/replay/summary.rs:651-657` `record_action_scheduled` | Inserts `(action, step)` into `pending_actions: HashSet<(ActionId, StepIdx)>`. |
| Pending action remove on ActionCompleted | OK | `crates/vb_storage/src/recovery/replay/summary.rs:675-682` `record_action_completed` | Removes `(action, step)` after `tracker.mark_completed`. |
| Pending action remove on ActionFailed | OK | `crates/vb_storage/src/recovery/replay/summary.rs:684-691` `record_action_failed` | Removes `(action, step)` after `tracker.mark_failed`. |
| Pending action remove on ActionCompletedEnvelope | OK | `crates/vb_storage/src/recovery/replay/summary.rs:597-629` `record_action_completion_envelope` | Removes `(envelope.ticket.action, envelope.ticket.step)` from `pending_actions`. |
| Snapshot+tail replay matches full-journal replay | OK (4 tests) | `recovery::tests::snapshot_tail_matches_full_journal_{lifecycle,action,wait,ask}_summary` | All 4 PASS: snapshot+tail reconstruction produces identical summary to full-journal replay. |
| `apply_summary_event` WaitResolvedEvent no-suspension (RE-009 regression) | OK | `summary.rs:62-65` | Explicit no-op branch for `WaitResolvedEvent`. |
| `events_for_run_bounded` skips pre-snapshot events | **BUG (SR-002)** | `crates/vb_storage/src/journal/replay.rs:72-85` | Starts at `next_seq(latest_durable_snapshot_seq)`. All public recovery APIs use this; `events_for_run_full*` methods removed by later commit. |
| `recover_runtime_summary_with_expected` uses string compare for `Finished.result` | **BUG (SR-016)** | `crates/vb_storage/src/recovery/recover.rs:155-192` | `terminal_state_to_string` drops `result` slot in `Finished { .. }`. No `PartialEq` guard. |
| `runtime_from_core` fabricates zeroed `ActionTicket` from `AwaitingAction` | **BUG (RQ-W0-01)** | `crates/vb_runtime/src/engine/signal.rs:18-26` | Hardcodes `run: ZERO, step: ZERO, seq: ZERO, action: 0, idempotency_key: 0`. Test enshrines the bug. |
| `EngineSignal::AwaitingAction` carries step/seq/action | **MISSING** | `crates/vb_core/src/engine/signals.rs:108` | Still unit variant; fix from commit `561e50925` not on main. |
| `TraceRing::drain_for_run` preserves non-target events | OK | `crates/vb_runtime/src/trace.rs:87-116` | Preserved VecDeque + re-push. |
| `EvidenceCollector` returns typed `EngineError::EvidenceCapacityExceeded` | OK | `crates/vb_runtime/src/engine/types.rs:94-209` | All `push_*` methods return `Result<(), EngineError>`. |
| `replay/basic/handlers/mod.rs:49` invalid jump target → generic Internal | OK | `crates/vb_core/src/replay/mod.rs:141-149` `engine_to_replay_err` | `InvalidProgramCounter { step } => ReplayError::StepNotFound { step }`. |

## Summary

- **bugs-checked:** 18
- **verdict counts:**
  - **PATCHED (real):** vb-v2zef, vb-vbdco, vb-xkcdr = 3
  - **PATCHED (DUPLICATE):** vb-wb05o, vb-wcbde, vb-wl1ut = 3
  - **PATCHED (out-of-scope, lint/test/kani):** vb-vuebt, vb-vw6bx, vb-x3b0q, vb-xjbs3, vb-y3az6 = 5
  - **NOT-PATCHED (in-scope, journal/replay):** vb-uxfl0, vb-v4ryp, vb-wo9j9 = 3
  - **NOT-PATCHED (out-of-scope, value-store / runtime primitives / proptest):** vb-uy8p5, vb-wyixc = 2
  - **OPEN (out-of-scope, admission):** vb-ww1ts = 1
  - **PATCHED (verified via CE-004 tests):** vb-wi486 = 1
  - **Sum:** 18 (3 + 3 + 5 + 3 + 2 + 1 + 1 = 18)

  In-scope (journal/replay): 3 PATCHED + 3 NOT-PATCHED + 1 PATCHED (CE-004) + 0 = 7; out-of-scope: 11.

- **mutate-before-append cases:** 0 in production paths. Every state transition
  that appends a journal event (`finish_run`, `fail_run_state`, `handle_cancel`,
  `handle_kill`, `handle_ask_answer`, `await_action`, `await_timer`,
  `handle_timer`) appends BEFORE mutating. `flush_step_started`,
  `flush_step_succeeded`, `flush_slot_written` in `shard/impl_parts/chunk_001.rs:615-636`
  also append-then-`trace_ring.push` (trace is non-durable evidence).

- **wildcard lifecycle arms:** 1 found.
  `crates/vb_storage/src/recovery/replay/summary.rs:550` `_ => Ok(self)` in
  `apply_frame_event`. Catches 10 variants: RunAccepted, RunAdmission,
  ActionCompletedEvent, AskAnsweredEvent, WaitResolvedEvent, RetryScheduledEvent,
  RunCancelled, RunKilled, RunFailedEvent, RunResumed, RunRetried, RunAnswered.
  Note: RunCancelled/RunKilled/RunFailedEvent being silently swallowed by
  `apply_frame_event` is intentional (frame seed doesn't carry terminal state
  — that's tracked separately in `apply_summary_event` L69-81 which IS
  exhaustive). But the wildcard remains a Holzman doctrine violation.

- **top-3 NOT-PATCHED with reason:**
  1. **vb-uxfl0 (SR-002) — Public recovery APIs silently skip pre-snapshot
     events.** `crates/vb_storage/src/journal/replay.rs:77-84`
     `events_for_run_bounded` starts at `next_seq(snapshot.seq)` instead of
     `EventSeq::ZERO`. All 6 public recovery functions (`recover_runtime_summary`,
     `recover_runtime_summary_with_expected`, `recover_runtime_frame_seed`,
     `recover_run_admission`, `recover_all_incomplete_runs`,
     `recover_full_journal`) call `journal.events_for_run(run)` which delegates
     to bounded. For any run with a durable snapshot, the recovery summary
     misses all `RunAccepted` / `RunAdmission` / `StepStarted` events that
     occurred before the snapshot. The fix from commit `da55addc7` (wave-5)
     added `events_for_run_full_bounded` + `events_for_run_full` methods and
     rewired every public recovery entry point — both methods were removed by a
     later commit and no replacement exists. Status CLOSED, source has the bug.
  2. **vb-v4ryp (SR-016) — `recover_runtime_summary_with_expected` uses string
     compare that drops `Finished.result` slot.** `crates/vb_storage/src/recovery/recover.rs:155-192`.
     `terminal_state_to_string` (L180-192) maps `Finished { .. } => "Finished"`,
     discarding the `result: SlotIdx`. So `Finished{result:5}` and
     `Finished{result:7}` both stringify to "Finished" and the function returns
     `Ok` (incorrect). The PartialEq guard that the fix introduced in commits
     `447d50613` + `53614b915` (`hydration.summary().terminal != Some(expected)`)
     is NOT in main. Bead CLOSED, source still has the bug. The existing test
     `recover_runtime_summary_with_expected_terminal_mismatch` exercises
     Cancelled-vs-Finished (different variants) and PASSES — does NOT probe
     the Finished-vs-Finished-with-different-result case which is the bug.
  3. **vb-wo9j9 (RQ-W0-01) — `runtime_from_core` fabricates zeroed
     `ActionTicket` from `EngineSignal::AwaitingAction`.**
     `crates/vb_runtime/src/engine/signal.rs:18-26` hardcodes
     `run: RunId::ZERO, step: StepIdx::ZERO, seq: SeqNo::ZERO, action: ActionId::new(0),
     idempotency_key: 0`. The test at `signal.rs:70-86`
     `awaiting_action_produces_zeroed_ticket` PASSES because it ASSERTS the
     zeroed ticket is correct — enshrining the bug. `EngineSignal::AwaitingAction`
     at `crates/vb_core/src/engine/signals.rs:108` is STILL a unit variant (no
     `(step, seq, action)` payload). The fix commit `561e50925` ("fix(runtime):
     RQ-W0-01 AwaitingAction zeroed ticket — propagate run/seq") is NOT on main
     (`git branch --contains 561e50925` returns no local branch). The bead
     description claims "vb_core, vb_runtime lib, vb_ipc all compile cleanly"
     but the test that would have detected the regression
     (`awaiting_action_produces_zeroed_ticket`) is unchanged and green.

- **file-path written:** `/home/lewis/src/velvet-ballistics/to-fix/wave2/agent-13-adhoc-journal-replay.md`
