//! Fuzz target for compiled IR decode.
//!
//! This target verifies that `CompiledWorkflow::try_from_parts` never panics on any
//! postcard-encoded `WorkflowParts` input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/compiled_ir/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_compiled_ir(data);
});
