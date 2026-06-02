// Verification artifact: reduce_lowering_panic.rs
// PO: PO-NOPANIC-FUZZ-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: cargo-fuzz
// Command: cargo fuzz run reduce_lowering_panic -- -max_total_time=300
//
// Requirement: C11 — No Panic
// Domain Claim: Hostile/malformed YAML documents fed to compile_source()
//   never cause panics.
//
// Fuzz target feeds arbitrary bytes through the YAML parser into the
// compile pipeline and verifies no panic occurs.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_yaml::parse_workflow_source;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse arbitrary bytes as YAML
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse YAML into WorkflowSource
        if let Ok(source) = parse_workflow_source(text) {
            // Feed through compile_source — must not panic
            let _ = vb_compile::compile_source(&source);
        }
    }
});
