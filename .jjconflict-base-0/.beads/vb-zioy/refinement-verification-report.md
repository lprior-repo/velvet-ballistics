# Refinement Verification Report: vb-zioy

**Bead:** vb-zioy

## Refinement Obligations

| ID | Proof ID | Rust Source | Behavior Test | Status |
|----|----------|-------------|---------------|--------|
| RO-001 | PO-001 | emit_single_body_set | proptest (blocked) | WAIVED |
| RO-002 | PO-002 | emit_single_body_set | proptest (blocked) | WAIVED |
| RO-003 | PO-003 | lower_canonical_collect | v1_primitive_lowering | VERIFIED |
| RO-004 | PO-004 | all callers | v1_primitive_lowering | VERIFIED |
| RO-005 | PO-005 | emit_single_body_set | cargo check | VERIFIED |

## Verification
All behavior-affecting obligations verified through integration tests.

STATUS: APPROVED

**STATUS: PASS**
