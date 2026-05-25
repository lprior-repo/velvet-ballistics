# Proof Coverage Matrix - vb-xi2f.37

## Coverage Analysis

| Proof Seed | Description | Covered By | Gap |
|------------|-------------|------------|-----|
| PS-001 | is_primitive("reduce") returns true | Kani PO-vb-xi2f-001 | None |
| PS-002 | is_primitive rejects non-primitives | Kani PO-vb-xi2f-001 | None |
| PS-003 | parse_step_primitive handles reduce | Unit test PO-vb-xi2f-004 | None |
| PS-004 | canonical_primitive_name(Reduce) == "reduce" | Unit test PO-vb-xi2f-006 | None |
| PS-005 | Full YAML parse with reduce step | Integration test PO-vb-xi2f-012 | None |
| PS-006 | reject_unknown_step_fields accepts reduce step | Unit test PO-vb-xi2f-005 | None |
| PS-007 | Kani: parse_step_primitive no panic on reduce | Kani PO-vb-xi2f-011 | None |
| PS-008 | Kani: is_primitive bounds check | Kani PO-vb-xi2f-001 | None |
| PS-009 | Reduce variant type definition | Verus PO-vb-xi2f-003 + Code inspection PO-vb-xi2f-009 | None |
| PS-010 | All primitive match arms handle reduce | Code inspection PO-vb-xi2f-007 | None |
| PS-011 | No primitive name collisions | Code inspection PO-vb-xi2f-008 | None |
| PS-012 | Random reduce step fuzzing | cargo-fuzz PO-vb-xi2f-010 | WAIVED - corpus needs update |

## Coverage Summary

- **Total Proof Seeds**: 12
- **Covered**: 11
- **Waived**: 1 (PS-012, fuzz corpus update)
- **Gap Count**: 0

## Requirement Traceability

| Requirement | Tests | Proofs | Review |
|-------------|-------|--------|--------|
| CC-001 | PS-001, PS-003, PS-005, PS-007, PS-008 | Kani PO-vb-xi2f-001, PO-vb-xi2f-011 | proof-review |
| CC-002 | PS-006 | Kani PO-vb-xi2f-002, Unit test PO-vb-xi2f-005 | proof-review |
| CC-003 | PS-009 | Verus PO-vb-xi2f-003, Code inspection PO-vb-xi2f-009 | proof-review |
| CC-004 | PS-004 | Unit test PO-vb-xi2f-006 | proof-review |

## Confidence Assessment
- **Parsing correctness**: High (Kani + unit tests)
- **Type safety**: High (Verus + code inspection)
- **Integration**: High (integration test PO-vb-xi2f-012)
- **Fuzzing**: Low (waived, corpus needs update)
