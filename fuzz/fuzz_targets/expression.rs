#![forbid(unsafe_code)]
//! Fuzz target for expression lex/parse/compile/eval roundtrip.
//!
//! This target verifies that expression evaluation never panics on any UTF-8 input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/expression/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_expression(data);
});
