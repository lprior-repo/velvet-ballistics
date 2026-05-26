//! Fuzz target for compile_source (exercises AstMarks::new internally).
//!
//! Since AstMarks is pub(crate), we exercise it through the public compiler
//! API. Verifies that the full YAML→IR compilation pipeline (including
//! AstMarks backfill) never panics on arbitrary input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/compile_source_ast_marks/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_compile_source_ast_marks(data);
});
