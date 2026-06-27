//! Fuzz target for compiled IR decode.
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural assertions on
//! `CompiledWorkflow::try_from_parts`:
//! - The decoded `CompiledWorkflow` has `slot_count == workflow.slot_count()`
//!   (the value persisted in `parts.slot_count` is preserved on the workflow).
//! - The decoded workflow has `node_count >= 1` (zero-node workflows are
//!   rejected by `try_from_parts`).
//! - The workflow digest is preserved across the decode step.
//! - All per-node slot indices in every `CompiledNodeKind` variant are
//!   `< slot_count` (enforced inside `fuzz_lib::fuzz_compiled_ir` via
//!   `check_node_slots`).
//!
//! Corpus seeds are maintained in `fuzz/corpus/compiled_ir/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: slot_count survives the postcard → try_from_parts
    // boundary. If the decode succeeds, the workflow MUST carry the same
    // slot_count that was persisted in the parts.
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let slot_count_in = parts.slot_count;
        if let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) {
            assert_eq!(
                workflow.slot_count(),
                slot_count_in,
                "decoded CompiledWorkflow slot_count must equal parts.slot_count"
            );
        }
    }

    fuzz_lib::fuzz_compiled_ir(data);
});
