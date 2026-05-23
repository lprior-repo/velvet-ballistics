# Proof Review: vb-jpq7.3

Reviewer: proof-reviewer  
Date: 2026-05-23  
Review state: refreshed after snapshot-authority test repairs and global-readiness artifact update  
Scope: `.beads/vb-jpq7.3/*`, `verification/tla/EngineYamlRecovery.*`, `verification/verus/recovery_hydration_contracts.rs`, `verification/verus/vb_jpq724_events_for_run_production.rs`, and touched storage/recovery source/tests.

## Verdict

REJECT.

The behavior-test surface is materially stronger than the previous review: current tests now cover corrupt latest snapshot magic, payload digest mismatch, postcard decode failure, snapshot payload run mismatch, snapshot payload sequence mismatch, strict `snapshot.seq + 1` tail gaps, bounded replay, taint read fail-closed, and explicit close/persist failure. That removes the earlier snapshot-authority behavior-test concern.

It does **not** close the formal proof gate. The formal artifacts remain too coarse/disconnected for the bead's critical claims, Kani is still unresolved, and the canonical global readiness gate is still blocked by `moon ci`.

## Current Evidence Inspected / Re-run

- `verification-ledger.jsonl` now records:
  - `cargo fmt --all -- --check`: PASS.
  - `moon ci`: FAIL / `BLOCK_GLOBAL`, raw log `/home/lewis/.local/share/opencode/tool-output/tool_e53cb9935001x2youOsXWkFzMl`.
  - `cargo test -p vb_storage events_for_run`: PASS, 24 tests, including digest/postcard snapshot failures.
  - `cargo test -p vb_storage latest_durable_snapshot_seq`: PASS, 4 tests, including run/seq mismatch.
  - `cargo test -p vb_storage trimming`: PASS, 25 tests.
- Re-ran formal checks:
  - `tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`: PASS, `838 states generated, 387 distinct states found, 0 states left on queue`.
  - `verus verification/verus/recovery_hydration_contracts.rs`: PASS, `10 verified, 0 errors` with deprecation warnings.
  - `verus verification/verus/vb_jpq724_events_for_run_production.rs`: PASS, `4 verified, 0 errors` with automatic-trigger notes.
- Source/test spot checks:
  - `crates/vb_storage/src/journal/tests.rs:1786-1862` covers latest snapshot payload digest mismatch and postcard decode failure before tail replay.
  - `crates/vb_storage/src/trimming/tests.rs:362-423` covers payload run mismatch and payload seq mismatch.
  - `crates/vb_storage/src/journal/replay.rs:24-31` propagates `latest_durable_snapshot_seq(run)?` and starts tail at `next_seq(snapshot_seq)?`.
  - `crates/vb_storage/src/trimming/logic.rs:34-48` decodes snapshot payload and checks run/seq consistency before trusting key authority.

## Findings

### CRITICAL — POT-REPLAY-001 / POT-SNAPSHOT-001: TLA+ still does not model the bead's critical storage/replay semantics

Artifacts: `verification/tla/EngineYamlRecovery.tla`, `verification/tla/EngineYamlRecovery.cfg`, `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`.

Current evidence:

- TLC passes on a tiny lifecycle model: `838 states generated, 387 distinct states found`.
- `EngineYamlRecovery.tla` still has only `recovery_source \in {"durable", "corrupt", "mismatch", "empty"}` and a generic `Replay == seq < 3 /\ seq' = seq + 1`.
- The lane decision still explicitly says the model does not encode concrete `EventSeq N+1` tail arithmetic or corrupt latest snapshot authority.

Why this remains blocking: The passing model is non-vacuous for its own toy state space, but it does not prove `snapshot.seq + 1`, latest snapshot key/payload authority, corrupt latest snapshot failure before replay, payload digest mismatch, postcard decode failure, run/seq mismatch, `SequenceGap`, `BadMagic`, or bounded `EventSeq` overflow behavior. The newly added behavior tests cover these cases dynamically; the TLA+ proof still does not.

Required repair/command:

```bash
tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla
```

after strengthening the model/config to include bounded `EventSeq`, snapshot key/payload records, decode validity states, corrupt/digest/postcard/mismatch latest snapshot outcomes, `tail_start = snapshot_seq + 1`, missing-first-tail `SequenceGap`, and overflow/fail-closed transitions.

### CRITICAL — POT-REPLAY-001 / Verus replay lane: replay proof remains disconnected and stale relative to strict tail semantics

Artifact: `verification/verus/vb_jpq724_events_for_run_production.rs`.

Current evidence:

- Verus passes: `verification results:: 4 verified, 0 errors`.
- The file still uses mirror-only `SpecRunId`, `SpecEventSeq`, and `SpecJournalEvent` rather than production exec functions/types.
- Its comment is stale and contradicts the repaired implementation: it says production uses `latest_durable_snapshot_seq(run).unwrap_or(EventSeq::ZERO)` and delegates from the snapshot sequence; current production propagates errors and starts at `next_seq(snapshot_seq)?`.
- The predicate only requires `events[i].seq >= start_seq` and adjacent `+1`; it does not require `events[0].seq == snapshot.seq + 1`.
- `Err(()) => true` still leaves all error cases unconstrained.

Why this remains blocking: The proof would continue to verify if production regressed to accepting a first tail event greater than `snapshot.seq + 1`, erased snapshot decode errors, or returned the wrong typed error. It is an auxiliary abstract invariant, not production-linked closure evidence.

Required repair/command:

```bash
verus verification/verus/vb_jpq724_events_for_run_production.rs
```

after replacing/augmenting it with production-bound contracts or a reviewer-accepted bridge that proves first-tail equality, typed error constraints, and linkage to `crates/vb_storage/src/journal/replay.rs:24-31` and `:69-77`.

### HIGH — POT-TAINT-001: recovery Verus remains auxiliary only

Artifact: `verification/verus/recovery_hydration_contracts.rs`.

Current evidence:

- Verus passes: `verification results:: 10 verified, 0 errors`.
- The artifact proves an abstract `SpecRecoveryInput` / `recovery_decision` lattice.
- It does not bind to `apply_tail_events`, `RunFrame::read_taint`, or `RecoveryError::SlotTaintReadFailed` production code.

Why this remains blocking if claimed as formal closure: behavior tests prove the taint failure case, but this Verus file is not an implementation proof. It may be accepted only as auxiliary/design evidence unless a bridge explicitly narrows the claim.

Required repair/command:

```bash
verus verification/verus/recovery_hydration_contracts.rs
```

after either binding the proof to production-shaped recovery semantics or documenting it as auxiliary and satisfying the required formal lane elsewhere.

### HIGH — Kani lane remains unresolved and the recorded command remains invalid

Artifacts: `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`.

Current evidence:

- Lane status remains `candidate-blocker`.
- Artifact list remains empty.
- Recorded command remains `cargo kani list -p vb_storage`, which is not a valid `cargo kani list` invocation for this Kani version.
- No scoped Kani harness pass, coverage/reachability output, or approved waiver is present.

Why this remains blocking: this bead centers on critical replay arithmetic and fail-closed error-lattice behavior. Kani cannot remain an unresolved candidate with no approved disposition.

Required repair/commands:

Either record a schema-valid, reviewer-approved Kani waiver, or add scoped non-hardcoded harnesses and run, for example:

```bash
cargo kani -p vb_storage --harness <vb_jpq7_3_replay_tail_harness> --coverage
cargo kani -p vb_storage --harness <vb_jpq7_3_snapshot_authority_harness> --coverage
```

The harnesses must use arbitrary/generator inputs rather than hardcoded dummy shapes, and must include cover/non-vacuity evidence.

### HIGH — Proof plan / lane artifacts are still rejected and schema-invalid

Artifacts: `.beads/vb-jpq7.3/proof-plan-review.md`, `.beads/vb-jpq7.3/verifier-lane-review.md`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`, `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`.

Current evidence:

- `agent-invocation-ledger.jsonl` records `proof-plan-reviewer` status `reject`.
- `proof-plan-review.md` rejects schema/lane completeness, TLA/Verus gaps, Kani unresolved state, missing per-requirement lane matrix, and missing Flux/proptest/fuzz split.
- `verifier-lane-review.md` rejects every submitted lane row and says `flux-rs`, separate `proptest`, and separate `cargo-fuzz` lane reviews are missing.

Why this remains blocking: proof-review cannot approve closure on top of rejected proof-plan/lane artifacts unless the formal package is repaired or explicit approved waivers are present.

Required repair:

Regenerate schema-valid proof obligations, lane decisions, verification ledger rows, and waiver rows; rerun proof-plan-reviewer; then rerun proof-reviewer.

### MEDIUM — Evidence ledger still records mostly summaries, not raw logs, for scoped behavior passes

Artifact: `.beads/vb-jpq7.3/verification-ledger.jsonl`.

Current evidence:

- The `moon ci` failure has a raw log path.
- Most PASS rows still use summaries such as `24 passed; 0 failed`, `4 passed; 0 failed`, or `NoViolationFound`, not durable raw output references.

Why this matters: summaries are adequate for quick triage but are not final proof evidence under the evidence standard. This is lower severity than the disconnected formal lanes, but it still blocks final assurance packaging unless raw logs are recorded or explicitly waived.

Required commands to capture raw logs:

```bash
rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run
rustup run nightly-2026-04-28 cargo test -p vb_storage latest_durable_snapshot_seq
rustup run nightly-2026-04-28 cargo test -p vb_storage trimming
rustup run nightly-2026-04-28 cargo test -p vb_storage recovery
rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract
bash scripts/check-ignored-fallible-results.sh
```

### MEDIUM — Global readiness remains blocked, now by canonical `moon ci`

Artifact: `.beads/vb-jpq7.3/global-readiness-report.md`, `.beads/vb-jpq7.3/verification-ledger.jsonl`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`.

Current evidence:

- `cargo fmt --all -- --check` is now PASS.
- `moon ci` is FAIL / `BLOCK_GLOBAL` with raw log `/home/lewis/.local/share/opencode/tool-output/tool_e53cb9935001x2youOsXWkFzMl`.
- Reported failures: `velvet-ballastics:panic-surface` on `crates/vb_codegen/src/parity.rs:438` and `:444`, plus `velvet-ballastics:check` workspace-test dead-code warnings.

Why this matters: this is not a storage/recovery formal proof defect, but `POT-GLOBAL-001` is a required closure obligation. It blocks bead closure without repair or explicit release-owner waiver.

Required command after repair/waiver:

```bash
moon ci
```

## Positive Current Assessment

- Behavior coverage for snapshot authority is now substantially better and includes run mismatch, sequence mismatch, digest mismatch, postcard decode failure, and corrupt magic.
- The storage/recovery implementation remains production-linked in the dynamic test suite for strict tail replay, error propagation, taint read failure, bounded replay, and close/persist failure.
- The current remaining blockers are formal/proof-package quality and global readiness, not an obvious missing behavior test in the inspected vb-jpq7.3 blast radius.

## Decision

REJECT. Do not close vb-jpq7.3 as formally proven. Behavior tests are stronger, but required formal artifacts remain disconnected/coarse, Kani is unresolved, proof-plan/lane artifacts remain rejected, raw evidence is incomplete, and `moon ci` is still blocked.

STATUS: REJECTED
