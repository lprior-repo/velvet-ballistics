# Proof Repair Guide: vb-scxh

## Status

- Current State 6 proof decision: `STATUS: APPROVED`.
- Proof findings count: `0`.
- No State 5 proof repair is required from this review.

## Downstream Route

- owner_state: 11 for raw evidence packaging and audits.
- owner_state: 12 for Truth Serum and final evidence decision.
- landing_status: blocked until State 11/12 required evidence passes or emits explicit blocking failure packets.

## Required Downstream Evidence

- `BD-SCXH-001` and `BD-SCXH-002`: capture raw BD output, exact 12 false-closure IDs, per-ID reopened/linked/follow-up status, and raw source markers.
- `SAFETY-SCXH-001` and `ERR-SCXH-006`: rerun bundle/bookmark verification and keep `BLOCK_LOCAL` if the bundle cannot be opened or the rescue ref is missing.
- `CI-SCXH-001`: audit or rerun green CI with raw command/status/task/test markers.
- `MUT-SCXH-001` and `ERR-SCXH-007`: preserve `FAIL_UNVIABLE / DEFERRED` as non-pass; do not relabel it as mutation adequacy.
- `SCOPE-SCXH-001` and `ERR-SCXH-008`: keep generated parity gaps deferred to `vb-gvmt` / `vb-qi37.10`; do not use representative generated parity as exhaustive closure proof.
- `TRUTH-SCXH-001` and `ERR-SCXH-003` through `ERR-SCXH-009`: final decision may approve close/unblock only after every required raw-evidence lane passes or has an approved waiver.

## Guardrails

- Do not treat the TLA PASS as closure or unblock authorization.
- Do not treat the safety-anchor failure as waived.
- Do not replace raw command evidence with subagent narrative.
- Do not proceed to landing while any downstream raw-evidence blocker remains open.
