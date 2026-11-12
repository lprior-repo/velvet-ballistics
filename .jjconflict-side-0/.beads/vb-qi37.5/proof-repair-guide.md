# Proof Repair Guide: vb-qi37.5 State 6 Attempt 3

## Required Repairs

1. Repair `KANI-PARITY-006`.
   Remove `kani::assume(!excluded)` from `crates/vb_compile/src/kani_idempotency_parity.rs`, enumerate all 45 `SideEffect x RetrySafety x Idempotency` combinations, and assert Ok/Err parity plus contracted reason-class parity where the contract requires it. Rerun `cargo kani -p vb_compile` and include raw output evidence.

2. Justify or replace `VERUS-PARITY-002`.
   The standalone Verus model may remain useful, but it must be tied to faithful production compile-gate semantics. Either add an approved extraction/refinement argument backed by executable parity evidence, or defer Verus parity until production compile logic/harness is repaired.

3. Resolve `FUZZ-ARTIFACT-011`.
   Make `cargo fuzz run admission_fuzz -- -runs=1000` executable in the isolated workspace, or add a valid waiver with owner, expiry, blocker classification, and compensating evidence. Discovery alone is insufficient.

## Rerun Targets

- `pwd -P`
- JSONL gates for `.beads/vb-qi37.5/proof-obligations.jsonl`, `.beads/vb-qi37.5/proof-obligations.planned.jsonl`, and `.beads/vb-qi37.5/traceability-matrix.jsonl`
- `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`
- `verus verification/verus/idempotency_decision.rs`
- `verus verification/verus/idempotency_certificate_summary.rs`
- `verus verification/verus/idempotency_replay_tracker.rs`
- `cargo kani -p vb_compile`
- `cargo fuzz run admission_fuzz -- -runs=1000` or approved waiver evidence

Next proof review may approve only if every required proof obligation is mapped to non-vacuous artifacts and either executed successfully or explicitly waived with valid governance.
