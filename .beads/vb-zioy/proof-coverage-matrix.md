# Proof Coverage Matrix: vb-zioy

## Requirements × Verification Layers

| Requirement | Contract Clause | Unit Test | Integration Test | proptest | Compile Check | Code Review |
|------------|-----------------|-----------|------------------|----------|---------------|-------------|
| REQ-001 | StepFieldShape uses diagnostic_step | Planned (PO-001) | — | Required (PO-001) | Automatic | Required |
| REQ-002 | UnsupportedStepPrimitive uses diagnostic_step | Planned (PO-002) | — | Required (PO-002) | Automatic | Required |
| REQ-003 | Signature accepts diagnostic_step | — | — | — | Required | Required |
| REQ-004 | collect passes original index | — | Planned (PO-003) | Required (PO-003) | Automatic | Required |
| REQ-005 | for_each passes original index | — | Planned (PO-004) | Required (PO-004) | Automatic | Required |
| REQ-006 | aggregate passes original index | — | Planned (PO-004) | Required (PO-004) | Automatic | Required |
| REQ-007 | repeat passes original index | — | Planned (PO-004) | Required (PO-004) | Automatic | Required |
| REQ-008 | parallel passes appropriate index | — | Planned (PO-004) | Required (PO-004) | Automatic | Required |
| REQ-009 | No synthetic step in diagnostics | — | Planned (PO-004) | Required (PO-004) | Automatic | Required |
| REQ-010 | Error text unchanged | — | Planned (PO-004) | — | Automatic | Required |

## Coverage Notes

- **REQ-003** is fully covered by the Rust type system (compile-time) plus human code review. No runtime or formal verification is applicable because it is a static API contract.
- **REQ-009** (no synthetic step in diagnostics) is implicitly covered by PO-004: if every scoped primitive caller passes the source index and `emit_single_body_set` uses it exclusively, then synthetic indices cannot leak into diagnostics.
- **REQ-010** (error text unchanged) is verified by existing and updated integration tests that assert `field` and `expected` strings remain identical.
