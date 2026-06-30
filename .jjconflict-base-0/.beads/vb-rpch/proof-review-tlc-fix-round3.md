# Proof Review — TLC Fix Round 3 for `vb-rpch`

Provenance:
- Reviewer agent: `proof-reviewer`
- Review date: 2026-05-24
- Workspace: `/home/lewis/src/vb-jpq7-jj-fix`
- Bead: `vb-rpch`
- Review scope: final adversarial review of TLC fix round 3 causal repair artifacts only.
- Rubrics loaded: `proof-reviewer`; `tla-plus`.
- Files written by reviewer: `.beads/vb-rpch/proof-review-tlc-fix-round3.md`, `.beads/vb-rpch/proof-findings-tlc-fix-round3.jsonl`.
- Reviewer command evidence: JSONL/log/source-sync validation command exited successfully and reported both round-3 JSONL files parse, all required logs contain expected outcomes, and source/evidence TLA/CFG copies match.
- No production code, tests, TLA specs, CFGs, proof artifacts, verifier harnesses, or evidence logs were repaired by this reviewer.

## Findings

No critical or high blocking findings remain open for the round-3 TLC lane.

### MEDIUM — Bounded finite TLA abstraction only

Artifact/evidence:
- `specs/tla/RecoveryReplayFull.cfg` lines 3-14 bound the primary model to singleton run/step/action/attempt domains, `MAX_SEQ = 3`, `MAX_EVENTS = 3`, equal ABI/policy inputs, and no corrupt snapshot input.
- `evidence/specs/RecoveryReplayFull.tlc.log` lines 22-28 show primary TLC completed with `31707925 states generated`, `3448392 distinct states found`, `0 states left on queue`, depth `12`, and no errors.
- `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl` line 1 records the finite-bound limitation.

Disposition: non-blocking caveat. This approval is only for the stated finite TLA abstraction. It is not a proof over larger domains, byte decoding, cryptographic digest computation, runtime ABI/policy lookup, scheduler queue implementation details, or liveness/fairness.

### LOW — Optional large stress target remains waived

Artifact/evidence:
- `.beads/vb-rpch/verification-ledger.tlc-fix-round3.jsonl` line 2 classifies `TLC-R2-002` as `WAIVED` and required false.
- `.beads/vb-rpch/formal-verification-report-tlc-fix-round3.md` lines 93-98 states the optional large stress cfg was not run and no proof claim depends on it.

Disposition: non-blocking caveat. The large cfg is not proof evidence and must not be cited as closure.

## Prior blocking finding closure

### Closed — Round 2 ABI/policy mismatch tautologies

Prior finding: `FINDING-TLC-R2-001` against `TLC-R2-004`, `TLC-R2-015`, `TLC-R2-016`.

Reviewed evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 263-269 now define `CheckActionAbiDigest` as `abi_expected # abi_found` and `CheckPolicyDigest` as `policy_expected # policy_found`; the stale tautological guard `0 # 1` is absent.
- `specs/tla/RecoveryReplayFull-error-action-abi-mismatch.cfg` lines 8-11 sets `ABIExpected = 1`, `ABIFound = 2`, and policy equal.
- `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log` lines 20-50 shows expected violation of `NotReachErrorActionAbiMismatch`, initial `abi_expected = 1`, `abi_found = 2`, and `CheckActionAbiDigest` recording `ActionAbiMismatch`.
- `specs/tla/RecoveryReplayFull-error-policy-digest-mismatch.cfg` lines 8-11 sets ABI equal and `PolicyExpected = 1`, `PolicyFound = 2`.
- `evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log` lines 20-50 shows expected violation of `NotReachErrorPolicyDigestMismatch`, initial `policy_expected = 1`, `policy_found = 2`, and `CheckPolicyDigest` recording `PolicyDigestMismatch`.

Verdict: fixed for abstract finite causal reachability. Runtime lookup and cryptographic computation remain outside this proof, correctly ledgered in `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl` line 2.

### Closed — Round 2 corrupt snapshot modeled as absent snapshot

Prior finding: `FINDING-TLC-R2-003` against `TLC-R2-004`, `TLC-R2-014`.

Reviewed evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 258-261 now require `snapshot_seq >= 0` and `"Corrupt" \in snapshot_inputs` before `CorruptSnapshot`.
- `specs/tla/RecoveryReplayFull-error-corrupt-snapshot.cfg` lines 8-13 sets equal ABI/policy inputs, `SnapshotInputs = {"Corrupt"}`, `MAX_SEQ = 1`, `MAX_EVENTS = 1`.
- `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log` lines 20-65 shows expected violation of `NotReachErrorCorruptSnapshot`, initial `snapshot_inputs = {"Corrupt"}`, `SetSnapshot` to `snapshot_seq = 0`, then `LoadCorruptSnapshot` recording `CorruptSnapshot`.

Verdict: fixed for abstract corrupt-marker reachability. Byte-level snapshot decoding remains outside this TLA proof, correctly ledgered in `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl` line 3.

### Closed — Round 2 non-idempotent replay witness was only a completed event

Prior finding: `FINDING-TLC-R2-002` against `TLC-R2-009`, `TLC-R2-017`.

Reviewed evidence:
- `specs/tla/RecoveryReplayFull.tla` lines 219-227 define `MarkReplayCandidate` from an `ActionCompleted`/`ActionFailed` event into `replay_candidates`.
- `specs/tla/RecoveryReplayFull.tla` lines 271-277 define `DetectNonIdempotentResolved` requiring a resolved event and a replay candidate with matching action/step/attempt before recording `NonIdempotentActionBlocked`.
- `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log` lines 20-68 shows expected violation of `NotReachResolvedActionGuardAntecedent`, with `ActionCompleted` then `MarkReplayCandidate` and `replay_candidates = {[step |-> 1, action |-> 1, attempt |-> 1]}`.
- `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log` lines 20-83 shows `ActionCompleted`, then `MarkReplayCandidate`, then `DetectNonIdempotentResolved` recording `NonIdempotentActionBlocked` with the same action/step/attempt candidate.

Verdict: fixed for the model's abstract candidate-before-mutation representation. The exact scheduler queue implementation remains outside this TLC evidence, correctly ledgered in `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl` line 4.

## Additional adversarial checks

- Stale partial operators: no occurrence found in current `specs/tla/RecoveryReplayFull.tla` for `UnmodeledRecoveryErrors`, `RecoveryErrorExhaustiveScopedPartial`, legacy weak `ReachResolvedActionAntecedent`, or tautological `0 # 1`.
- JSONL parse: `.beads/vb-rpch/trusted-base-ledger.tlc-fix-round3.jsonl` parsed successfully with 6 rows; `.beads/vb-rpch/verification-ledger.tlc-fix-round3.jsonl` parsed successfully with 20 rows.
- Primary bounded TLC: `evidence/specs/RecoveryReplayFull.tlc.log` lines 22-28 match the claimed no-error completion, state counts, drained queue, and depth 12.
- Independent non-vacuity witnesses: all required logs from `TLC-R2-005` through `TLC-R2-010` contain the expected invariant violations; the strengthened no-reexecution witness contains the resolved action plus matching replay candidate.
- Per-error witnesses: all required logs from `TLC-R2-011` through `TLC-R2-019` contain the expected invariant violations; the repaired causal traces for corrupt snapshot, ABI mismatch, policy mismatch, and non-idempotent blocking were inspected directly.
- Digest ordering: `evidence/specs/RecoveryReplayFull-error-compiled-ir-digest-mismatch.tlc.log` lines 67-95 shows `WorkflowChecked` before `CheckIrDigest` records `CompiledIrDigestMismatch`; `evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log` lines 67-95 shows `WorkflowChecked` before `IrChecked`.
- Source/evidence sync: reviewer validation compared current `specs/tla/RecoveryReplayFull.tla` and all `specs/tla/RecoveryReplayFull*.cfg` against `evidence/specs/` byte-for-byte and reported `SYNC_OK all RecoveryReplayFull.tla/cfg`.

## Approval scope

This review approves the round-3 TLC lane only as bounded finite TLA+ evidence for the modeled recovery replay abstraction and the causal reachability witnesses listed above. It does not prove byte decoding, cryptographic digest computation, runtime ABI lookup, runtime policy lookup, exact scheduler queue mechanics, Rust implementation refinement, or liveness/fairness. Those remain bridge/evidence-packaging obligations or accepted caveats as applicable.

STATUS: APPROVED
