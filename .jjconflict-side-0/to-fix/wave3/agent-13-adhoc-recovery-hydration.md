# Wave 3 / Agent 13 — Ad-hoc Recovery-Hydration-Expert Deep-Dive

Working dir: `/home/lewis/src/velvet-ballistics`
Chunk: 9 bug IDs (vb-tqz3v, vb-u1ezv, vb-uo52e, vb-uu31g, vb-uxfl0, vb-v4ryp, vb-w2wde, vb-wb05o, vb-whzz4)
Scope: recovery replay, pending-action hydration, lifecycle-state derivation, snapshot+tail validation
Mode: read-only, no bead creation, no source edits

## Summary Matrix

| bug-id | pri | pending-hydration | full-history-replay | cross-snapshot-validation | lifecycle-exhaustive | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-tqz3v (SA-001) | P1 | n/a (batch write) | n/a | n/a | n/a | `cargo test -p vb_storage --lib put_run_header` / `put_snapshot` | 3/3 + 3/3 pass | NOT-PATCHED | `batch.rs:123-134,137-148` use `encode_record(...)?` without `self.aborted = true`; compare `put_workflow_source` (`batch.rs:78-103`) which DOES set `aborted = true` on each failure arm |
| vb-u1ezv (SC-002) | P3 | n/a (codec) | n/a | n/a | n/a | `cargo test -p vb_storage --lib record_kind` (37 pass) | tests pass | NOT-PATCHED | `types.rs:75-94` — `EventSeq::new(value: u64)` accepts `u64::MAX`; no `try_new`; no `MAX_ENCODABLE` constant; `EventSeq::MAX = Self(u64::MAX)` declared at line 93 |
| vb-uo52e (SR-008) | P2 | n/a (digest gate) | n/a | n/a | n/a | `cargo test -p vb_storage --lib workflow_digest_rejection` | 1/1 pass | NOT-PATCHED | `recovery/replay/summary.rs:301-318` — `reject_workflow_digest_mismatch` returns `Ok(())` for empty events AND for events-without-RunAccepted (`.map_or(Ok(()), …)`); test at `summary/tests.rs:330` actively locks in the silent-pass behavior |
| vb-uu31g (SC-005) | P1 | n/a (trim) | n/a | n/a | n/a | `cargo test -p vb_storage --lib trim` | 38/38 pass | NOT-PATCHED | `trimming/logic.rs:269-307` (`check_retention_policy`) and `:325-349` (`compute_retained_terminal_runs`) call `has_terminal_event` per-run with no per-invocation HashMap memoization; close reason claimed memoization but it is absent |
| vb-uxfl0 (SR-002) | P1 | OK | FAIL | OK (pre-hydrate) | OK | `cargo test -p vb_storage --lib events_for_run_starts_after_snapshot` / `events_for_run_skips_corrupt_pre_snapshot` | 1/1 + 1/1 pass | NOT-PATCHED | `journal/replay.rs:72-85` (`events_for_run_bounded`) sets `start_seq = next_seq(snapshot.seq)` when a snapshot exists; pre-snapshot events are skipped; `recovery/recover.rs:140-216` (`recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_run_admission`) operate on tail events only — summary counters and pending-actions hydration are computed from the tail slice alone |
| vb-v4ryp (SR-016) | P4 | n/a | n/a | n/a | n/a | `cargo test -p vb_storage --lib terminal_state_mismatch` | 2/2 pass | NOT-PATCHED | `recovery/recover.rs:165-192` still routes through `terminal_state_to_string`; error variant `TerminalStateMismatch { expected: String, found: String }` (`recovery/types.rs:115-122`) holds strings; the partial-equal variant still exists but is unused on this path; close reason in `53614b915` only trimmed dead imports, comparison remains string-based |
| vb-w2wde (P0) | P0 | n/a | n/a | n/a | n/a | `cargo test -p vb_storage --lib bounded_scan` (0 names) + grep audit | audit pass | PATCHED | `journal/replay.rs:30-49` (`classify_replay_push_len` allocation-free classifier) and `:136-160` (`push_replay_event` uses `try_reserve(1)` then `push`); no `Vec::with_capacity(limit.max_events())` left on this path; overflow returns `JournalError::TooManyEvents` |
| vb-wb05o (RA-023) | P3 | n/a (admission) | n/a | n/a | n/a | `cargo test -p vb_runtime --lib admit_artifact_run` | 21/21 pass | PATCHED | `admission.rs:743-750` returns `AdmissionError::CapabilityCountMismatch { required_count, granted_count }` typed variant; per-capability `?` at line 742 still short-circuits but that is the documented order (under-grant first, cardinality gate second); closed as duplicate of vb-12yr3 |
| vb-whzz4 (BH-W0-S05) | P0 | n/a | n/a | n/a | n/a | `cargo test -p vb_storage --lib record_kind` | 37/37 pass | NOT-PATCHED | `records.rs:199-233` — `pub const fn id(self) -> u16` still enumerates 27 match arms duplicating the `#[repr(u16)]` discriminant values at `:139-197`; no `self as u16` replacement and no `RecordKind::WIRE` table; only behavioural change since close was adding two variants (`RunKilled = 28`, `AskTimedOut = 29`, `WaitResolved = 31`) which made the match arm count grow |

## Lifecycle arm coverage (recovery)

| function | exhaustive? | wildcards | location |
|---|---|---|---|
| `apply_summary_event` (summary.rs:27-87) | YES | none | every `JournalEvent` variant is named (incl. explicit `RunResumed`/`RunRetried`/`RunAnswered` group with comment, lines 82-86) |
| `apply_summary_event_checked` (summary.rs:154-228) | NO — has wildcard `_ => { apply_summary_event(summary, event); Ok(()) }` at line 223 | yes | wildcard covers all action/scheduling events not handled by ticket/envelope logic — defensible because the inner `apply_summary_event` is already exhaustive |
| `FrameSeedAccumulator::apply_frame_event` (summary.rs:490-552) | NO — has wildcard `_ => Ok(self)` at line 550 | yes | silently no-ops `RunAccepted`, `RunAdmission`, `RunCancelled`, `RunKilled`, `RunFailedEvent`, `RetryScheduledEvent`, `AskAnsweredEvent`, `WaitResolvedEvent`, `RunResumed`, `RunRetried`, `RunAnswered`; intentionally correct for frame-seed purposes but masks any new lifecycle event variant added later |
| `derive_dimensions_from_snapshot_and_tail` (hydrate_support.rs:190-259) | NO — has wildcard `_ => {}` at line 236 | yes | silent skip of `RunAccepted`, `RunAdmission`, `RunCancelled`, `RunKilled`, `RunFailedEvent`, `AskAnsweredEvent`, `RunResumed`, `RunRetried`, `RunAnswered` |

## Pending-action hydration (Section 18)

Tracked across `FrameSeedAccumulator` (`summary.rs:401-460`):

- `ActionScheduled` → `pending_actions.insert((action, step))` (`:651-657`)
- `ActionScheduledTicket` with `effect == Apply` → `pending_actions.insert((ticket.action, ticket.step))` (`:659-673`)
- `ActionScheduledTicket` with `effect == Duplicate` → NOT inserted again, NOT removed (correct)
- `ActionCompletedEnvelope` (Apply effect) → `pending_actions.remove(&…)` (`:617-625`)
- `ActionCompletedEvent` → `pending_actions.remove(&…)` (`:675-682`)
- `ActionFailedEvent` → `pending_actions.remove(&…)` (`:684-691`)

Coverage gaps that hydration currently misses (the bug asked about):

1. **`RejectAction` / cancellation that resolves an outstanding scheduled action** — no event variant for "action cancelled without completion" exists; if it were added, `FrameSeedAccumulator::apply_frame_event` wildcard (`:550`) would silently swallow it.
2. **`ActionScheduledTicket` Duplicate without ever-seen completion** — the `mark_scheduled_ticket_effect` (types.rs:415-444) returns `Duplicate` only when the ticket is byte-identical to an existing schedule; the duplicate is then a no-op in both summary counters AND `pending_actions`. That is correct, but the `_ =>` wildcard at `summary.rs:550` is still the safety net.
3. **`StepFailed` / `RetryScheduled` / `WaitScheduled` interrupting a scheduled action** — these affect the step state but the action remains in `pending_actions` because no completion or failure event was issued. This is consistent with "action still pending", but if the runtime meant "action abandoned", the hydration produces a stale pending action. There is no explicit hydration assertion for this case.
4. **`summarize_recovery_events` does NOT publish `pending_actions`** — it returns `RecoveryHydration::Summary` only. The summary at `summary.rs:111-152` never populates `pending_actions`. The frame-seed path (`recover_runtime_frame_seed_from_events_inner` at `:320-367`) does publish them. So callers using the summary API alone have no visibility into pending action hydration.

## Cross-snapshot invariants

`snapshot+tail` validation lives in `hydrate.rs:202-218`:

| invariant | implemented? | evidence |
|---|---|---|
| snapshot.run == run_id | yes | `validate_snapshot_metadata` (`hydrate.rs:111-124`) |
| tail events all share run_id | yes | `validate_tail_events_match_run` (`hydrate.rs:220-229`) |
| tail seq strictly > snapshot.seq | yes | `validate_tail_events_after_snapshot` (`hydrate.rs:231-240`) |
| non-empty recovery data | yes | `validate_recovery_data_present` (`hydrate.rs:154-165`) |
| snapshot workflow digest == accepted workflow | NO | no digest check in `validate_snapshot_recovery_inputs`; the accepted workflow digest is checked separately by `reject_workflow_digest_mismatch` (`summary.rs:301-318`) which is itself the subject of SR-008 |
| tail digest continuity (no skips/gaps) | YES at journal layer | `validate_replay_sequence` (`journal/replay.rs:122-134`) |
| snapshot payload digest integrity | YES | `events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay` (`journal/tests.rs:1923-1965`) |
| cross-snapshot pc / step / slot consistency across snapshot boundaries | NO | no cross-snapshot invariant check beyond "tail starts at next_seq(snapshot.seq)"; a run with snapshot at seq=10 and a tail event at seq=10 (corrupt) is rejected by `validate_tail_events_after_snapshot` but a run with snapshot at seq=10 and a tail event at seq=11 that disagrees with the snapshot's slot values is not cross-checked |

## Wildcard lifecycle arms (exhaustive-risk list)

The following wildcards could silently absorb a new `JournalEvent` variant added in the future:

- `recovery/replay/summary.rs:550` — `FrameSeedAccumulator::apply_frame_event` `_ => Ok(self)`
- `recovery/replay/summary.rs:223` — `apply_summary_event_checked` `_ => { apply_summary_event(summary, event); Ok(()) }`
- `recovery/hydrate_support.rs:236` — `derive_dimensions_from_snapshot_and_tail` `_ => {}`
- `events.rs:429` — `JournalEvent::slot_value` `_ => Ok(None)` (justified; only `SlotWrittenEvent` carries a slot value)

Each of these is a soft surface where adding a new event variant will not produce a compile error.

## Counts

- bugs-checked: 9
- PATCHED: 2 (`vb-w2wde`, `vb-wb05o`)
- NOT-PATCHED: 7 (`vb-tqz3v`, `vb-u1ezv`, `vb-uo52e`, `vb-uu31g`, `vb-uxfl0`, `vb-v4ryp`, `vb-whzz4`)
- PARTIAL: 0
- UNKNOWN: 0

## Top-3 NOT-PATCHED with reason

1. **vb-uo52e / SR-008 — `reject_workflow_digest_mismatch` still passes silently when no `RunAccepted` event exists.** The function at `recovery/replay/summary.rs:301-318` still does `find_map(...).map_or(Ok(()), …)`, which returns `Ok(())` whenever the events iterator yields no `RunAccepted`. The test at `recovery/replay/summary/tests.rs:330` actively pins the buggy behavior with `assert_eq!(reject_workflow_digest_mismatch(&[], expected).ok(), Some(()));`. Fix would replace the wildcard with an explicit `events.iter().any(matches!(…, RunAccepted {..}))` guard returning `RecoveryError::ReplayDivergence` when no acceptance evidence exists.

2. **vb-v4ryp / SR-016 — `recover_runtime_summary_with_expected` still compares terminal states via string.** At `recovery/recover.rs:166-174` both sides are converted via `terminal_state_to_string` (`:180-192`) and compared as `String`. The error variant `RecoveryError::TerminalStateMismatch { expected: String, found: String }` (`recovery/types.rs:115-122`) keeps both fields as strings, so even though `RecoveryTerminalState` derives `PartialEq` (`:135`), the production path never uses it. The 2026-06-23 wave-13 commit `53614b915` claimed to land the partial-equal comparison but only deleted an unused `RecoveryTerminalState` import — the actual function body still stringifies.

3. **vb-uxfl0 / SR-002 — recovery APIs still consume only the post-snapshot tail.** `journal/replay.rs:72-85` (`events_for_run_bounded`) sets `start_seq = next_seq(snapshot.seq)` whenever a durable snapshot exists, and the recovery orchestrator at `recovery/recover.rs:140-216` calls `journal.events_for_run(run)` and feeds the result straight into `summarize_recovery_events` / `recover_runtime_frame_seed_from_events`. Both summary counters (`steps_started`, `slots_written`, …) and the `pending_actions` HashSet at `summary.rs:408` are populated only from the tail slice, so any pre-snapshot evidence present in storage is dropped. The two existing tests `events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed` (`journal/tests.rs:1759`) and `events_for_run_skips_corrupt_pre_snapshot_event_by_key_range` (`:2002`) lock the skip into the contract; the fix would have to either (a) start at seq 0 and require the snapshot to be decoded into the hydration product, or (b) require pre-snapshot events to be physically trimmed before replay is allowed.

## Other NOT-PATCHED (single-line reason each)

- vb-tqz3v / SA-001: `batch::put_run_header` and `batch::put_snapshot` propagate `encode_record` errors via `?` without setting `self.aborted = true` (`batch.rs:123-148`), unlike `put_workflow_source` (`:78-103`).
- vb-u1ezv / SC-002: `EventSeq::new` is unchanged; no `try_new` and no `MAX_ENCODABLE` (`types.rs:75-94`); the decoder-side `next_seq` (`codec/mod.rs:141-146`) returns `SequenceOverflow` but the constructor still admits `u64::MAX`.
- vb-uu31g / SC-005: no `HashMap` memoization exists in `compute_retained_terminal_runs` (`trimming/logic.rs:325-349`) nor in `check_retention_policy` (`:269-307`); the per-run `has_terminal_event` is invoked directly inside the loop.
- vb-whzz4 / BH-W0-S05: `RecordKind::id` (`records.rs:202-232`) still enumerates 27 explicit match arms duplicating the `#[repr(u16)]` discriminants at `:139-197`; no `self as u16` and no `RecordKind::WIRE` table.

## Targeted test commands and results

```text
cargo test -p vb_storage --lib workflow_digest_rejection       # SR-008 — 1/1 pass (locks in buggy behavior)
cargo test -p vb_storage --lib frame_seed_with_workflow        # SR-002 — 4/4 pass
cargo test -p vb_storage --lib terminal_state_mismatch         # SR-016 — 2/2 pass
cargo test -p vb_storage --lib events_for_run_starts_after_snapshot  # SR-002 — 1/1 pass (locks in skip behavior)
cargo test -p vb_storage --lib events_for_run_bounded          # SR-002 — 1/1 pass (overflow guard)
cargo test -p vb_storage --lib events_for_run_skips_corrupt_pre_snapshot  # SR-002 — 1/1 pass (locks in skip behavior)
cargo test -p vb_storage --lib trim                            # SC-005 — 38/38 pass (correctness only; perf not measured)
cargo test -p vb_storage --lib put_run_header                  # SA-001 — 3/3 pass (positive path only; no failure path)
cargo test -p vb_storage --lib put_snapshot                    # SA-001 — 3/3 pass (positive path only; no failure path)
cargo test -p vb_storage --lib record_kind                     # SC-002 / BH-W0-S05 — 37/37 pass (exercises the duplicated match table)
cargo test -p vb_runtime --lib admit_artifact_run              # RA-023 — 21/21 pass
cargo test -p vb_storage --lib recovery                        # full recovery corpus — 213/213 pass
```

All test paths compile and pass; the gaps are in the negative / fail-closed coverage that the bugs require.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-13-adhoc-recovery-hydration.md`
