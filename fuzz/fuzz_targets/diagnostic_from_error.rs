//! Fuzz target for diagnostic_from_error span propagation.
//!
//! Verifies that diagnostic_from_error never panics on any ValidationError
//! variant and that Diagnostic.span always equals error.span (contract C6.2).
//!
//! Corpus seeds are maintained in `fuzz/corpus/diagnostic_from_error/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_diagnostic_from_error(data);
});
