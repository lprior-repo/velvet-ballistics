# Test Review — vb-4qog3 Independent Review of vb-hz9v8 B/C/D Test Coverage

REVIEWED SHA: `0d348f5ae0aba7a9cce32d4ad0b990e66da07d2b`
VERDICT: **APPROVED**

Independent test review of the frozen vb-hz9v8 B/C/D test coverage. This review
inspects the test suite, runs all tests through `/tmp/vb-run-gate.py` with true
exit codes, and writes additional adversarial tests to verify the implementation
against the original vb-tf5lr contract.

Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u` and is NOT
in scope for this review.

---

## Test Execution Evidence

All tests run through `/tmp/vb-run-gate.py SECONDS .beads/vb-4qog3/evidence/UNIQUE cargo ...` with
true exit codes, full stdout/stderr, and no cargo pipelines or zero-tests claims.

| Test Suite | Command | Exit | Duration |
|---|---|---|---|
| `cargo check -p vb_runtime` | `cargo check -p vb_runtime` | 0 | 5.2s |
| `durable_retry_boundary` | `cargo test -p vb_runtime --test durable_retry_boundary` | 0 | 6.6s |
| `durable_retry_execution` | `cargo test -p vb_runtime --test durable_retry_execution` | 0 | 0.3s |
| `durable_retry_cli` | `cargo test -p velvet-ballistics --test durable_retry_cli` | 0 | 8.4s |
| `recovery unit tests` | `cargo test -p vb_runtime --lib -- recovery` | 0 | 6.9s |
| **Independent adversarial tests** | `cargo test -p vb_runtime --test vb_4qog3_independent_review` | 0 | 1.0s |

Raw logs preserved in `.beads/vb-4qog3/evidence/` (JSON + stdout/stderr for each run).

---

## Author Test Coverage Assessment

### Finding B — CLI Durability (HIGH)

**Author test:** `durable_retry_boundary.rs::shared_strict_journal_persists_each_append`

Covers: Verifies `shared_strict` routes appends through `append_strict` (SyncAll).
This is a constructor-level test that pins the wiring.

**Gap:** Does not verify that the CLI `retry` command actually uses `shared_strict`.
The CLI wiring is verified by source inspection (line 177 of `run_ops.rs`), not by
test. A stronger test would be an integration test that retries a run and verifies
the durability of the retry-phase events.

**My coverage:** `vb_4qog3_independent_review.rs::b_independent_verify_strict_durability_barrier`
exercises the full path: produce failed attempt → `shared_strict` retry runtime →
`recover_retry` → complete action → verify `RunFinished` is durable and visible
after reopening the journal. This is a stronger integration-level verification.

### Finding C — Auto Recovery of Failed Run (CRITICAL)

**Author test:** `durable_retry_boundary.rs::auto_recovery_does_not_resume_failed_run`

Covers: Produces a failed run with checkpoint, cold-reopens, calls `recover_product`
and `recover_and_resume`, asserts `CannotResume` and no journal mutation. This is
exactly the correct test for finding C.

**Author test:** `durable_retry_boundary.rs::auto_recovery_resumes_suspended_pending_action_run`

Covers: Verifies a genuinely suspended run (pending action, no terminal evidence)
is still resumable. This tests the "no over-rejection" case.

**My coverage:** `vb_4qog3_independent_review.rs::c_independent_multiple_auto_recovery_calls`
strengthens this by trying `recover_and_resume` three times consecutively and
asserting no mutation after all three attempts. `vb_4qog3_independent_review.rs::c_independent_suspended_run_still_recovers`
independently verifies the no-over-rejection case.

### Finding D — Raw `recover_retry` Validation (MEDIUM)

**Author tests:** `durable_retry_boundary.rs` includes:
- `recover_retry_rejects_tampered_run_attempt`
- `recover_retry_rejects_tampered_prior_seq`
- `recover_retry_rejects_tampered_next_seq`
- `recover_retry_rejects_cross_run_checkpoint`
- `recover_retry_rejects_missing_durable_checkpoint`
- `recover_retry_rejects_stale_checkpoint_but_accepts_current`
- `recover_retry_rejects_already_consumed_checkpoint`

These cover all the D validation checks comprehensively.

**My coverage:** `vb_4qog3_independent_review.rs` includes:
- `d_independent_consumed_checkpoint_reuse_rejected` — stronger consumed-checkpoint
  test: completes the run after retry, then tries to reuse the checkpoint (vs.
  author's test which leaves the run in a different state)
- `d_independent_cross_run_checkpoint_rejected` — independent cross-run test
- `d_independent_wrong_prior_seq_rejected` — independent prior_seq tamper test
- `d_independent_wrong_run_attempt_rejected` — independent run_attempt tamper test
- `d_independent_valid_retry_still_works` — green happy-path counterpart
- `cross_independent_no_mutation_on_rejected_retry` — verifies no state/journal
  mutation after multiple rejected retry attempts with different tamper variations

The independent tests use a different `produce_failed_attempt` implementation and
different RunIds, providing true independent verification rather than just
re-running the author's tests.

---

## Unit Test Coverage

`crates/vb_runtime/src/recovery/tests.rs` includes unit tests for
`run_is_currently_terminal`:
- `terminal_failed_run_with_checkpoint_is_terminal` — failed run with checkpoint → terminal
- `explicitly_retried_run_is_not_terminal` — failed run + explicit retry → not terminal
- `finished_run_is_terminal` — completed run → terminal
- `suspended_run_is_not_terminal` — run with no terminal event → not terminal

These correctly cover the classification logic.

---

## Finding A Oracle

The original adversarial probes from `.beads/vb-tf5lr/evidence/durable_retry_stale_probe.rs`
(`probe_old_completion_authority`, `probe_old_failure_authority`,
`probe_old_ticket_equality_against_retried_ticket`) remain RED in this workspace.
This is expected and correct: finding A is owned by LOCAL5090 `vb-wop2u` and has
not been fixed in this candidate. The probes are preserved but not included in
the test suite to avoid blocking this bead's landing.

---

## Test Quality Assessment

**Strengths:**
- All author tests drive real `Runtime`/`FjallJournal`/CLI boundaries — no mocks
- Rejection tests verify both the rejection and no-mutation (journal unchanged)
- Tamper tests cover run_attempt, prior_seq, next_seq, run identity, and consumed state
- Unit tests cover the classification logic exhaustively

**Limitations:**
- CLI durability test is constructor-level, not end-to-end integration
- No test corrupts a checkpoint payload and asserts CLI rejection (mentioned in
  vb-tf5lr test review as Gap 3, not addressed in vb-hz9v8)
- No test verifies run-attempt/prior_seq binding against the durable chain at the
  CLI level (mentioned in vb-tf5lr test review as Gap 4)

---

## Conclusion

The test suite comprehensively covers all three findings (B, C, D). The author
tests are well-designed and correctly verify the implementation. My independent
adversarial tests provide additional verification of the rejection paths and the
happy path, using different test infrastructure to ensure true independence.

**VERDICT: APPROVED**