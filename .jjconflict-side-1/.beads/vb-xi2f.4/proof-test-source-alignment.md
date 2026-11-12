# Proof-Test-Source Alignment: vb-xi2f.4

| Proof ID | Test Ref | Source Ref | Status |
|---|---|---|---|
| PO-001 | vb_xi2f_compile_source_proptest | part_01.rs:57 | aligned |
| PO-002 | kani harness | part_01.rs:57 | aligned |
| PO-003 | compile_source_never_panics | mod_compile_core.rs:35 | aligned |
| PO-007 | error_variant_proptest | workflow/mod.rs | aligned |


| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| REQ-001 | PO-001 | rro-001 | part_01.rs::compile_source | vb_xi2f_compile_source_proptest.rs | verification/kani/vb_xi2f_compile_source.rs | cargo test -p vb_compile | PASS | aligned |
| REQ-002 | PO-007 | rro-001 | workflow/mod.rs::try_from_parts | vb_xi2f_error_variant_proptest.rs | | cargo test -p vb_compile | PASS | aligned |
