# Proof-Test-Source Alignment: vb-zioy

## Alignment Matrix

| Proof ID | Test Ref | Source Ref | Status |
|---|---|---|---|
| PO-001 | proptest_body_dispatcher.rs (disabled) | part_04.rs:211 | not_aligned (global) |
| PO-002 | proptest_error_parity.rs (disabled) | part_04.rs:211 | not_aligned (global) |
| PO-003 | v1_primitive_lowering.rs::compile_workflow_rejects_multi_step_body_in_scoped_primitives | part_03.rs:167, part_03.rs:193-200 | aligned |
| PO-004 | v1_primitive_lowering.rs (scoped primitives) | part_02.rs:161, part_03.rs:92, part_04.rs:15, part_04.rs:84 | aligned |
| PO-005 | cargo check + grep | part_04.rs:211, part_02.rs:192, part_03.rs:136, part_03.rs:195, part_04.rs:52, part_04.rs:119 | aligned |

## Requirement Alignment

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| REQ-001 | PO-001 | RO-001 | part_04.rs:211::emit_single_body_set | proptest_body_dispatcher.rs (disabled) | — | cargo test --package vb_compile proptest_body_dispatcher | FAIL_GLOBAL | not_aligned (global) |
| REQ-002 | PO-002 | RO-002 | part_04.rs:211::emit_single_body_set | proptest_error_parity.rs (disabled) | — | cargo test --package vb_compile proptest_error_parity | FAIL_GLOBAL | not_aligned (global) |
| REQ-004 | PO-003 | RO-003 | part_03.rs:167::lower_canonical_collect | v1_primitive_lowering.rs::compile_workflow_rejects_multi_step_body_in_scoped_primitives | — | cargo test --package vb_compile --test v1_primitive_lowering compile_workflow_rejects_multi_step_body_in_scoped_primitives | PASS | aligned |
| REQ-005 | PO-004 | RO-004 | part_02.rs:161::lower_canonical_for_each, part_03.rs:92::emit_together_branches, part_04.rs:15::lower_canonical_aggregate, part_04.rs:84::lower_canonical_repeat | v1_primitive_lowering.rs (scoped primitives) | — | cargo test --package vb_compile --test v1_primitive_lowering | FAIL_GLOBAL | aligned* |
| REQ-003 | PO-005 | RO-005 | part_04.rs:211::emit_single_body_set signature + 5 call sites | cargo check --package vb_compile | — | cargo check --package vb_compile && grep | PASS | aligned |

\* PO-004 bead-scoped tests pass (24 passed covering collect, for_each, aggregate, repeat, together); 7 failures are pre-existing choose primitive issues unrelated to this bead.
