# Proof Writer Report — TLC Fix Round 2 (`vb-rpch`)

Status: artifacts repaired and bounded TLC evidence captured. This report does **not** approve the proof; it is input for the next proof-review/formal-verifier gate.

## Obligations touched

- `TLC-R2-001`: replaced exploding primary cfg with singleton domains, `MAX_SEQ=3`, `MAX_EVENTS=3`; primary TLC completed.
- `TLC-R2-002`: preserved old large cfg as `specs/tla/RecoveryReplayFull-large-stress.cfg`; stress run not executed in this pass.
- `TLC-R2-003`: added `digest_stage`; `CheckIrDigest` now requires `WorkflowChecked`; `DigestVerificationOrder` now asserts `IrChecked => WorkflowChecked`.
- `TLC-R2-004`: added causal error transitions for all current non-`None` TLA `ErrorDomain` variants.
- `TLC-R2-005`..`TLC-R2-010`: split non-vacuity into independent witness cfgs.
- `TLC-R2-011`..`TLC-R2-019`: added independent per-error reachability witness cfgs.
- `TLC-R2-020`: synced `specs/tla/RecoveryReplayFull*` model/cfg artifacts into `evidence/specs/`; required `cmp -s` check passed.

## Model repairs

- `specs/tla/RecoveryReplayFull.tla`
  - Added `digest_stage` variable, TypeOK membership, Init value, and `vars` tuple.
  - Updated unchanged-tuples for all existing transitions.
  - Repaired workflow/IR digest actions:
    - workflow match records `"WorkflowChecked"`;
    - workflow mismatch records `WorkflowSourceDigestMismatch`;
    - IR checks require `"WorkflowChecked" \in digest_stage[run]`;
    - IR match records `"IrChecked"`;
    - IR mismatch records `CompiledIrDigestMismatch` only after workflow check.
  - Replaced old digest invariant with explicit order invariant over `digest_stage`.
  - Added causal error actions: `RecoverRunWithoutEvents`, `LoadCorruptSnapshot`, `CheckActionAbiDigest`, `CheckPolicyDigest`, `DetectNonIdempotentResolved`, `DetectReplayDivergence`, and `DetectFrameDimensionOverflow`.
  - Added independent negated witness invariants for replay order, snapshot tail, incomplete runs, terminal exclusion, no re-execution guard, digest order, and every modeled error variant.

## Config repairs

- `specs/tla/RecoveryReplayFull.cfg`: feasible bounded primary cfg, singleton domains, all event types, `MAX_SEQ=3`, `MAX_EVENTS=3`.
- `specs/tla/RecoveryReplayFull-large-stress.cfg`: old historical large bounds preserved as optional/stress only.
- Added independent witness cfgs under `specs/tla/RecoveryReplayFull-nonvacuity-*.cfg` and `specs/tla/RecoveryReplayFull-error-*.cfg`.

## Evidence summary

- Primary bounded TLC: `evidence/specs/RecoveryReplayFull.tlc.log`
  - `Model checking completed. No error has been found.`
  - `36532738 states generated, 4088199 distinct states found, 0 states left on queue.`
  - Complete search depth: `11`.
- Witness TLC logs: all required witness commands produced the expected negated-invariant violation; these non-zero TLC exits are classified as expected witness success, not invariant-proof success.
- Sync check: required `cmp -s` command exited successfully; log path `evidence/specs/RecoveryReplayFull-sync.cmp.log` is empty because `cmp -s` is silent on success.

## Pending / not claimed

- Optional stress cfg `specs/tla/RecoveryReplayFull-large-stress.cfg` was not run. It remains `PENDING_OPTIONAL_STRESS`; no approval depends on it.
- No liveness/fairness proof is claimed.
- No unbounded domain/cardinality proof is claimed beyond the stated finite cfg bounds.
- No cryptographic digest payload proof is claimed; digest values are abstract finite model values.
- `ActionAbiMismatch` and `PolicyDigestMismatch` remain abstract typed error-domain witnesses, not proof that deferred runtime GAP-3 lookup behavior is implemented.
