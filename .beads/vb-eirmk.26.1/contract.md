# Contract: vb-eirmk.26.1 — Integrate TimerWheel into shard pending-timer scheduling

## Identity and authority

- Bead: vb-eirmk.26.1, parent vb-eirmk.26, source baseline 23749f4b5b0b1822243a956ae22dd1b59912a0e1.
- Governing phase requirement: master phase 55 requires replacing IndexMap<RunId, PendingTimer> and its due-timer scan with production TimerWheel scheduling backed by BTreeMap<Instant, Vec<TimerEntry>>.
- This contract covers the demonstrated shard-aggregate integration gap; it does not claim a fairness or maximum timer-delay guarantee.
- Preserve the closed ownership/history of vb-99n6, vb-3n0rd, and vb-vi3g; reuse their lifecycle, deadline-unit, and existing wheel work without reopening or duplicating them.

## Baseline facts

- crates/vb_runtime/src/shard/types.rs defines public PendingTimer { step, kind, generation, deadline }; private RunAggregate currently stores IndexMap<RunId, PendingTimer>.
- crates/vb_runtime/src/shard/timer_wheel.rs already defines public TimerEntry { run, generation, deadline, kind } and dual indexes BTreeMap<Instant, Vec<TimerEntry>> plus a run map; insert computes generation internally and fire_expired removes every due entry.
- crates/vb_runtime/src/shard/impl_parts/chunk_001.rs::drive_one_due_timer scans pending_timer_iter, chooses minimum (deadline, RunId), and sends exactly one selected timer to handle_timer.
- Existing public shard timer methods expose PendingTimer, including pending_timer_insert, pending_timer_get, pending_timer_clone, pending_timer_remove, pending_timer_contains, and pending_timer_count; preserve their signatures and observable values.
- Shard::timer_entry and Runtime::capture_timer_entry create/capture the existing public TimerEntry; Runtime::timer_entry_fired forwards it as a timer-fire command.
- Pending timer snapshots include run_id, step, kind, generation, and deadline; recovery recreates the wait/ask timer from the persisted run boundary and existing timeout-unit rules.

## Minimal compatible internal design

- Make TimerWheel the sole canonical owner/index of each pending timer deadline, run, kind, and supplied generation inside RunAggregate; remove the IndexMap<RunId, PendingTimer> full-timer store.
- Keep one bounded IndexMap<RunId, StepIdx> sidecar in RunAggregate for metadata absent from public TimerEntry and to preserve deterministic run iteration order used by bounded snapshots; it must not duplicate deadline, kind, or generation.
- Preserve the existing PendingTimer public value by reconstructing it from the sidecar step and TimerWheel::get_entry; preserve pending_timer_clone as an IndexMap<RunId, PendingTimer> result in the same sidecar order.
- Add only crate-private wheel operations needed to insert an exact caller-supplied generation and peek/select one due entry; keep TimerWheel::insert, TimerEntry fields/layout, existing public methods, and all persisted schemas unchanged.
- Select the earliest due deadline, then the smallest RunId within equal-deadline entries, matching the current (deadline, run.get()) oracle; do not use fire_expired in shard tick because it drains all due entries at once.
- Peek before handling and let the existing validated handle_timer path perform removal at its current commit point, so a rejected, stale, or not-yet-due fire does not consume timer ownership.
- Preflight live-run and max_active_runs capacity checks, sidecar reservation, and checked generation before mutating either index; any returned error leaves wheel entries, sidecar entries, and snapshots unchanged.
- Keep sidecar and wheel membership one-to-one; replacement updates the existing sidecar entry without changing its insertion position, cancellation/removal removes both indexes with current swap_remove ordering, and clear resets both.
- Keep public TimerWheel::insert generation behavior for existing callers; shard integration uses the crate-private exact-generation path so caller-supplied PendingTimer.generation is never silently renumbered.

## Invariants and error contract

- Every wheel entry has exactly one sidecar StepIdx, and every sidecar run has exactly one matching wheel entry; pending_timer_count <= active_run_count <= max_active_runs, so timer count also remains within configured capacity.
- At most one pending timer belongs to a run; replacement removes its prior deadline index before the new deadline becomes observable.
- pending_timer_get, pending_timer_clone, captured TimerEntry, snapshots, and timer-fire validation agree on run, step where represented, kind, deadline, and generation.
- New/non-live run insertion returns existing RuntimeError::RunNotFound; capacity refusal returns existing RuntimeError::ActiveRunCapacityExceeded { capacity }; checked generation exhaustion follows existing None to RuntimeError::InvalidTimerFire path.
- Stale generation/deadline/kind, absent timer, early fire, cancellation, answer, finish, kill, and shutdown preserve current typed outcomes and remove timer ownership exactly once where the existing lifecycle commits that transition.
- No new public error variant, public API, dependency, configuration, or persisted-format version is introduced; allocation-failure behavior is not redefined by this contract.

## Behavioral acceptance scenarios

- Given one due wait timer and one later timer, when one empty shard tick runs, then only the earliest timer resolves once, the later run and timer remain unchanged, and the next deadline remains the later deadline.
- Given two due timers with the same deadline inserted in reverse RunId order, when successive empty shard ticks run, then the lower RunId resolves first and exactly one timer is consumed per tick.
- Given a pending run timer replaced with a strictly later deadline and the caller's checked next generation, when one shard tick runs at the old deadline, then the old entry is absent and that run has no resolution; when the replacement deadline arrives, that run resolves once with the replacement's exact fields.
- Given a same-run replacement that changes only step while preserving kind, generation, and deadline, when PendingTimer and TimerEntry are read, then the new step is preserved and the captured TimerEntry equals the prior token; no oracle treats that token as stale solely because step changed.
- Given a replacement whose authority tuple changes from generation g to its checked successor, when the old captured token is delivered through the existing timer-fire command path, then the exact result is Err(RuntimeError::InvalidTimerFire) and run state, current timer, journal, and counters remain unchanged; the replacement token follows its own deadline and authority fields.
- Given a pending wait or ask timer, when cancellation or ask-answer removes it, then wheel length, reconstructed lookup, and timer snapshots show no entry for that run, while next_deadline equals the earliest remaining deadline or None; subsequent stale fire preserves the existing typed error and cannot advance the run.
- Given max_active_runs = 2 and two distinct runs suspended on future timers, when each run is admitted, then active_run_count and pending_timer_count both equal 2 and remain within their configured bounds.
- Given that full valid shard, when an existing run's timer is replaced, then public Shard::pending_timer_insert returns Ok(Some(the exact previous PendingTimer)), both counts remain 2, and snapshots and next-deadline selection reflect the replacement.
- Given that full valid shard, when one more valid ShardCommand::SubmitWithInputsAndContracts using the same workflow is enqueued, then enqueue returns Ok(()) and the tick that processes it returns exactly Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 }); active_run_count, pending_timer_count, and existing timer snapshots remain unchanged.
- White-box diagnostic only, not public acceptance and not required formal proof: construct RunAggregate membership with two synthetic live runs, one existing timer, and call its private insertion helper with supplied capacity 1 for the second run; require exactly Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 }) and unchanged wheel entries, step sidecar, pending-timer clone, count, and next deadline. This synthetic lower-helper setup violates the shard's valid active/timer cardinality and demonstrates only defensive error atomicity.
- Given a timer at generation u64::MAX, when the existing checked next-generation path is invoked, then it returns no generation and the caller returns Err(RuntimeError::InvalidTimerFire) without changing either index or any snapshot.
- Given a non-live run, when insertion is attempted, then the exact result is Err(RuntimeError::RunNotFound) and both indexes are unchanged.
- Given multiple timer entries, when bounded pending-timer snapshots are taken, then selected membership, sorting, step, kind, generation, and deadline match the existing IndexMap-based snapshot oracle and the PendingTimerBoundarySnapshot schema is unchanged.
- Given a timer-suspended run is dropped and recovered through existing recovery, then run/step/kind and resume behavior are preserved, generation follows the existing checked recovery policy without a cross-reopen monotonicity guarantee, and the deadline follows vb-3n0rd recovery/time-slot semantics; do not require an Instant to survive reopen or alter journal/checkpoint formats.
- Given two due timers, when the first timer handler returns an error before its existing ownership-removal point, then the uncommitted timer and all other due timers remain represented exactly once and may be retried through the established path.

## Existing evidence and registration

- Reuse crates/vb_runtime/src/shard/timer_wheel.rs and its registered utility tests; those tests establish wheel-local insertion, cancellation, and next_deadline, not shard-aggregate integration.
- Preserve and strengthen crates/vb_runtime/src/shard/lifecycle_tests/timer_due_regressions.rs::idle_tick_resolves_only_due_owned_timer_once and owned_timer_fired_before_deadline_preserves_all_run_authority; crates/vb_runtime/src/shard/lifecycle.rs includes this test module.
- Preserve crates/vb_runtime/src/shard/lifecycle_tests/chunk_006.rs::cancel_clears_pending_timer and timer_fire_after_cancel_returns_run_not_found as cancellation/stale-fire regression oracles.
- Add focused shard lifecycle integration cases under the existing lifecycle test registration for the scenarios above; keep timer-wheel unit tests separate from shard integration assertions.

## Production caller blast radius

- Owner representation and returned-value adapters: crates/vb_runtime/src/shard/types.rs and crates/vb_runtime/src/shard/impl_parts/chunk_001.rs.
- Timer creation and recovery: crates/vb_runtime/src/shard/lifecycle/chunk_005.rs::pending_timer_for_awaiting and crates/vb_runtime/src/shard/lifecycle/chunk_001.rs::restore_recovered_timer.
- Due selection and validated fire: crates/vb_runtime/src/shard/impl_parts/chunk_001.rs::drive_one_due_timer and crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::handle_timer.
- Cancellation, answer, terminal cleanup: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs, crates/vb_runtime/src/shard/transitions_cleanup.rs, and crates/vb_runtime/src/shard/ask_authority.rs.
- Snapshot and recovery compatibility: crates/vb_runtime/src/shard/snapshot_chunks/chunk_002_shard_impl.rs and existing runtime recovery callers; preserve boundary snapshot and durable record types.
- Public timer-token compatibility: crates/vb_runtime/src/shard/timer_wheel.rs::TimerEntry and crates/vb_runtime/src/runtime.rs::{capture_timer_entry,timer_entry_fired} remain source-compatible and unchanged in representation.

## Explicit exclusions and handoff

- No fairness/latency bound, producer scheduling policy, full timer-wheel rewrite, bounded-ring redesign, runtime API redesign, new persisted schema, dependency, gate, or unrelated timer-unit change is in scope.
- This artifact is the design contract only; it does not claim implementation, behavioral-test, CODE review, TEST review, canonical-check, or phase-55 acceptance evidence.

