#![forbid(unsafe_code)]
//! Fuzz target for YAML event parsing.
//!
//! This target verifies that YAML profile validation and event parsing never panic
//! on any UTF-8 input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/yaml_events/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_yaml_events(data);
});
