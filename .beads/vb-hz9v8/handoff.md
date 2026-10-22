# Handoff — vb-hz9v8 (B/C/D durable retry boundary fixes)

Author: DeepSeek v4Pro (fresh implementation context)
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-hz9v8-deepseek-boundaries`
Root owns final publication/review/CI/main and bead remote sync.

## Scope

Fixed independently PROVEN findings B/C/D from `.beads/vb-tf5lr/{code-review,test-review}.md`.
Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u`; NOT touched here.

## Source changes (7 files)

1. `crates/velvet_ballistics/src/run_ops.rs` — **B**: `cmd_retry` constructs the retry runtime
   journal via `StorageRuntimeJournal::shared_strict` instead of `shared_journaled`. The original
   per-run append mode is not journaled (`RuntimePolicy` is `Strict` for both `--durability
   strict` and `journaled`), so strict is the fail-safe default (never weaker, upgrades journaled
   retry to a `SyncAll` barrier). Verified path: `shared_strict` → `DurabilityProfile::Strict` →
   `append_strict` → `JournalWriteBatch::strict().commit()` → `fjall::PersistMode::SyncAll`.

2. `crates/vb_runtime/src/recovery/full.rs` — **C**: added `run_is_currently_terminal(events)`
   which computes the current run attempt excluding `RetryCheckpoint` (whose `attempt` is the
   authorized-next attempt) and reports whether the latest terminal event is at that attempt.

3. `crates/vb_runtime/src/recovery/full/product.rs` — **C**: added a private `reason:
   Option<&'static str>` override to `NonResumableRecoveryProduct` and a `RuntimeRecoveryProduct::
   cannot_resume_terminal(summary)` constructor (reason `"terminal_state"`). Deliberately avoids
   adding a field to the Verus-bound `vb_storage::RecoveryCannotResumeState` and its drift-gated
   mirrors.

4. `crates/vb_runtime/src/recovery.rs` — **C**: re-export `run_is_currently_terminal`.

5. `crates/vb_runtime/src/runtime.rs` — **C + D**:
   - C: `product_from_recovered_context` returns `CannotResume` (`terminal_state`) when the run
     is currently terminal, before building the resumable product.
   - D: `recover_retry` now calls `validate_retry_checkpoint_against_durable_chain` before
     enqueueing. It binds the supplied checkpoint to the actual latest durable `RetryCheckpoint`
     (blake3 integrity + decode-and-value-equality of the full checkpoint), and verifies
     `prior_seq + 1 == durable.seq`, `next_seq == durable.seq + 1`, that the durable checkpoint is
     the journal head (rejects an ALREADY-CONSUMED checkpoint whose retry appended later
     StepStarted/ActionScheduled/RunFinished events), and that the immediately preceding event is
     the run's own `RunFailedEvent` at `run_attempt - 1`. Checked arithmetic throughout;
     fail-closed, no state/journal mutation. Do NOT use `prior_seq + 1 == next_seq` (off by one —
     the separate `RetryCheckpoint` event sits between).

6. `crates/vb_runtime/src/recovery/tests.rs` — **C** unit tests for `run_is_currently_terminal`
   (failed+checkpoint → terminal; explicitly-retried active → not terminal; finished → terminal;
   suspended no-terminal → not terminal).

7. `crates/vb_runtime/tests/durable_retry_boundary.rs` (new) — public-API regression tests:
   - C: `auto_recovery_does_not_resume_failed_run` (recover_product CannotResume + recover_and_resume
     Err + journal unchanged), `auto_recovery_resumes_suspended_pending_action_run` (no over-reject).
   - D: tampered `run_attempt`/`prior_seq`/`next_seq`, cross-run, missing-checkpoint,
     stale-vs-current multiple-attempt rejection, and already-consumed checkpoint (run completed
     after retry, stale `next_seq`) rejection; valid current checkpoint still drives the step.
   - B: `shared_strict_journal_persists_each_append`.

## Contract / evidence

- Contract: `.beads/vb-hz9v8/rust-contract.md`
- Raw red repro preserved: `.beads/vb-hz9v8/evidence/durable_retry_stale_probe_raw_red.rs`,
  `.beads/vb-hz9v8/evidence/probe_final_raw_red.json`.
- Runner evidence (JSON + stdout/stderr): `.beads/vb-hz9v8/evidence/`.

## Verification (all through `/tmp/vb-run-gate.py`, true exit codes)

| Check | Result |
|---|---|
| `cargo check -p vb_runtime` | 0 |
| `cargo check -p velvet-ballistics` | 0 |
| `cargo clippy -p vb_runtime -p velvet-ballistics --lib --bins --all-features` (canonical flags) | 0 |
| `cargo test -p vb_runtime --lib` | 1938 passed |
| `cargo test -p vb_runtime --test durable_retry_boundary` | 10 passed |
| `cargo test -p vb_runtime --test durable_retry_execution` | 2 passed |
| `cargo test -p velvet-ballistics --test durable_retry_cli` | 3 passed |
| `cargo test -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery` | 20 passed |
| `cargo test -p velvet-ballistics --test vb_md3w_retained_input_recovery --test vb_7x0ol_probe_c6_continue` | 2 passed |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_7x0ol_probe_registered_executor` | 5 passed |

Formatting: `rustfmt --edition 2024` run per-file on the 7 changed files. Note: whole-workspace
`cargo fmt` is pre-existing-broken on `crates/vb_storage/src/types/flux_sequence.rs` (unparseable
`#[flux_rs::sig]` attribute) — unrelated to this bead; parent owns that surface.

## Known pending (not in this bead's scope)

- Finding A stale-ticket probes in `.beads/vb-tf5lr/evidence/durable_retry_stale_probe.rs`
  (`probe_old_*`) remain RED here until root merges LOCAL5090 `vb-wop2u`. Not weakened/removed.
- `RuntimeError::Recovery { error }` has an empty `Display` (pre-existing gap in
  `crates/vb_runtime/src/error/display.rs`); diagnostics surface only via `Debug`. Left unchanged.

## NOT done (root owns)

JJ mutations, commit/describe, publication, `bd` remote sync, canonical CI/moon/fmt --all, bead
close/status transitions.
