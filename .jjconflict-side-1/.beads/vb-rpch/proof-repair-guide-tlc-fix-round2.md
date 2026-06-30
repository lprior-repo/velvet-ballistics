# Proof Repair Guide — TLC Fix Round 2 for `vb-rpch`

Status: repair guidance only. Do not treat this document as proof approval.

## Blocking repairs required

1. Replace ABI/policy tautological error actions.
   - `CheckActionAbiDigest` and `CheckPolicyDigest` must not be guarded by `0 # 1` alone.
   - Add finite abstract expected/found inputs or equivalent modeled mismatch state.
   - Reachability witnesses must show the mismatch input causes `ActionAbiMismatch` / `PolicyDigestMismatch`.
   - Keep the existing caveat that this does not prove deferred GAP-3 runtime lookup implementation.

2. Strengthen non-idempotent replay modeling.
   - A completed/failed action alone is insufficient.
   - Model a later replay/schedule candidate for the same action/step/attempt and prove it is blocked or records `NonIdempotentActionBlocked`.
   - Update `RecoveryReplayFull-nonvacuity-no-reexecution.cfg` so its trace cannot stop after a single `ActionCompleted` event.

3. Separate corrupt snapshot from no snapshot.
   - `snapshot_seq = -1` is absence/initial state, not a corrupt snapshot input.
   - Add a finite abstract corrupt marker or snapshot/run mismatch condition and require it before `CorruptSnapshot`.
   - Preserve the caveat that TLA does not prove byte-level snapshot parsing/decoding.

## Evidence to regenerate after repair

- Primary bounded TLC log: `evidence/specs/RecoveryReplayFull.tlc.log`.
- Affected witness logs at minimum:
  - `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log`
  - `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log`
  - `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log`
  - `evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log`
  - `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log`
- Source/evidence sync command must pass again after model/config edits.

## Review gate

Request another proof-review only after raw logs show the expected causal traces, not just the expected invariant names.

STATUS: REJECTED
