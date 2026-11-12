# Test Review — vb-tf5lr Durable Retry Authority

REVIEWED SHA: `f6a3631b50f11846c012b6f7d189bba73861dad5`
VERDICT: **REJECTED**

The acceptance tests for this bead pass but do not exercise the two authority
requirements that the bead's own contract and master §72 mandate. Independently added
adversarial probes (real public API, real `FjallJournal`, real failure-boundary
checkpoint producer, real cold reopen) reproduce defects the existing suite misses.

## Baseline (reproduced, true-exit runner)

- `cargo test -p vb_runtime --test durable_retry_execution` → 2 passed.
- `cargo test -p velvet-ballistics --test durable_retry_cli` → 3 passed.

Both green, as reported. Green ≠ coverage of the stale-ticket / auto-recovery contract.

## Gap 1 — stale-ticket rejection is never exercised, and the tests pin the wrong behavior

`durable_retry_execution.rs::durable_retry_reopens_and_completes_failed_step` asserts
`retry_ticket.attempt == 1` (line ~220) and `checkpoint_roundtrips_full_retry_authority`
asserts `checkpoint.action_attempts == [1, 0]` (line ~310, verbatim, not advanced) —
while its own docstring claims "the advanced per-action attempt counter". These assert
the exact behavior that defeats the stale guard (code-review Finding A).

The contract requirements are:

- rust-contract.md:60-61 — "Stale `ActionCompleted`/`ActionFailed` tickets (attempt <
  current) reject with `RuntimeError::StaleAttempt { incoming, current }` and leave
  journal/runtime unchanged."
- master §72.3 — "On retry, the attempt counter increments. Any `ActionCompleted`/
  `ActionFailed` event carrying a stale attempt number is rejected with
  `StaleAttempt { expected, found }`."

No existing test submits the **old** ticket from the first failed attempt after a
recover/retry and asserts `StaleAttempt` + no mutation. The three CLI tests cover only
re-execute, wrong-step rejection, and non-failed-run rejection — none exercises a stale
ticket.

**Required test:** after a genuine terminal action failure → durable checkpoint → cold
`recover_retry` → `tick_all`, submit the actual old `ActionTicket` (completion and
failure separately) and assert exact `StaleAttempt { incoming: 1, current: 2 }` (or the
post-fix attempt values) with byte-identical journal prefix and unchanged counters.
My probe `durable_retry_stale_probe.rs::probe_old_completion_authority` /
`probe_old_failure_authority` / `probe_old_ticket_equality_against_retried_ticket` is
the failing-before counterexample (preserved in `.beads/vb-tf5lr/evidence/`).

## Gap 2 — automatic cold recovery of a failed run is never exercised

No test drives `Runtime::recover_product` / `recover_and_resume` on a failed journal
that carries a `RetryCheckpoint` and asserts non-resumable / no mutation. The contract
("next-attempt checkpoint must remain evidence only until explicit retry") is untested.

My probe `probe_auto_recovery_does_not_resume_failed_run` shows
`recover_product` → `Resumable` and `recover_and_resume` → `Ok(())` with journal growth
7→10 and `runs_failed 0->1` (code-review Finding C).

**Required test:** cold-reopen, `recover_and_resume(failed_run)`, assert `Err`
(`RecoveryCannotResume`) and identical journal event count.

## Gap 3 — payload-digest tamper rejection is implemented but untested

`load_latest_retry_checkpoint` (`run_ops.rs:257-262`) was added by this commit to reject
a tampered checkpoint payload, but no test corrupts a checkpoint payload and asserts
the CLI rejects it with no mutation. A CLI-level tamper test is required to lock the
boundary.

## Gap 4 — run-attempt/prior_seq/next_seq binding is untested

No test asserts `recover_retry` rejects a checkpoint whose `run_attempt`/`prior_seq` is
inconsistent with the durable chain, or a `next_seq` inconsistent with `prior_seq`
(code-review Finding D). Add a raw-API adversarial test if the API is intended to be
public-safe; otherwise document the trust boundary explicitly.

## Conclusion

The existing 3 CLI + 2 runtime tests are green and valuable for the happy path and
non-mutating rejection paths, but they do not cover — and in two cases actively pin the
wrong behavior for — the stale-ticket and automatic-recovery authority requirements
that this bead's contract names as acceptance criteria. Until the required tests exist
and pass, the test suite cannot approve the bead.

STATUS: **REJECTED**
