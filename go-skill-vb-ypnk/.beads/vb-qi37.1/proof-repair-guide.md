# Proof Repair Guide: vb-qi37.1

## Required Routing

- Route back before State 6 approval. The immediate owner is State 5 for proof artifacts, but `PO-017`, `PO-021`, and `PO-022` may require an upstream contract/production-design decision before State 5 can honestly pass.
- Do not mark `PO-017`, `PO-021`, or `PO-022` as passed until action ABI and policy digest behavior is either implemented and proof-linked, or the contract/obligation plan explicitly removes or waives those full-mode families with owner, reason, expiry/trigger, and compensating evidence.

## Fixes Required

1. For `PO-017`, `PO-021`, and `PO-022`, repair the digest proof gap.
   Required result: `DigestCheck::Full` obligations for workflow source, compiled IR, action ABI, and policy digest mismatch are either production-linked and verified, or contractually rescoped before proof review.
   Minimum evidence: raw command output from the relevant verifier plus artifact mapping from proof function/spec to the real `verify_digests` behavior or a valid scope revision in `contract.md`, `traceability-matrix.jsonl`, and `proof-obligations.planned.jsonl`.

2. Replace `proof_typed_recovery_errors_are_decision_outputs` with a non-vacuous proof for `PO-016`.
   Required result: the proof models typed error variants and proves fallible recovery/storage/runtime decisions preserve typed diagnostics rather than silently succeeding or discarding errors.
   Minimum evidence: `verus verification/verus/recovery_verification.rs` exits `0` and the proof obligation text in `proof-evidence.md` names the proof functions that discharge `VERUS-INV-005`.

3. Keep the TLA+ repair, but preserve raw rerun evidence.
   Required result: TLC still exits `0`, deadlock checking remains enabled by omission of `CHECK_DEADLOCK FALSE`, and `RecoveryHydration.cfg` continues to check the listed invariants and liveness property.

## Rerun Targets

```bash
pwd -P
test -s .beads/vb-qi37.1/proof-writer-report.md
jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null
tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla
verus verification/verus/recovery_verification.rs
```

## Acceptance Bar For Next State 6 Attempt

- `proof-writer-report.md` must not contain `BLOCKED_PRODUCTION_DESIGN` for required owner_state 5 obligations unless those obligations are no longer required by a valid contract/plan repair.
- `proof-evidence.md` must map every required State 5 obligation to raw command evidence, a non-vacuous proof/model property, and the concrete artifact path.
- `proof-review.md` may be approved only after `PO-017`, `PO-021`, and `PO-022` are no longer missing production linkage and `PO-016` is no longer tautological.
