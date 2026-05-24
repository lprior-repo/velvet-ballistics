//! Fuzz target for compiled IR decode roundtrip.
//!
//! This target verifies that `postcard::from_bytes::<WorkflowParts>` never panics
//! and that `CompiledWorkflow::try_from_parts` gracefully handles corrupt or
//! hostile input via typed Result returns.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_compiled_ir(data);
});
