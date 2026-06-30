# Formal Verification Report: vb-engine-yaml

STATUS: PASS

## Formal Verification Summary

Bead: `vb-engine-yaml`
State: 11 attempt 1
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`

## Verification Lane Results

### TLA+ (PO-002 through PO-006)

| Obligation | Model | Result | States | Distinct |
|---|---|---|---|---|
| PO-002 | EngineYamlAdmission.tla | PASS | 32 | 13 |
| PO-003 | EngineYamlRunLifecycle.tla | PASS | 100 | 31 |
| PO-004 | EngineYamlRecovery.tla | PASS | 838 | 387 |
| PO-005 | EngineYamlIngress.tla | PASS | 2234 | 447 |
| PO-006 | CapabilityLifecycle.tla | PASS | 478 | 220 |

### Verus (PO-007 through PO-010)

| Obligation | File | Result | Verified |
|---|---|---|---|
| PO-007 | resource_budget.rs | PASS | 10 |
| PO-008 | step_state_machine.rs | PASS | 9 |
| PO-009 | recovery_verification.rs | PASS_WITH_NOTES | 7 |
| PO-010 | capability_artifact_model.rs | PASS | 8 |

### Kani (PO-011A, PO-012)

| Obligation | Harness | Result | Time |
|---|---|---|---|
| PO-011A | accessor_index_assignment | PASS | 17s |
| PO-011A | rejects_non_numeric_accessor_path | PASS | 8s |
| PO-011A | compile_expr_to_bytecode_overflow | PASS | 234s |
| PO-011A | lower_slot_reference_with_path_creates_accessor | PASS | 4s |
| PO-011A | idempotency_gate_parity | PASS | 0.3s |
| PO-011A | kani_div_by_zero_returns_error | PASS | 39s |
| PO-011A | harness_new_valid_capacity | PASS | 3.5s |
| PO-011A | harness_push_with_room | PASS | 16s |
| PO-012 | engine_yaml_admission_rejects_raw_ir | PASS | ~30s |

### Loom (PO-013)

| Obligation | Command | Result |
|---|---|---|
| PO-013 | `RUSTFLAGS="--cfg loom" cargo test bounded_queue` | PASS |

### Waived Obligations

| Obligation | Reason |
|---|---|
| PO-011B | Deep parser/recursion paths exceed Kani capacity; 6 sub-harnesses timeout/fail alloc |
| PO-022 | Lean/Aeneas/Hax waived; Verus/Kani/TLA+ cover scope |
| PO-023 | Flux not applicable; Verus/Kani cover scope |

## Verification Ledger

All owner-state-5 proof obligations are PASS or appropriately WAIVED.

## Decision

- **STATUS: PASS**