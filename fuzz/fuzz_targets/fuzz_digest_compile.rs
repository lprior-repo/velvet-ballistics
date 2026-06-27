// Fuzz target: fuzz_digest_compile
// Bead: vb-xi2f.34 — Finish Digest Semantics
// Proof obligation: PO-FUZZ-FINISH-001 — YAML parser → compile → digest boundary
//
// Feed arbitrary bytes through the full parse → compile → digest pipeline.
// Neither parsing, compiling, nor digest computation may panic on any input.
//
// INVARIANT Oracle (replaces crash-only):
// - parse Ok  ⇒  source.steps() is non-empty (validator requires ≥1 step).
// - compile Ok ⇒ workflow.node_count() > 0 (validator requires ≥1 node).
// - compile Err ⇒ errors.is_empty() == false (no silent rejections).
//
// GOD RULE 4: No loop oscillations. Pure fuzz harness.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::compile_source;
use vb_compile::parse_workflow_source;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse arbitrary bytes as YAML → WorkflowSource.
    // Parsing failures (invalid YAML, invalid schema, etc.) are expected
    // and must not panic.
    let parsed = match parse_workflow_source_bytes(data) {
        Ok(source) => {
            // INVARIANT: every successful parse yields ≥1 step. The
            // EmptySteps validator runs before Ok is returned, so a
            // zero-step Ok means validation was bypassed.
            assert!(
                !source.steps().is_empty(),
                "parse_workflow_source Ok returned 0 steps"
            );
            source
        }
        // Err path: the YAML parser returns a structured YamlError.
        // Documented by the match arm — no panic, no empty failure.
        Err(()) => return,
    };

    // The digest is computed by canonical_digest() at the start of
    // compile_source() (part_01.rs:46) — before the lowering loop.
    // This exercises the full digest computation path.
    match compile_source(&parsed) {
        Ok(workflow) => {
            // INVARIANT: a successful compile must emit ≥1 compiled node.
            assert!(
                workflow.node_count() > 0,
                "compile_source Ok returned 0 nodes"
            );
        }
        Err(errors) => {
            // INVARIANT: every Err must carry at least one diagnostic.
            assert!(
                !errors.is_empty(),
                "compile_source Err with empty errors vec"
            );
        }
    }
});

/// Try to parse bytes as UTF-8 then parse YAML.
/// Non-UTF-8 bytes are benign and simply fail parsing.
fn parse_workflow_source_bytes(data: &[u8]) -> Result<vb_compile::WorkflowSource, ()> {
    let text = std::str::from_utf8(data).map_err(|_| ())?;
    parse_workflow_source(text).map_err(|_| ())
}
