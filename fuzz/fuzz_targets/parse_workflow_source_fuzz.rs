//! Fuzz target: `vb_yaml::parse_workflow_source` (FT-002)
//!
//! Feeds arbitrary byte sequences (interpreted as UTF-8) into the YAML
//! AST parser. Must never panic — any valid UTF-8 string is valid input.
//!
//! ## Obligations Covered
//!
//! - FT-002: YAML parse boundary via `parse_workflow_source(yaml: &str)`
//! - Covers Saphyr parser panic, StepPrimitive deserialization bugs,
//!   ScalarValue parsing edge cases, TogetherBranch deserialization
//!   from malformed YAML, and pattern-rejection in validator.
//!
//! ## Non-Goals
//!
//! - Does not verify parse success/failure — only panic-freedom.
//! - Invalid UTF-8 is silently skipped (fuzz input is random bytes).
//! - Does not check AST correctness — that's covered by unit proptest.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only attempt to parse valid UTF-8; random bytes that aren't
    // UTF-8 are silently dropped (the parser operates on &str).
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = vb_yaml::parse_workflow_source(s);
    }
});
