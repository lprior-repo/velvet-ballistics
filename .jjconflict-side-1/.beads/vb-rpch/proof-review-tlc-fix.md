# Proof Review — TLC Fix Pass for `vb-rpch`

Provenance:
- Reviewer agent: `proof-reviewer`
- Review date: 2026-05-24
- Workspace: `/home/lewis/src/vb-jpq7-jj-fix`
- Bead: `vb-rpch`
- Review scope: adversarial review of TLC-fix planner, proof-writer, and formal-verifier artifacts only.
- Rubrics loaded: `proof-reviewer`; `tla-plus`.
- Files written by reviewer: `.beads/vb-rpch/proof-review-tlc-fix.md`, `.beads/vb-rpch/proof-findings-tlc-fix.jsonl`, `.beads/vb-rpch/proof-repair-guide-tlc-fix.md`.
- JSONL parse check: `proof-obligations.tlc-fix.planned.jsonl` OK 8 rows; `verifier-lane-decisions.tlc-fix.jsonl` OK 4 rows; `delivery-scope-tlc-fix.jsonl` OK 11 rows; `trusted-base-ledger.tlc-fix.jsonl` OK 9 rows; `verification-ledger.tlc-fix.jsonl` OK 8 rows.

## Findings

### CRITICAL — `TLC-FIX-006` did not close; primary cfg is partial BFS, not proof

Artifact/evidence:
- `.beads/vb-rpch/verification-ledger.tlc-fix.jsonl` rows 2, 3, 4, 6 classify primary invariant obligations as `FAIL_LOCAL`.
- `.beads/vb-rpch/formal-verification-report-tlc-fix.md` lines 63-94 states the primary command timed out.
- `evidence/specs/RecoveryReplayFull.formal-timeout.tlc.log` lines 22-28 show `34,905,320 states generated`, `34,899,577 distinct`, `34,898,477 states left on queue`, and runner termination after `180000 ms`.

This cannot advance as exhaustive TLC evidence. The smoke cfg is useful, but it is not the required primary bounded model.

### CRITICAL — `TLC-FIX-007` / `TLA-005 RecoveryErrorExhaustive` remains unclosed

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 37, 167-170, 172-191, 260-264.
- `.beads/vb-rpch/proof-evidence-tlc-fix.md` lines 115-117 explicitly says most error variants are not causally generated.
- `.beads/vb-rpch/trusted-base-ledger.tlc-fix.jsonl` row 8 marks the claim `PARTIAL_PENDING`.

`last_error \in ErrorDomain` is a type membership fact, not reachability. Only digest mismatch errors have causal transitions. The named variants `NoRecoveryData`, `CorruptSnapshot`, `ActionAbiMismatch`, `PolicyDigestMismatch`, `NonIdempotentActionBlocked`, `ReplayDivergence`, and `FrameDimensionOverflow` are not exhaustively reached from defined inputs.

### HIGH — Non-vacuity evidence is a single reduced witness and does not cover every obligation antecedent

Artifact/evidence:
- `specs/tla/RecoveryReplayFull-nonvacuity.cfg` lines 3-9 restrict domains and `EnabledEventTypes` to `RunAccepted`, `ActionCompleted`, `RunFinished`.
- `specs/tla/RecoveryReplayFull.tla` lines 266-274 define one combined `AllNonVacuityWitnessesReached` predicate.
- `evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log` lines 20-72 show the expected violation, but it stops at the first combined witness with `55,340` states still on queue.

This is acceptable as one reachability witness only. It does not independently prove each planned non-vacuity target, does not exercise `ReachModeledDigestError`, does not reach per-error variants, and weakens `NoResolvedReExecution` non-vacuity to merely having an `ActionCompleted` event rather than a later same action schedule candidate being blocked.

### HIGH — `DigestVerificationOrder` is toy/tautological and not an order proof

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 161-165 permit `DigestCheckNext` to set any digest level nondeterministically.
- `specs/tla/RecoveryReplayFull.tla` lines 172-191 model workflow/IR checks but do not record check history or enforce workflow-before-IR through an invariant.
- `specs/tla/RecoveryReplayFull.tla` lines 219-223 define `DigestVerificationOrder` as nonzero digest fields on `RunAccepted`; `AppendEvent` creates all events with digest values `1, 1` at lines 225-229.

The invariant does not state or prove verification order. It is satisfied by construction because appended events always have nonzero digest fields. The model needs explicit check-stage/history state or the claim must be downgraded.

### MEDIUM — Model reductions and semantics are ledgered but not tied tightly enough to the contract

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.tla` line 21 introduces `EnabledEventTypes`.
- `.beads/vb-rpch/trusted-base-ledger.tlc-fix.jsonl` row 7 correctly admits non-vacuity uses a narrowed event set.
- `specs/tla/RecoveryReplayFull.tla` lines 128-133 model `SetSnapshot` as an abstract marker with no snapshot payload.

The reductions are disclosed, which is good. They are not enough for approval because the bridge to the recovery contract is still blocked, especially snapshot payload semantics and digest/error semantics.

## What is actually good

- `TLC-FIX-001` parser/config-loader acceptance is credible for the current files: the primary run parsed and semantically processed the module before timeout.
- `TLC-FIX-004` removed the previous tautological `PROPERTY Spec` from the primary cfg.
- `TLC-FIX-005` smoke run passed exhaustively with `5,883,676` states generated, `505,140` distinct states, `0` queue, graph depth `9`.
- `TLC-FIX-007` produced a useful, scoped reachability witness for the combined modeled antecedents; it is not a full proof.
- `TLC-FIX-008` source/evidence sync is credible for TLA/cfg copies.

## Verdict

This pass cannot advance. Required primary TLC proof did not complete, `RecoveryErrorExhaustive` is explicitly partial, non-vacuity is scoped and reduced, and `DigestVerificationOrder` does not encode order semantics.

STATUS: REJECTED
