//! Fuzz target: `vb_compile::compile_workflow` (FT-001)
//!
//! Feeds arbitrary byte sequences into the YAML compiler entry point.
//! Must never panic — any input is valid to attempt compilation.
//!
//! ## Obligations Covered
//!
//! - FT-001: Parse+compile boundary via `compile_workflow(source: &[u8])`
//! - Covers YAML parser panic, excessive memory, stack overflow,
//!   integer overflow in YAML parsing, and digest computation panic.
//!
//! ## Non-Goals
//!
//! - Does not verify compile success/failure — only panic-freedom.
//! - Does not check output correctness — that's covered by unit/proptest.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Any byte sequence must be handled without panic.
    // compile_workflow may return Err for invalid inputs; that's expected.
    let _ = vb_compile::compile_workflow(data);
});
