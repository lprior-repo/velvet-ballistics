# vb-r5c1c Durable CLI Retry — Rust Contract

Bound to actual public APIs. Anything here that names a symbol is exercised by the
acceptance tests in `crates/velvet_ballistics/tests/durable_retry_cli.rs` and the
runtime round-trip tests in `crates/vb_runtime/src/run_checkpoint.rs`.

## Producer (runtime failure boundary)

- When a run reaches a failed terminal boundary — either `Shard::handle_action_failure`
  with `ActionFailureOutcome::FailRun` (step `preflight.ticket.step`) or the engine
  failure path `apply_terminal_failed` (step `state.frame.pc()`) — the selected
  failed step is marked `Failed` in the live frame and `RunCheckpointV1::capture`
  is invoked with:
  - `run_id` = the failed run,
  - `artifact_digest` = `state.admission.artifact_digest()` (missing admission ⇒ no
    checkpoint; fail-closed),
  - `run_attempt` = `state.run_attempt.get().checked_add(1)` (overflow ⇒ no checkpoint),
  - `retry_step` = `preflight.ticket.step`,
  - `prior_seq` = the durable sequence of the `RunFailed` event already appended.
- The serialized checkpoint (`postcard`) is BLAKE3-digested and appended as one
  `RuntimeJournalEvent::RetryCheckpoint { run, attempt, step, payload_digest, payload }`.
- `attempt` in the event and `run_attempt` in the payload are both the **next** run
  attempt (`failed + 1`). The checkpoint payload's `action_attempts[retry_step]` is the
  per-action attempt advanced by one; the run attempt is advanced exactly once.
- Only the selected failed step is reset to `Pending`; every prior `Succeeded`/`Skipped`
  step state, all slots/taint, value-store arenas, collect pagination, admission, and
  action contracts are captured verbatim.
- Any capture or append failure leaves the run durably `Failed` (its `ActionFailed` +
  `RunFailed` batch is already persisted) but **without** a retry checkpoint, so the run
  is not retryable. No partial retry authority is ever written.

## Consumer (CLI `retry`)

`cmd_retry(run_id, requested_step, db, output)`:

1. Parse `run_id` as `RunId`; reject parse failure (storage exit).
2. Open `FjallJournal` once; read `events_for_run(run)`; empty ⇒ storage exit.
3. `analyze_retry(&events)` must report `can_retry`; otherwise lifecycle exit.
4. `observed_failed_step` (from the journal) must equal `requested_step`; otherwise
   lifecycle exit with `observed_failed_step`/`requested_step` in the payload. No
   `saturating_add`/`MAX` inference.
5. Find the **latest** `JournalEvent::RetryCheckpoint`; verify the recorded BLAKE3
   `payload_digest` matches the payload bytes (tamper rejection), then decode into
   `RunCheckpointV1`. Missing/undecodable/tampered ⇒ lifecycle exit. Reject if
   `checkpoint.run_id != run` or `checkpoint.retry_step != requested_step`.
6. `next_seq = last_event.seq() + 1` (checked).
7. Build a `StorageRuntimeJournal` over the opened journal and a default
   (`Journaled`-policy) `ShardConfig`; construct `Runtime`.
8. `runtime.recover_retry(run, checkpoint, next_seq)` then `tick_all()`.
   `handle_recover` re-validates artifact identity, workflow identity, policy, ABI
   grant authority, and frame/store/collect integrity **before** mutating run state,
   then drives re-execution of the failed step.
9. Report `success` only after the runtime drives the recovered attempt and appends
   durable evidence. Rejections in steps 1–8 leave the journal bytes/events unchanged.

## Invariants verified by tests

- Retry preserves every prior journal event and appends only; the prefix is byte-identical.
- The next attempt is checked arithmetic; `u16::MAX` failure rejects before mutation.
- Stale `ActionCompleted`/`ActionFailed` tickets (attempt < current) reject with
  `RuntimeError::StaleAttempt { incoming, current }` and leave journal/runtime unchanged.
- Cold reopen loads the latest checkpoint and drives only the current attempt; prior
  attempt events are ignored for execution.
- `RunCheckpointV1` round-trips `frame`/`value_store`/`action_attempts`/`admission`/
  `collect_states`/`action_contracts`/`run_attempt`/`retry_step`/`prior_seq` exactly.

## Out of scope (unchanged)

- Legacy `velvet_ballistics::lifecycle::retry` (vb-qi37.16.5 single-event transition).
- `vb-nn1yo` journal-analysis repair, ABI digest implementations, and the separate
  FullV1 grant lifecycle `vb-hk1j8`.
