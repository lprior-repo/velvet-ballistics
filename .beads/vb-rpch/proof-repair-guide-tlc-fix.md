# Proof Repair Guide — TLC Fix Pass for `vb-rpch`

Provenance:
- Reviewer agent: `proof-reviewer`
- Review date: 2026-05-24
- Workspace: `/home/lewis/src/vb-jpq7-jj-fix`
- Bead: `vb-rpch`
- Status: rejected repair guide for TLC-fix pass.

## Required repairs before re-review

1. Close or honestly downgrade the primary TLC run.
   - Required exact command from repo root:
     ```bash
     tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
     ```
   - Approval requires final `Model checking completed. No error has been found.`, final state counts, `0 states left on queue`, and graph depth/diameter.
   - If it cannot finish, produce a revised proof plan with smaller contract-preserving bounds/reductions and do not claim exhaustive proof for the current primary cfg.

2. Split non-vacuity into independent reachability checks.
   - Required shape:
     ```bash
     tlc -config specs/tla/RecoveryReplayFull-nonvacuity-replay-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log 2>&1
     tlc -config specs/tla/RecoveryReplayFull-nonvacuity-snapshot-tail.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log 2>&1
     tlc -config specs/tla/RecoveryReplayFull-nonvacuity-incomplete-runs.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log 2>&1
     tlc -config specs/tla/RecoveryReplayFull-nonvacuity-no-reexecution.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log 2>&1
     tlc -config specs/tla/RecoveryReplayFull-nonvacuity-digest-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log 2>&1
     ```
   - Each run must intentionally violate a negated reachability invariant and include the raw counterexample trace.

3. Repair or downgrade `RecoveryErrorExhaustive`.
   - Add causal model actions and per-variant reachability checks for every `ErrorDomain` variant, then run per-error TLC witness commands; or remove the claim from proven coverage and submit an explicit waiver.
   - Do not use `last_error \in ErrorDomain` or `RecoveryErrorExhaustiveScopedPartial` as proof of exhaustiveness.

4. Repair `DigestVerificationOrder` semantics.
   - Add explicit state recording workflow-check completion and IR-check completion.
   - Prove IR check cannot occur before workflow check.
   - Add a non-vacuity trace that reaches both successful and mismatch digest paths.

5. Re-run evidence sync after proof repairs.
   ```bash
   cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && \
   cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && \
   cmp -s specs/tla/RecoveryReplayFull-smoke.cfg evidence/specs/RecoveryReplayFull-smoke.cfg && \
   cmp -s specs/tla/RecoveryReplayFull-nonvacuity.cfg evidence/specs/RecoveryReplayFull-nonvacuity.cfg
   ```

6. Rebuild ledgers/reports honestly.
   - `verification-ledger.tlc-fix.jsonl` must mark each obligation `PASS` only with raw final evidence or `FAIL_LOCAL`/`PARTIAL` if incomplete.
   - `trusted-base-ledger.tlc-fix.jsonl` must record every finite bound, event reduction, symmetry/reduction, and pending partial claim.

## Re-review command checklist

Before requesting proof-reviewer again, provide raw logs for:

```bash
tlc -config specs/tla/RecoveryReplayFull-smoke.cfg specs/tla/RecoveryReplayFull.tla
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla
tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla
```

Plus any new per-antecedent/per-error non-vacuity commands introduced by the repair.

STATUS: REJECTED
