# Test Plan: vb-qi37.4

STATUS: APPROVED

## Summary

- Behaviors identified: 8.
- Scope: proof-wrapper rerun, proof artifacts, Loom compile repair, and existing admission evidence suite.
- Trophy allocation: 3 proof/model, 3 integration, 1 fuzz/deep, 1 static/CI.

## Behavior Inventory

- Admission rejects invalid accepted artifacts before live run allocation.
- Admission preserves digest binding across accepted artifact, header, and admission metadata.
- Duplicate run ids reject with `RunAlreadyExists` before new state allocation.
- Capacity exhaustion rejects with `ActiveRunCapacityExceeded` before live state allocation.
- Header persistence failure prevents success acknowledgement.
- Capability mismatch rejects with typed admission error.
- Canonical proof wrapper runs configured proof rollup without shell parse failure.
- Loom admission/journal queue models compile and execute under `cfg(loom)`.

## BDD Scenarios

- Given existing admission integration tests, when `cargo test -p velvet_ballastics --test admission_evidence_integration` runs, then all admission evidence scenarios pass with exact typed outcomes.
- Given accepted artifact storage tests, when `cargo test -p vb_storage --test accepted_artifact_red_phase` runs, then accepted artifact envelope/error behavior passes.
- Given durability diagnostic tests, when `cargo test -p velvet_ballastics --test admission_durability_code` runs, then admission durability code mapping passes.
- Given the repaired proof wrapper, when `moon run :verify-proof` runs, then it exits 0 and reports all proof checks passed.
- Given the repaired Loom model imports, when `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue` runs, then journal queue models pass.

## Verification Scenarios

- TLA+: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`.
- Verus admission: `verus verification/verus/admission_artifact_model.rs`.
- Verus capability: `verus verification/verus/capability_artifact_model.rs`.
- Deep/fuzz/mutation/CI: `moon run :verify-deep`, `moon run :fuzz-smoke`, `moon run :mutants-smoke`, and `jj diff --name-only | moon ci --stdin`.

## Mutation Checkpoint

- `moon run :mutants-smoke` must catch the scoped mutant; observed 1 mutant tested, 1 caught.

## Open Questions

- None blocking for State 13. Broader product admission-shell guarantees remain represented by existing integration tests and CI evidence.
