# Proof Coverage Matrix: vb-xi2f.4

## Requirements → Obligations → Verifiers

| Requirement | Contract Clause | Proof Seed | Obligation | Verifier | Target | Status |
|-------------|-----------------|------------|------------|----------|--------|--------|
| REQ-001 | No unchecked compiled workflow construction is reachable from public canonical compile APIs | seed-001 | PO-001 | verus | compile_source postcondition | planned |
| REQ-001 | No unchecked compiled workflow construction is reachable from public canonical compile APIs | seed-001 | PO-002 | kani | compile_source panic-freedom | planned |
| REQ-001 | No unchecked compiled workflow construction is reachable from public canonical compile APIs | seed-001 | PO-003 | proptest | compile_source validated output | planned |
| REQ-001 | No unchecked compiled workflow construction is reachable from public canonical compile APIs | seed-001 | PO-004 | flux-rs | compile_source return path refinement | planned |
| REQ-002 | Invalid generated parts return typed validation errors | seed-002 | PO-005 | verus | WorkflowError → CompileError::Workflow mapping | planned |
| REQ-002 | Invalid generated parts return typed validation errors | seed-002 | PO-006 | kani | try_from_parts error variant correctness | planned |
| REQ-002 | Invalid generated parts return typed validation errors | seed-002 | PO-007 | proptest | try_from_parts error variant coverage | planned |
| REQ-002 | Invalid generated parts return typed validation errors | seed-002 | PO-008 | flux-rs | try_from_parts return type refinement | planned |

## Coverage Summary

- **Behavior-affecting obligations**: 8 / 8 planned
- **Non-behavior obligations**: 0
- **Waived obligations**: 0
- **Blocked obligations**: 0
- **Verifiers used**: verus, kani, proptest, flux-rs
- **Verifiers excluded (with evidence)**: tla-plus, loom, miri, cargo-fuzz

## Risk Tag Coverage

| Risk Tag | Obligations |
|----------|-------------|
| api-safety | PO-001, PO-002, PO-003, PO-004 |
| validation-bypass | PO-001, PO-002, PO-003, PO-004 |
| error-handling | PO-005, PO-006, PO-007, PO-008 |
| validation | PO-005, PO-006, PO-007, PO-008 |
