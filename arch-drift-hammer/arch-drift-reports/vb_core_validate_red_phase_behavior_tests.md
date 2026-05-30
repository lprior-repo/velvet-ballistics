# Architectural Drift Report: `red_phase_behavior_tests.rs`

## File Analysis

| Metric | Value |
|--------|-------|
| **File** | `crates/vb_core/src/engine/validate/tests/red_phase_behavior_tests.rs` |
| **Total Lines** | 1645 |
| **Threshold** | 300 |
| **Drift** | **EXCEEDS THRESHOLD by 1345 lines** |
| **Unit Tests** | 61 `#[test]` |
| **Formal Proofs** | 8 `#[kani::proof]` |
| **Total Test Items** | 69 |

## Module Breakdown

| Module | Test Count | Lines (est.) |
|--------|-----------|--------------|
| `valid_workflow_graph_acceptance` | 4 | ~100 |
| `cycle_detection_and_backward_edges` | 5 | ~140 |
| `disconnected_unreachable_nodes` | 4 | ~60 |
| `duplicate_node_ids_detection` | 4 | ~100 |
| `per_node_kind_invalid_configurations` | 14 | ~400 |
| `resource_contract_exceeded` | 5 | ~100 |
| `target_out_of_bounds` | 6 | ~140 |
| `taint_secret_validation` | 2 | ~55 |
| `error_message_exactness_verification` | 6 | ~120 |
| `complex_nested_workflow_validation` | 4 | ~200 |
| `determinism_idempotency_of_validation` | 5 | ~90 |
| `kani_harnesses` | 8 | ~80 |

## Location Category

**Validation Phase Test Suite** — `engine/validate/tests/`

This file tests the red-phase validation behavior of the workflow engine,
specifically:
- Graph structure validation (reachability, cycles, node ID consistency)
- Bounds checking (slots, constants, expressions, step indices)
- Resource contract enforcement
- Error exactness and determinism
- Formal Kani proofs for panic-freedom and determinism

## DDD Cohesion Assessment

| Concern | Status |
|---------|--------|
| Primitive Obsession | ✅ Uses `StepIdx`, `SlotIdx`, `ConstIdx`, `ExprIdx` newtypes |
| Workflow State Transitions | ✅ Explicit error types via `WorkflowError` enum |
| Parse, Don't Validate | ✅ Validation functions return `Result<(), WorkflowError>` |
| Test Organization | ⚠️ All in single file — violates 300-line limit |

## Findings

### CRITICAL: File Size Violation
- **1645 lines** vs **300 line maximum**
- Ratio: **5.5x over threshold**
- This file MUST be split to comply with architectural constraints

### Test Module Cohesion
The 12 modules are logically separated but physically co-located.
Each module has clear semantic boundaries matching validation concerns.

### Recommendations

#### Option A: Split by Validation Category (Recommended)
Split into 4-5 files:

```
engine/validate/tests/
├── red_phase_graph_tests.rs      # valid_workflow_graph_acceptance,
│                                 # cycle_detection_and_backward_edges,
│                                 # disconnected_unreachable_nodes,
│                                 # duplicate_node_ids_detection,
│                                 # complex_nested_workflow_validation
├── red_phase_bounds_tests.rs     # target_out_of_bounds,
│                                 # per_node_kind_invalid_configurations
├── red_phase_contract_tests.rs  # resource_contract_exceeded,
│                                 # taint_secret_validation
├── red_phase_error_tests.rs     # error_message_exactness_verification,
│                                 # determinism_idempotency_of_validation
└── red_phase_kani_proofs.rs     # kani_harnesses (isolated behind cfg(kani))
```

#### Option B: Split by Test Count (Simpler)
Split into 3 files:
1. `red_phase_behavior_tests_part1.rs` — modules 1-4 (~600 lines)
2. `red_phase_behavior_tests_part2.rs` — modules 5-8 (~600 lines)
3. `red_phase_behavior_tests_part3.rs` — modules 9-12 (~400 lines + kani)

#### Update Required
After splitting:
1. Create new files with appropriate module declarations
2. Update `engine/validate/tests/mod.rs` (or create one) to expose new modules
3. Remove `red_phase_behavior_tests.rs` or convert to a re-export aggregator

## Status

```
STATUS: DRIFT_DETECTED
ACTION_REQUIRED: Split file into multiple test modules
SEVERITY: HIGH
```

## Evidence

```bash
# Line count evidence
$ wc -l red_phase_behavior_tests.rs
   1645 red_phase_behavior_tests.rs

# Test count evidence
$ grep -c '#\[test\]' red_phase_behavior_tests.rs
61
$ grep -c '#\[kani::proof\]' red_phase_behavior_tests.rs
8
```
