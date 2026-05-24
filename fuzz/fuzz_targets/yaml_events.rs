//! Fuzz target for YAML event parsing.
//!
//! This target verifies that `validate_yaml_profile`, `parse_yaml_events`, and
//! `build_source_map` never panic on arbitrary UTF-8 input and handle hostile
//! YAML through typed error returns.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_yaml_events(data);
});
