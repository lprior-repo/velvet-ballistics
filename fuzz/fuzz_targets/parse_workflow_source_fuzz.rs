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
//! ## INVARIANT Oracle (replaces crash-only)
//!
//! - Ok path: the parsed `WorkflowSource` must have a non-empty `steps()`
//!   slice. The `validate_workflow_document_shape` gate runs before Ok
//!   is returned and rejects the empty workflow with `EmptySteps`. A
//!   zero-step Ok would mean validation was bypassed.
//! - Err path: a structured `YamlError` is returned. Documented by the
//!   match arm — no panic, no discarded failure.
//!
//! ## Non-Goals
//!
//! - Does not verify parse success/failure — only panic-freedom plus the
//!   structural invariant above.
//! - Invalid UTF-8 is silently skipped (fuzz input is random bytes).
//! - Does not check AST correctness — that's covered by unit proptest.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only attempt to parse valid UTF-8; random bytes that aren't
    // UTF-8 are silently dropped (the parser operates on &str).
    let Ok(text) = std::str::from_utf8(data) else { return; };
    match vb_compile::parse_workflow_source(text) {
        Ok(source) => {
            // INVARIANT: a successful parse must yield ≥1 step.
            assert!(
                !source.steps().is_empty(),
                "parse_workflow_source Ok returned 0 steps (validator bypassed)"
            );
        }
        // INVARIANT: parse failure returns a structured YamlError,
        // not a panic or an empty Option.
        Err(_yaml_error) => {}
    }
});
