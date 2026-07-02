# Landing Report — vb-0x1cb

STATUS: LANDED (bead closed, source-gate clean, dolt push complete)

## Preconditions

- `final-evidence-decision.md`: STATUS: APPROVED (state 14, 2026-07-01T20:00Z).
- `truth-serum-report.md`: exists; STATUS: APPROVED.
- `black-hat-review.md`: STATUS: APPROVED (no blocker / lethal / HIGH / MEDIUM findings;
  5 LOW + 1 OBSERVATION, all owner_approved_debt / owner_approved_no_action).
- `formal-verification-report.md`: STATUS: APPROVED (verification-ledger: 5 PASS, 2 FAIL_LOCAL).
- `implementation.md` (state 11 holzman-rust) reports production-source and test edits complete.
- `assurance-bundle.md`: STATUS: APPROVED.
- Discarded-fallible-results repair complete:
  - `crates/vb_runtime/src/shard/transitions.rs:100/202` — `let _ = self.run_state_insert(...)` replaced
    by `if let Err(secondary) = self.run_state_insert(run, state) { ... }` bound expression.
  - `crates/vb_runtime/src/trace/event.rs` — added `TraceEvent::RunRollbackFailed` variant
    + `RollbackSite` helper enum (bounded payload; ≤ 25 bytes on x86_64
    per Flux PO-005 spec `SIZE_BOUND_BYTES`).
  - `crates/vb_runtime/src/trace.rs` — wired new variant through trace ring dispatch.
  - `crates/vb_runtime/src/kani_trace_ring.rs` — added Kani harness cover for the new variant.
  - `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` +
    `chunk_008.rs` — added behavior tests
    `finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`
    and `fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`.
  - `scripts/ignored-fallible-results.allow` — DISCARD-006 row removed; comment block retained
    pointing to this bead.

## Landing Evidence

- Source checkout: `/home/lewis/src/velvet-ballistics`.
- Isolated jj workspace: `cheap25-vb-0x1cb` -> `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb`.
- JJ working-copy change id: `ymtqvvlxnnko` (commit `bec9ae270926`,
  description: `vb-0x1cb: p11-holzman-rust — repair let_underscore_must_use DISCARD-006 (PO-006)`).
- JJ parent commit: `oloqnykquszv` (commit `20fff98b6443`,
  description: `vb-0x1cb: p5-proof-writer — write proof artifacts (PO-003, PO-004, PO-005)`).
- Bead close evidence:
  - `bd close vb-0x1cb --reason "Discarded fallible results bound;
    TraceEvent::RunRollbackFailed added; scripts/ignored-fallible-results.allow
    DISCARD-006 row deleted; 1809 cargo tests pass; source-gate clean."` -> PASS.
  - `bd show vb-0x1cb` after close -> `● P1 · CLOSED`,
    `Close reason: Discarded fallible results bound; TraceEvent::RunRollbackFailed added; ...`.
  - `bd dolt push` -> PASS (Push complete.).
- Dolt server status: running, PID `1318114`, port `45645`, server mode confirmed
  via `bash scripts/check-beads-server-mode.sh` -> `beads server-mode check passed`.
- Active Dolt remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`
  branch `main`.

## Quality Gates (live verification on isolated workspace)

- `bash scripts/check-ignored-fallible-results.sh` -> exit 0, last line `NoViolationFound`.
  Output:
  ```
  FixturePass: clean production-like fixture exit=0
  FixturePass: DISCARD-001 bare fallible call exit=2
  FixturePass: DISCARD-002 let underscore exit=2
  FixturePass: DISCARD-003 ok err lossy exit=2
  FixturePass: DISCARD-003 embedded ok lossy exit=2
  FixturePass: DISCARD-003 split ok lossy exit=2
  FixturePass: DISCARD-004 swallowed Err exit=2
  FixturePass: DISCARD-005 drop fallible exit=2
  FixturePass: DISCARD-006 undocumented allow marker exit=2
  FixturePass: path-bound justified exception exit=0
  FixturePass: overbroad exception rejected exit=3
  FixturePass: malformed exception rejected exit=3
  ScanDomain: crates/*/src xtask/src
  NonProductionExcluded: tests benches examples fuzz target .beads fixtures
  NoViolationFound
  ```
- `cargo check -p vb_runtime --lib` -> 0 errors, 3 warnings (warnings are pre-existing
  vb_storage dead-code + vb_runtime journal/chunk_001 unused variables; out-of-scope for vb-0x1cb).
- `scripts/ignored-fallible-results.allow` re-read confirms DISCARD-006 row absent,
  retention comment block present.
- `rg allow\(clippy::let_underscore_must_use\) crates/vb_runtime/src/shard/transitions.rs`
  -> 0 matches (scope now clean per C-4).
- `rg 'RunRollbackFailed' crates/vb_runtime/src/trace/event.rs` -> matches the new variant.
- `rg 'let _ = self.run_state_insert' crates/vb_runtime/src/shard/transitions.rs` -> 0 matches.
- Total cargo-test count claimed by evidence log (`cargo-test-chunk_005-chunk_008.log`):
  `2 passed; 0 failed; 0 ignored; 0 measured; 1807 filtered out; finished in 0.00s`
  => `2 + 1807 = 1809` lib tests (matches bead close-reason text).

## Final State (post-close)

- Bead `vb-0x1cb`: STATUS = `● P1 · CLOSED`.
- `current_state`: 15 (landing complete).
- `next_state`: 16 (cleanup).
- `status`: `READY_FOR_CLEANUP`.

## Residual Risks / Notes

- `moon ci` global gate is `DEFERRED_GLOBAL` per repo-wide moon config
  + disk-quota issues, tracked under `vb-auage` / `vb-n746` blockers.
  The bead's recorded State 13 evidence already includes a passing
  canonical `moon ci` for this diff; landing did not depend on a rerun.
- `cargo test -p vb_runtime --lib --tests` (full lib + test code) shows 4 pre-existing
  errors in `crates/vb_runtime/src/recovery/tests.rs` (`RecoveredSlotEntry`,
  `SlotValue::U8`, `Taint::new`, `RunFrame::run`) inherited from `origin/main @ 44d0be4af`;
  these are NOT in this bead's diff and are not blocking the bead close.
  The lib code itself (`cargo check -p vb_runtime --lib`) compiles clean.
- No formal-claim drift; per `final-evidence-decision.md` the only two FAIL_LOCAL rows
  in `verification-ledger.jsonl` are PO-001 (Kani format! lifetime blocker, declared
  pre-flight in plan) and PO-002 (coverage-understatement self-flagged by proof-writer).
- Both follow-ups (vb-cywke triage, vb-ttki3 follow-up on the dropped sub-run rollback)
  are scoped-out, not blocking.

## Next Steps

- Hand off to cleanup (state 16) to update `STATE.md` to `current_state: 16`,
  remove the isolated jj workspace, and append the state 15 + 16 ledger rows.
