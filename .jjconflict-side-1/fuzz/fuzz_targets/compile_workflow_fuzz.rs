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
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with typed-result assertions:
//! - Ok path: the compiled workflow must have `node_count() > 0` (the
//!   canonical compiler rejects `EmptySteps` before returning Ok, so a
//!   zero-node Ok would indicate a bypass of `validate_workflow_document_shape`).
//! - Err path: `CompileErrors::is_empty()` must be `false` (errors are
//!   collected in a `Vec`, but every `CompileErrors` returned by the
//!   facade is populated — an empty Vec would mean the compiler silently
//!   rejected input without explaining why).
//!
//! ## Non-Goals
//!
//! - Does not verify compile success/failure — only panic-freedom plus
//!   the structural invariants above.
//! - Does not check output correctness — that's covered by unit/proptest.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Any byte sequence must be handled without panic and produce a
    // structurally well-formed result on both Ok and Err paths.
    let result = vb_compile::compile_workflow(data);
    match result {
        Ok(workflow) => {
            // Ok path: a successfully compiled workflow must have
            // compiled at least one node. The compiler enforces
            // EmptySteps rejection before this point; an empty node set
            // would mean validation was bypassed.
            assert!(
                workflow.node_count() > 0,
                "compile_workflow Ok returned 0 nodes (validator bypassed)"
            );
        }
        Err(errors) => {
            // Err path: every Err must carry at least one diagnostic.
            // An empty errors vec would mean the compiler returned
            // Err(CompileErrors(vec![])) without telling the caller why.
            assert!(
                !errors.is_empty(),
                "compile_workflow Err with empty errors vec"
            );
        }
    }
});
