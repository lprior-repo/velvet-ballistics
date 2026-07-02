# Test Plan: vb-xi2f.4

## Scope
Compiler emission validation routing.

## Unit Tests
- empty_nodes_returns_error
- entry_out_of_bounds_returns_error
- step_out_of_bounds_returns_error
- slot_out_of_bounds_returns_error
- unreachable_node_returns_error
- backward_edge_returns_error

## Proptest
- compile_source_never_panics
- yaml_compiler_compile_never_panics
- arbitrary_invalid_entry_returns_typed_error
- arbitrary_invalid_slot_returns_slot_error

## Source Scan
- no_unchecked_construction_in_public_compile_apis

## Proof Coverage
- PO-003: compile_source validated output
- PO-007: error variant coverage


## Proof/Refinement Coverage Matrix

| Proof ID | Test | Coverage |
|---|---|---|
| PO-003 | compile_source_never_panics | full |
| PO-007 | error_variant_proptest | full |
