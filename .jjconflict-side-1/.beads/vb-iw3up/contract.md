# Rust contract: handcrafted CLI IR input metadata

Existing WorkflowParts fixture base_parts must include input_slots. Actual type CompiledInputSlot has exactly pub slot:SlotIdx and pub kind:InputSlotKind. Declare a single slot0/I64 input, matching slot_count1 and the existing valid test payload I64(42) consumed by Finish(slot0). No names, required flags, or defaults exist in this IR metadata. All malformed artifact source tests and exact diagnostics remainunchanged; no production change. Valid input shape and IR validation boundaries preserved.

Command: bounded cargo test -p velvet-ballistics --test ir_artifact_admission; then fullCLIpackage, with raw logs and failures reported exactly. Independent code and test reviews required. No proof planning.

Local initial draft invented metadata fields and a tests/cli path; rejectedthose claims beforeimplementation. Actual path crates/velvet_ballistics/tests/ir_artifact_admission.rs and actual struct crates/vb_core/src/workflow/mod.rs:312-317 verified.
