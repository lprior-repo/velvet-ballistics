# Proof Writer Report — TLC Fix Round 3 for `vb-rpch`

Status: repair artifacts only. This is not proof approval.

## Scope

- Workspace: `/home/lewis/src/vb-jpq7-jj-fix`
- Bead: `vb-rpch`
- Repaired obligations: `TLC-R2-001`, `TLC-R2-004`, `TLC-R2-009`, `TLC-R2-014`, `TLC-R2-015`, `TLC-R2-016`, `TLC-R2-017`, `TLC-R2-020`.
- Edited only TLA+/CFG/evidence/report artifacts.

## Semantic repairs

1. `TLC-R2-004` / `TLC-R2-015` / `TLC-R2-016`: replaced tautological ABI/policy mismatch actions with finite modeled input variables initialized from CFG constants:
   - `abi_expected`, `abi_found`
   - `policy_expected`, `policy_found`
   - `CheckActionAbiDigest` requires `abi_expected # abi_found`.
   - `CheckPolicyDigest` requires `policy_expected # policy_found`.
   - Witness CFGs set only the targeted mismatch unequal, so raw TLC traces display causal mismatch inputs.

2. `TLC-R2-009` / `TLC-R2-017`: strengthened non-idempotent replay modeling:
   - Added `replay_candidates` state and `MarkReplayCandidate` action.
   - `DetectNonIdempotentResolved` now requires both a prior `ActionCompleted`/`ActionFailed` and a later same action/step/attempt replay candidate.
   - `ReachResolvedActionGuardAntecedent` now requires the same resolved-event + replay-candidate pair.

3. `TLC-R2-004` / `TLC-R2-014`: separated corrupt snapshot from no snapshot:
   - Added `snapshot_inputs` finite input set initialized from `SnapshotInputs` CFG constant.
   - `LoadCorruptSnapshot` now requires `snapshot_seq >= 0` and `"Corrupt" \in snapshot_inputs`.
   - The corrupt snapshot witness trace now sets a snapshot before recording `CorruptSnapshot`.

4. `TLC-R2-001`: kept primary CFG bounded and feasible:
   - singleton run/step/action/attempt domains
   - `MAX_SEQ = 3`
   - `MAX_EVENTS = 3`
   - ABI/policy inputs equal and `SnapshotInputs = {}` in primary CFG.

5. `TLC-R2-020`: synced source/evidence copies for `RecoveryReplayFull.tla` and all `RecoveryReplayFull*.cfg` files under `evidence/specs/`.

6. `TLC-R2-004` / `TLC-R2-009` / `TLC-R2-017`: removed stale round-2 partial-exhaustiveness claims:
   - Replaced `UnmodeledRecoveryErrors` and `RecoveryErrorExhaustiveScopedPartial` with `ModeledRecoveryErrors == ErrorDomain \ {NoneError}` and `RecoveryErrorWitnessCoverageClaim` naming all nine modeled non-`None` variants.
   - Removed the misleading `UnmodeledRecoveryErrors # {}` claim; round 3 now has causal witness transitions for every modeled non-`None` `ErrorDomain` variant.
   - Strengthened `AllNonVacuityWitnessesReached` to use `ReachResolvedActionGuardAntecedent` and removed the unused weak legacy resolved-action predicate.

## Files changed

- `specs/tla/RecoveryReplayFull.tla`
- `specs/tla/RecoveryReplayFull*.cfg`
- `evidence/specs/RecoveryReplayFull.tla`
- `evidence/specs/RecoveryReplayFull*.cfg`
- `evidence/specs/RecoveryReplayFull*.tlc.log` for commands rerun in round 3
- `.beads/vb-rpch/proof-writer-report-tlc-fix-round3.md`
- `.beads/vb-rpch/proof-evidence-tlc-fix-round3.md`
- `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl`

## Commands run

See `.beads/vb-rpch/proof-evidence-tlc-fix-round3.md` for exact commands and raw evidence paths.

## Residual limitations

- TLA+ models finite abstract digest/snapshot inputs only. It does not prove byte-level snapshot decoding, cryptographic digest computation, or deferred runtime ABI/policy lookup implementation.
- Primary TLC evidence is bounded to the stated CFG domains and limits.
- Witness CFGs intentionally use expected invariant violations to demonstrate reachability; they are not primary safety proofs.
- `RecoveryErrorWitnessCoverageClaim` is a model-domain coverage claim over the finite TLA `ErrorDomain`; per-error causal reachability remains evidenced by the individual witness CFG/log files rather than by one simultaneous state predicate.
