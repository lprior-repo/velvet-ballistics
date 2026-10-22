# Code Review — vb-tf5lr Durable Retry Authority (independent adversarial)

REVIEWED SHA: `f6a3631b50f11846c012b6f7d189bba73861dad5`
BASELINE: `685a70fc3ea7fb15e4f3f5a53e3c5803c23e244a`
VERDICT: **REJECTED**

Scope: real public `Runtime`/`FjallJournal`/CLI adversarial exercise of the durable
retry authority model, not inspection-only. Every finding below was reproduced with
a running example (see `.beads/vb-tf5lr/evidence/`), not assumed.

---

## Finding A — Stale ActionTicket authorizes the retried attempt (CRITICAL)

**Verdict:** REJECT — violates master §72.3 and rust-contract.md:60-61.

**Trigger.** A run fails its first attempt at an action step (`preflight.ticket.step`),
the failure boundary persists a durable `RetryCheckpoint`, the journal is cold-reopened,
`Runtime::recover_retry` + `tick_all` re-schedules the failed step, then the **old**
`ActionTicket` from the first attempt is submitted as completion (and separately as
failure) while the new pending ticket is active.

**Observed (red counterexample, `crates/vb_runtime/tests/durable_retry_stale_probe.rs`):**

```text
old=ActionTicket { run: 91002, step: 0, seq: SeqNo(0), action: 0, attempt: 1, ... }
new=ActionTicket { run: 91002, step: 0, seq: SeqNo(0), action: 0, attempt: 1, ... }
complete_action_with_output(old) = Ok(())   tick_all = Ok(true)   counters (0,0)->(1,0)
fail_action(old)                = Ok(())   tick_all = Ok(true)   counters (0,0)->(0,1)
```

The old ticket is **byte-identical** to the new ticket (`run`, `step`, `seq`,
`action`, `attempt=1`, `idempotency_key`, `capacity` all equal). The stale
`ActionCompleted`/`ActionFailed` is accepted and mutates the retried attempt: an old
completion completes the run (`runs_completed 0->1`) with the stale value; an old
failure fails the run a second time (`runs_failed 0->1`). No `StaleAttempt` rejection.

**Root cause (two compounding non-advancements):**

1. `crates/vb_runtime/src/run_checkpoint.rs:112-117` — `RunCheckpointV1::capture`
   clones `state.action_attempts` **verbatim** (comment admits this), so the failed
   step's per-action attempt counter is **not** advanced at the retry boundary.
   This contradicts rust-contract.md:23-24 ("`action_attempts[retry_step]` is the
   per-action attempt advanced by one").
2. `crates/vb_core/src/frame/checkpoint.rs:18-19,121` — the frame's `executed` counter
   is captured and restored verbatim, so the re-drive re-reads the same sequence:
   `crates/vb_runtime/src/engine/execute/handlers_suspend.rs:69`
   (`seq = SeqNo::new(run.executed())`) → `crates/vb_runtime/src/engine/action.rs:59-64`
   re-issues a ticket with `attempt: 1` and the identical `seq`/`idempotency_key`.

Because `attempt` and `seq` are both unchanged, the stale guard
`require_pending_action_ownership` (`crates/vb_runtime/src/shard/lifecycle/chunk_004.rs:44-64`)
sees `pending == ticket` (line 49) and returns `Ok(())`; the `attempt < pending.attempt`
branch (lines 50-60) is never reachable.

**Expected per contract.** On retry the per-action attempt counter must increment
(master §72.3: "On retry, the attempt counter increments"), the new ticket must carry
distinct authority, and any stale `ActionCompleted`/`ActionFailed` must reject with
`StaleAttempt { incoming, current }` leaving journal/runtime unchanged
(rust-contract.md:60-61).

**Impact.** Split-brain between overlapping retries is exactly the failure master §72
exists to prevent: a late/duplicate completion or failure from a prior attempt is
indistinguishable from the live attempt and is applied. Durably audited results can be
overwritten by stale evidence; a run can be reported completed from an aborted attempt.

**Fix (discrete).** Advance the failed step's per-action attempt counter at the retry
boundary (`capture` must write `action_attempts[retry_index] + 1` with checked
arithmetic, failing closed on `u16` overflow per the existing
`ActionAttemptOverflow` error), and/or advance the ticket's durable `seq` so the new
ticket is never bit-identical to the old one. Then the existing stale guard rejects the
old ticket with `StaleAttempt`. Update rust-contract.md:23-24 to match, and correct the
tests that currently pin the verbatim behavior (see test-review.md). Do **not** merely
increment beyond `max_attempts`, weaken the guard, or add a random nonce; the fix must
be a durable, deterministic run-attempt authority.

---

## Finding C — Automatic cold recovery silently resumes a failed run (CRITICAL)

**Verdict:** REJECT — the next-attempt checkpoint is not "evidence only until explicit
retry".

**Trigger.** Produce a genuinely failed run with a durable `RetryCheckpoint`, cold-reopen,
then call the public automatic recovery path `Runtime::recover_product` /
`Runtime::recover_and_resume` (no explicit `retry`).

**Observed (red, `probe_auto_recovery_does_not_resume_failed_run`):**

```text
recover_product  -> is_resumable=true  cannot_resume_reason=None
recover_and_resume = Ok(())   tick_all = Ok(true)
events 7 -> 10 ; counters (0,0) -> (0,1)
```

`recover_product` classifies the failed run as **Resumable**, and `recover_and_resume`
drives it, appending 3 further events and re-counting the run as failed. The failed run
is silently re-executed/re-failed by the automatic path instead of waiting for an
explicit retry.

**Root cause.** The automatic recovery classifier
(`crates/vb_runtime/src/recovery/full.rs:126-176`,
`mark_step_recovery_boundary` at line 150-166) treats
`RecoveredStepState::Failed` as a non-blocking state and does **not** treat a terminal
`RunFailed` event (nor the presence of a `RetryCheckpoint`) as cannot-resume. The
legacy frame-seed replay path is unaware of the new durable-retry checkpoint and
re-admits the failed run.

**Expected.** A failed run whose last terminal evidence is `RunFailed` (with or without
a `RetryCheckpoint`) must classify as `CannotResume`; the checkpoint is durable evidence
only until the explicit `retry` command invokes `recover_retry`.

**Impact.** Any automatic/startup recovery sweep that calls `recover_and_resume` will
re-run terminal failed runs, appending duplicate failure evidence and re-counting
failures — exactly the "failed run must not silently resume next attempt" prohibition.

**Fix (discrete).** Teach the recovery classification to treat terminal `RunFailed`
(and `RunFinished`/`RunCancelled`) as cannot-resume, and/or make
`recover_product`/`recover_and_resume` reject runs that carry a `RetryCheckpoint`
without an explicit retry command. Add a regression test driving `recover_and_resume`
on a failed journal with a checkpoint and asserting no journal mutation.

---

## Finding B — CLI `retry` silently downgrades Strict durability (HIGH)

**Verdict:** REJECT the durability binding; the admission policy is preserved but the
durability barrier is silently downgraded.

**Evidence.** `crates/velvet_ballistics/src/run_ops.rs:171-172` constructs the retry
runtime with `ShardConfig::default()` (policy `Strict`, see
`crates/vb_runtime/src/shard/types.rs:1212-1221`) but hardcodes
`StorageRuntimeJournal::shared_journaled` → `DurabilityProfile::Journaled` (no per-append
`SyncAll`, `crates/vb_runtime/src/journal/chunk_002.rs:29-36`). The `run` command maps
`--durability strict` → `open_storage_runtime_journal(db, /*strict=*/true)`
(`crates/velvet_ballistics/src/run_compiled.rs:149-167`), i.e. a strict append barrier.

`retry` has no `--durability` flag and no persistence of the original durability mode
(the admission `RuntimePolicy` is `Strict` for **both** `--durability strict` and
`--durability journaled` runs; the append barrier is a runtime-construction concern not
journaled). A `--durability strict` run, when retried, has its retry-phase evidence
(`ActionScheduledTicket`/`ActionCompleted`/`RunFinished`) appended **without** a sync
barrier.

**Impact.** For strict runs, the retry reports `ExitCode::SUCCESS` and may return before
retry evidence is fsynced — an acknowledged data-loss window violating the Strict
durability contract (master §61 write-path, §3258 `SyncAll` before return). The
admission/verification policy is **not** downgraded; the durability barrier is.

**Fix (discrete).** Persist the run's durability mode (or derive it from the journal),
and have `cmd_retry` select `shared_strict` vs `shared_journaled` accordingly, or default
`retry` to a strict barrier. Do **not** silently downgrade Strict.

---

## Finding D — Raw `recover_retry` trusts caller-supplied run-attempt authority (MEDIUM)

**Verdict:** Acceptable at the CLI boundary (digest-protected); documented trust
boundary at the raw public API.

**Evidence.** `Runtime::recover_retry(run, checkpoint, next_seq)`
(`crates/vb_runtime/src/runtime.rs:420`) is public and, together with
`RunCheckpointV1::restore`/`validate_against`
(`crates/vb_runtime/src/run_checkpoint.rs:153-231`), validates version, workflow/artifact
digest, frame run identity, step/slot counts, `retry_step` pending/PC, `action_attempts`
length, admission run/artifact, and action contracts — but **not** `run_attempt` or
`prior_seq` against the durable journal. `handle_recover` discards `prior_seq`
(`crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:416`) and
`RecoverRunCommand::for_retry` (`crates/vb_runtime/src/shard/types.rs:267`) never
cross-checks `checkpoint.prior_seq` against the supplied `next_seq`.

**Observed (red, `probe_recover_retry_accepts_tampered_run_attempt`):**
`recover_retry` with a checkpoint whose `run_attempt` is tampered to `99` (durable chain
recorded `2`) returns `Ok(())`; the run is admitted and tagged with the caller-supplied
attempt. The CLI boundary is protected — `load_latest_retry_checkpoint`
(`crates/velvet_ballistics/src/run_ops.rs:257-262`, added by this commit) verifies the
BLAKE3 `payload_digest` before decode, and `cmd_retry` is the only caller.

**Impact.** Latent, not CLI-reachable today: any future internal caller of
`recover_retry` that skips digest verification can inject run-attempt/prior-seq
authority that the durable chain never recorded.

**Fix (discrete).** In `recover_retry`/`for_retry`, validate
`checkpoint.prior_seq.checked_add(1) == next_seq` (the checkpoint is the immediately-
preceding event) and reject `run_attempt`/`prior_seq` values inconsistent with the run's
durable `RunFailed`/`RetryCheckpoint` evidence. This tightens the internal trust boundary
without weakening the CLI digest gate.

---

## Cross-cutting note — contract/implementation drift

rust-contract.md:23-24 states `action_attempts[retry_step]` is "advanced by one"; the
implementation captures verbatim. The implementation is internally self-consistent with
the (buggy) tests, but the contract text and master §72.3 disagree with both. Fix the
implementation to match the contract and master, not the other way around.

**Legitimate discrete fix set:** (A) advance per-action attempt at capture (checked);
(C) classify terminal failed runs as cannot-resume in automatic recovery; (B) preserve
durability mode in `cmd_retry`; (D) bind `prior_seq`/`run_attempt`/`next_seq` in
`recover_retry`. Update rust-contract.md and the two tests that pin the verbatim
behavior.
