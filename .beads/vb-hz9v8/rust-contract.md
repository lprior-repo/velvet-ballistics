# Rust Contract — vb-hz9v8 Durable Retry Boundary Defects (B/C/D)

Bead: `vb-hz9v8`
Scope: independently PROVEN findings B, C, D from `.beads/vb-tf5lr/{code-review,test-review}.md`.
Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u`; NOT in scope here.
Root owns final publication/review/CI/main and bead remote sync.

Authoritative upstream contract: `/velvet-ballistics-MASTER.md` §61 (write-path), §72 (retry
authority), §3258 (`SyncAll` before return). Rust rules: no `unsafe`/`unwrap`/`expect`/
`panic`/`todo`/`unimplemented`/`dbg`, no unchecked arithmetic/index/cast. Canonical source
clippy zero-tolerance.

---

## Finding B — CLI `retry` silently downgrades Strict durability (HIGH)

**Defect.** `cmd_retry` (`crates/velvet_ballistics/src/run_ops.rs`) constructs the retry runtime
with `StorageRuntimeJournal::shared_journaled(…)`, i.e. `DurabilityProfile::Journaled` (no
per-append `SyncAll`). The `run` command maps `--durability strict` →
`open_storage_runtime_journal(db, /*strict=*/true)` → `shared_strict` (a strict append barrier).
A `--durability strict` run, when retried, therefore appends its retry-phase evidence
(`StepStarted`/`ActionScheduledTicket`/`ActionCompleted`/`RunFinished`) WITHOUT a sync barrier.

The admission `RuntimePolicy` is `Strict` for **both** `--durability strict` and
`--durability journaled` (see `run_compiled.rs::runtime_config_for_durability`: only
`DurabilityMode::None` relaxes the policy). The append barrier is a runtime-construction
concern and is **not** journaled per-run. There is no per-run durability mode recorded in
`RunAdmission`/`JournalEvent`, so the original mode cannot be derived from the journal.

**Contract (minimal).** `cmd_retry` MUST select the strict append barrier by constructing the
retry runtime journal via `StorageRuntimeJournal::shared_strict(Arc::new(journal))`. This is the
safe default: it never weakens a previously-strict run (it upgrades a journaled run's retry
phase to strict). Because the original durability is not recorded, strict is the only
fail-safe choice.

**Verified durability path (actual, not string scan).**
`shared_strict` → `StorageRuntimeJournal { profile: DurabilityProfile::Strict }` →
`append_storage_event` → `self.journal.append_strict(event)` →
`JournalWriteBatch::strict().commit()` → `fjall::PersistMode::SyncAll` (atomic single-barrier
commit; see `crates/vb_storage/src/journal/append.rs`, `crates/vb_storage/src/batch/commit.rs:7-9`).
The `append_strict` SyncAll barrier is already independently proven by
`crates/vb_storage/src/edge_case_tests.rs::append_strict_commit_failure_leaves_event_not_visible_and_retry_is_idempotent`.

---

## Finding C — Automatic cold recovery silently resumes a terminal failed run (CRITICAL)

**Defect.** `Runtime::recover_product` / `Runtime::recover_and_resume` classify a run whose last
terminal evidence is `RunFailed` (with a durable `RetryCheckpoint`) as **Resumable**, then
`recover_and_resume` re-drives it, appending duplicate failure evidence and re-counting the run
as failed. The next-attempt checkpoint is evidence only until an explicit `retry`.

Root cause: the runtime classifier `crates/vb_runtime/src/recovery/full.rs::classify_full_recovery_resume`
treats `RecoveredStepState::Failed` as non-blocking and never inspects terminal evidence. The
`RetryCheckpoint` event's `attempt` field is the *authorized next* attempt (`run_attempt + 1`,
see `transitions.rs::capture_retry_checkpoint`), which inflates any naive "latest attempt" scan
and makes the stale `RunFailed` (previous attempt) look superseded.

**Contract (minimal).** The automatic recovery classifier MUST treat a run as `CannotResume`
when its latest terminal evidence (`RunFailedEvent`/`RunFinished`/`RunCancelled`/`RunKilled`)
belongs to the run's **current** attempt — where "current attempt" is the maximum `attempt()`
over journal events **excluding `RetryCheckpoint`** (its `attempt` names a not-yet-started
attempt). Concretely:

- A failed run ending in `RunFailedEvent(attempt=N)` + `RetryCheckpoint(attempt=N+1)` (no further
  events) has current attempt `N`; the last terminal event is at attempt `N` → **CannotResume**.
- A run that failed at `N`, was **explicitly** retried, and is now genuinely active (e.g.
  `StepStarted(attempt=N+1)`, pending `ActionScheduledTicket`) has current attempt `N+1`; its last
  terminal event is stale (attempt `N`) → **Resumable** (preserves actual pending-action / Ask
  receipt-reconciliation semantics).
- Terminal `RunFinished`/`RunCancelled`/`RunKilled` at the current attempt → **CannotResume**.

`recover_product` MUST return `RuntimeRecoveryProduct::CannotResume` (not `Err`) for such runs so
`recover_and_resume` rejects them via the existing typed `RecoveryCannotResume` error, leaving
the journal and shard state unchanged. The explicit `recover_retry` path (finding D) is unaffected
and remains the only valid explicit transition out of a terminal failed state.

The reason token is `"terminal_state"`; it is carried by a private reason field on the
vb_runtime `NonResumableRecoveryProduct` (NOT a new `RecoveryCannotResumeState` flag, to avoid
changing the Verus-bound `vb_storage` decision type and its drift-gated mirrors).

---

## Finding D — Raw `recover_retry` trusts caller-supplied run-attempt authority (MEDIUM)

**Defect.** `Runtime::recover_retry(run, checkpoint, next_seq)` validates checkpoint version,
digest identities, frame dimensions, retry-step pending/PC, action-attempt length, admission,
and action contracts — but not `run_attempt`/`prior_seq`/`next_seq` against the durable chain.
`RecoverRunCommand::for_retry` never cross-checks `checkpoint.prior_seq` against `next_seq`, and
`handle_recover` discards `prior_seq`. The CLI boundary is digest-protected
(`load_latest_retry_checkpoint`), but the raw public API is not.

**Durable event sequence (actual, observed from `transitions.rs::capture_retry_checkpoint`).**
At a failure boundary the journal ends:

```
… RunFailedEvent { run, seq=S,          attempt=A }      // the audited failure
  RetryCheckpoint { run, seq=S+1,       attempt=A+1 }    // authorized next attempt
```

where `A = state.run_attempt` (failed attempt), `checkpoint.run_attempt = A+1`,
`checkpoint.prior_seq = S`, the `RetryCheckpoint` seq is `S+1 = prior_seq + 1`, and the CLI/probe
compute `next_seq = (S+1)+1 = prior_seq + 2`. The reviewer's `prior_seq+1 == next_seq` is off by
one because of the separate `RetryCheckpoint` event; do NOT use that formula.

**Contract (minimal).** Before enqueueing `ShardCommand::Recover`, `Runtime::recover_retry` MUST
bind the supplied checkpoint to the durable chain (checked arithmetic throughout, fail-closed,
no state/journal mutation on rejection):

1. Load the run's full events. Find the **latest** `RetryCheckpoint` event for the run; fail
   closed (`RuntimeError::Recovery`) if absent (legacy/missing).
2. Verify `blake3(durable.payload) == durable.payload_digest` (integrity of the durable record).
3. Decode `durable.payload` to a `RunCheckpointV1` and require it to equal the supplied
   `checkpoint` (binds run/step/attempt/prior_seq/artifact/workflow/policy/grants/frame/etc.).
4. Require `checkpoint.run_attempt == durable.attempt`.
5. Require `checkpoint.prior_seq.checked_add(1) == durable.seq` (binds `prior_seq` to the actual
   checkpoint position).
6. Require `durable.seq.checked_add(1) == next_seq` (binds `next_seq`).
7. Require the durable checkpoint to be the journal head (its `seq` equals the last event's
   `seq`): an explicit retry that already consumed the checkpoint appends
   `StepStarted`/`ActionScheduled`/`RunFinished` after it, so a stale `next_seq` alone cannot be
   trusted to detect an already-consumed checkpoint.
8. Require the event at `seq == checkpoint.prior_seq` to be a same-run `RunFailedEvent` with
   `attempt == checkpoint.run_attempt.checked_sub(1)` (binds to the actual preceding failure).

A tampered/stale/cross-run/already-consumed checkpoint fails any of 3/4/7/8 and is rejected
before activation.
The correct current explicit retry (real failure-boundary checkpoint, real `next_seq`) satisfies
1–8 and continues to drive the selected failed step, preserving the prior journal prefix.

---

## Verification plan

- Adopt the real adversarial probes from `.beads/vb-tf5lr/evidence/durable_retry_stale_probe.rs`
  as permanent runtime tests for C and D (real `Runtime` + real `FjallJournal`, real
  failure-boundary checkpoint producer, real cold reopen). Preserve the raw red repro in
  `.beads/vb-hz9v8/evidence/`.
- C: assert `recover_product` → `CannotResume` and `recover_and_resume` → `Err` with byte-identical
  journal event count (no mutation).
- D: assert `recover_retry` rejects tampered `run_attempt`/`prior_seq`/`next_seq`, wrong-run and
  stale/multiple-attempt checkpoints, with no journal/state mutation; and that the valid current
  explicit retry still drives the failed step preserving the prior prefix.
- B: assert `cmd_retry` builds a strict journal (constructor-level) and the strict `SyncAll` path
  is exercised; keep the CLI re-execute happy path green.
- Run the 20 existing Fjall recovery tests and the existing `durable_retry_execution` /
  `durable_retry_cli` suites unchanged except as needed for B/C/D.
- Finding A oracle (`durable_retry_stale_probe.rs` stale-ticket tests) may remain RED in this
  isolated candidate until root merges LOCAL5090 A; report honestly, do not weaken/remove.
