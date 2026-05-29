# Proof Coverage Matrix — vb-7m21 (Replan, Reduced Scope)

## Coverage Summary

- **9 proof seeds** × **8 verifiers** = 72 lane decisions
- **14 required proof obligations** (down from 39 in original over-scoped plan)
- **58 not-applicable lane decisions** (all with concrete evidence)
- **0 blocked tooling**
- **0 behavior waivers**

## Required Obligations by Seed

| Requirement | Proof Seed | Kani | Proptest | Cargo-Fuzz | Obligations |
|---|---|---|---|---|---|
| REQ-5 | PS-vb-7m21-001 | PO-vb-7m21-kani-001 | PO-vb-7m21-prop-001 | PO-vb-7m21-fuzz-001 | 3 |
| REQ-3 | PS-vb-7m21-002 | PO-vb-7m21-kani-002 | PO-vb-7m21-prop-002 | PO-vb-7m21-fuzz-002 | 3 |
| REQ-6 | PS-vb-7m21-003 | PO-vb-7m21-kani-003 | PO-vb-7m21-prop-003 | PO-vb-7m21-fuzz-003 | 3 |
| REQ-4 | PS-vb-7m21-004 | — | PO-vb-7m21-prop-004 | — | 1 |
| REQ-8 | PS-vb-7m21-005 | — | PO-vb-7m21-prop-005 | — | 1 |
| REQ-9 | PS-vb-7m21-006 | — | PO-vb-7m21-prop-006 | — | 1 |
| REQ-10 | PS-vb-7m21-007 | — | PO-vb-7m21-prop-007 | — | 1 |
| REQ-11 | PS-vb-7m21-008 | — | PO-vb-7m21-prop-008 | — | 1 |
| REQ-16 | PS-vb-7m21-009 | — | — | — | 0 (review only) |

## Lane Profile Matrix

| Seed | TLA+ | Verus | Kani | Flux | Loom | Miri | Proptest | Fuzz |
|------|------|-------|------|------|------|------|----------|------|
| PS-001 / REQ-5 | NA | NA | REQ | NA | NA | NA | REQ | REQ |
| PS-002 / REQ-3 | NA | NA | REQ | NA | NA | NA | REQ | REQ |
| PS-003 / REQ-6 | NA | NA | REQ | NA | NA | NA | REQ | REQ |
| PS-004 / REQ-4 | NA | NA | NA | NA | NA | NA | REQ | NA |
| PS-005 / REQ-8 | NA | NA | NA | NA | NA | NA | REQ | NA |
| PS-006 / REQ-9 | NA | NA | NA | NA | NA | NA | REQ | NA |
| PS-007 / REQ-10 | NA | NA | NA | NA | NA | NA | REQ | NA |
| PS-008 / REQ-11 | NA | NA | NA | NA | NA | NA | REQ | NA |
| PS-009 / REQ-16 | NA | NA | NA | NA | NA | NA | NA | NA |

REQ = required. NA = not_applicable with documented evidence.

## Excluded Verifier Justification

- **Verus (all 9 seeds NA)**: Test-first bead, no production implementation in scope until State 11; no exec/spec binding targets exist. Evidence: contract.md:26-27, codebase-map.md:7-8.
- **Flux (all 9 seeds NA)**: Test-first bead, no new behavior-affecting Rust code to annotate. Evidence: contract.md:26-27.
- **TLA+ (all 9 seeds NA)**: No temporal protocol, retry, lease, lifecycle, distributed, or interleaving behavior. Evidence: boundary-map.md:36-39.
- **Kani (seeds PS-004..009 NA)**: Integration seeds operating through higher-level public APIs where bounded model checking adds minimal value over proptest. Evidence: codebase-map.md:71-79.
- **Loom (all 9 seeds NA)**: No implementation concurrency risk. Evidence: boundary-map.md:36-39.
- **Miri (all 9 seeds NA)**: No unsafe/FFI/UB risk. Evidence: boundary-map.md:41-44.
- **Cargo-fuzz (seeds PS-004..009 NA)**: No parser/codec/hostile byte-input surface. Evidence: proof-seeds.jsonl model_boundary fields.
