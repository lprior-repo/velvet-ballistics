# Durable wait / timed-ask recovery decision for `vb-d9ywf`

Scope: contract/evidence decision only. No production recovery behavior was implemented here.

## Workspace checks

- Required workspace: `/home/lewis/src/isoloated/velvet-ballistics-w25-runtime-a2`
- `git rev-parse --show-toplevel` returned `/home/lewis/src/isoloated/velvet-ballistics-w25-runtime-a2`.
- `jj root` returned `/home/lewis/src/isoloated/velvet-ballistics-w25-runtime-a2`.
- The coordination checkout at `/home/lewis/src/velvet-ballistics` was not edited.

## Decision

The current durable journal model is **not sufficient** to implement honest live hydration for wait timers or timed asks. They must remain fail-closed until a new durable timer-authority model is added to the Fjall-backed journal.

This is not a missing mapper only. The durable events preserve that a wait/ask boundary exists, but they do not preserve the timer authority that the runtime requires to accept a later timer fire or a timed-ask answer.

## Evidence from inspected source

### Live runtime authority requires process-local `Instant` and generation

- `crates/vb_runtime/src/shard/types.rs:36-42` defines `PendingTimer { step, kind, generation, deadline: Instant }`.
- `crates/vb_runtime/src/shard/types.rs:44-53` requires exact `generation`, `deadline`, and `kind` equality in `matches_authority`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_002_parts/chunk_001_boundary_control.rs:180-204` rejects `TimerFired` unless `pending_timer.matches_authority(generation, deadline, kind)` succeeds, then removes that exact timer before advancing the frame.
- `crates/vb_runtime/src/runtime.rs:776-798` exposes only captured live `TimerEntry` authority (`generation`, `deadline`, `kind`) to the caller; legacy run-only timer delivery is explicitly fail-closed at `runtime.rs:770-774`.

### Scheduling records do not persist the authority fields

- Runtime journal variants at `crates/vb_runtime/src/journal/chunk_001.rs:121-157` carry only `run` and `step` for `WaitScheduled`, `WaitResolved`, `AskScheduled`, and `AskTimedOut`; no deadline, generation, monotonic clock base, or timer id exists.
- Storage `JournalEvent` variants at `crates/vb_storage/src/events.rs:190-237` and `events.rs:320-330` carry only `run`, `seq`, `step`, and `attempt` for `WaitScheduledEvent`, `WaitResolvedEvent`, `AskScheduledEvent`, `AskAnsweredEvent`, and `AskTimedOutEvent`.
- Runtime-to-storage conversion at `crates/vb_runtime/src/journal/chunk_002.rs:234-272` hard-codes `attempt: 1` and persists only `run`, `seq`, and `step` for the wait/ask timer events.

### Current recovery intentionally fail-closes those states

- Storage recovery classifies unresolved waits as `pending_timers` and unresolved asks as `pending_asks` in `crates/vb_storage/src/recovery/hydrate.rs:293-385`.
- Full runtime recovery preserves the open-ask exception only for asks with **no timeout**. `crates/vb_runtime/src/recovery/full.rs:156-197` returns `RecoveredOpenAsk` only when the compiled `Ask` node has `timeout_slot: None` and a valid `AskResume` successor.
- Full runtime recovery marks `RecoveredStepState::Waiting` as `pending_timers` and timed/unrecoverable asks as `pending_asks` in `crates/vb_runtime/src/recovery/full.rs:106-142`.
- The Fjall-backed integration tests encode the current honest boundary:
  - pending action resumes after reopen: `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs:794-898`.
  - wait timer fails closed after reopen: `runtime_fjall_pending_action_recovery.rs:900-927`.
  - open ask resumes and can be answered after reopen: `runtime_fjall_pending_action_recovery.rs:929-983`.
  - timed ask fails closed after reopen: `runtime_fjall_pending_action_recovery.rs:985-1012`.

## Why a minimal implementation would be fake today

Possible shortcuts are not valid recovery:

- Rebuilding a `PendingTimer` with `Instant::now()` would fabricate a new deadline unrelated to the original wait/timeout.
- Rebuilding with generation `1` would erase stale-timer/replacement history and can accept a timer fire that was never durably scheduled.
- Answering a timed ask without timer authority would bypass the current `require_ask_timer_authority` guard and weaken timeout-vs-answer race semantics.
- Deriving a deadline from workflow slot values is insufficient because the persisted event model does not define how those values map to the process-local `Instant` used by `PendingTimer` and `TimerEntry`.

Therefore any implementation that makes `recover_and_resume` succeed for waits or timed asks under the current schema would be a synthesized resume, not durable hydration.

## Minimal future obligations for the follow-up bead

Keep Fjall mandatory. A valid fix needs a new durable timer-authority contract before runtime hydration can resume waits/timed asks:

1. Add a durable timer authority value object with at least: `run`, `step`, `kind`, generation/freshness token, durable deadline representation, and enough clock-domain metadata to reconstruct or safely rebase authority after restart.
2. Persist that authority atomically with `WaitScheduled` / timed `AskScheduled` in the Fjall journal; unresolved authority must replay after process restart.
3. Define stale replacement semantics across crash/reopen: old timer fires must be rejected; new timer fires must be accepted only with matching durable authority.
4. Decide the clock model explicitly (`Instant` cannot be serialized as-is): either migrate timer authority to a deterministic tick/deadline model or persist wall-clock deadlines with monotonic-rebase rules and fail-closed overflow handling.
5. Hydrate `PendingTimer` / timer-wheel entries only from durable authority, never from `Instant::now()` alone.
6. Extend full runtime recovery so `RecoveredStepState::Waiting` and timed `Ask` become resumable only when matching durable timer authority exists; otherwise keep `pending_timers` / `pending_asks`.
7. Add Fjall reopen tests for wait and timed-ask positive recovery, stale generation rejection, corrupt/missing authority rejection, deadline overflow/rebase rejection, and answer-vs-timeout ordering.

Until those obligations exist, the correct `vb-d9ywf` conclusion for wait/timed-ask is **CannotResume**, not fake resume.
