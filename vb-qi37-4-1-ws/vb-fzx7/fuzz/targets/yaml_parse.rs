//! YAML parse fuzz target
//!
//! Fuzz target for `parse_yaml_events` from vb_yaml crate.
//! This target ensures the YAML parser does not panic on any input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_yaml::parse_yaml_events;

fuzz_target!(|data: &[u8]| {
    // Parse must not panic - errors are Ok, panics are bugs
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_yaml_events(text);
    }
});
