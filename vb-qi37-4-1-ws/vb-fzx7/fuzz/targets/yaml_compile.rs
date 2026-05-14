//! YAML compile fuzz target
//!
//! Fuzz target for `compile_workflow` from vb_compile crate.
//! This target ensures the workflow compiler does not panic on any input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::compile_workflow;

fuzz_target!(|data: &[u8]| {
    // Compile must not panic - errors are Ok, panics are bugs
    let _ = compile_workflow(data);
});
