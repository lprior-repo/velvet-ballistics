// Fuzz target: fuzz_digest_compile
// Bead: vb-xi2f.34 — Finish Digest Semantics
// Proof obligation: PO-FUZZ-FINISH-001 — YAML parser → compile → digest boundary
//
// Feed arbitrary bytes through the full parse → compile → digest pipeline.
// Neither parsing, compiling, nor digest computation may panic on any input.
//
// GOD RULE 4: No loop oscillations. Pure fuzz harness.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::compile_source;
use vb_yaml::parse_workflow_source;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse arbitrary bytes as YAML → WorkflowSource.
    // Parsing failures (invalid YAML, invalid schema, etc.) are expected
    // and must not panic.
    if let Ok(source) = parse_workflow_source_bytes(data) {
        // If parsing succeeded, attempt to compile the workflow.
        // Compilation failures (validation errors, unknown output names, etc.)
        // are expected and must not panic.
        // The digest is computed by canonical_digest() at the start of
        // compile_source() (part_01.rs:46) — before the lowering loop.
        // This exercises the full digest computation path.
        let result = compile_source(&source);
        match result {
            Ok(workflow) => {
                // Compiled workflow must have at least one node
                assert!(
                    workflow.node_count() >= 1,
                    "compiled workflow must have at least 1 node"
                );
            }
            Err(errors) => {
                // Compilation errors are expected for some inputs.
                assert!(
                    !errors.is_empty(),
                    "compile errors must contain at least one error"
                );
            }
        }
    }
});

/// Try to parse bytes as UTF-8 then parse YAML.
/// Non-UTF-8 bytes are benign and simply fail parsing.
fn parse_workflow_source_bytes(data: &[u8]) -> Result<vb_yaml::ast::WorkflowSource, ()> {
    let text = std::str::from_utf8(data).map_err(|_| ())?;
    parse_workflow_source(text).map_err(|_| ())
}
