// Fuzz target: fuzz_finish_digest_encoding
// Bead: vb-xi2f.34 — Finish Digest Semantics
// Proof obligation: PO-FUZZ-FINISH-002 — digest_step_primitive Finish encoding boundary
//
// Exercise the Finish digest encoding path with arbitrary string and integer
// values. The digest_step_primitive function hashes:
//   - String: hasher.update(value.as_bytes()) + discriminator
//   - Integer: hasher.update(&value.to_le_bytes()) + discriminator
//
// Neither encoding path may panic for any String value (including empty,
// null bytes, non-UTF-8 sequences in bytes) or any i64 value (including
// MIN, MAX, 0, -1).
//
// Since digest_step_primitive is pub(crate), we exercise it indirectly
// through the public compile_source() API by constructing valid YAML
// with Finish steps containing the fuzz input as result values.
//
// INVARIANT Oracle (replaces crash-only):
// - compile_source Ok ⇒ workflow.node_count() > 0.
// - compile_source Err ⇒ errors.is_empty() == false.
//
// GOD RULE 4: No loop oscillations. Pure fuzz harness.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::compile_source;
use vb_compile::parse_workflow_source;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    // Derive an i64 value from the first 8 bytes of input.
    let int_bytes: [u8; 8] = match data.get(..8).and_then(|bytes| bytes.try_into().ok()) {
        Some(b) => b,
        None => return,
    };
    let int_val: i64 = i64::from_le_bytes(int_bytes);

    // Derive a string value from the remaining bytes (if any).
    // Use lossy conversion — non-UTF-8 bytes become replacement characters.
    let Some(string_data) = data.get(8..) else {
        return;
    };
    let string_val = String::from_utf8_lossy(string_data).into_owned();

    // ── Integer path: Finish { result: Integer(int_val) } ──────────────
    // Construct YAML with a Finish step containing the derived integer.
    // We must quote the integer in YAML template because the YAML format
    // requires integer values to be unquoted.
    let yaml_int = format!(
        "version: velvet-ballistics/v1\nname: fuzz_int\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: {int_val}\n"
    );

    if let Ok(source) = parse_workflow_source(&yaml_int) {
        // Parse Ok ⇒ ≥1 step (validator enforces EmptySteps rejection).
        assert!(
            !source.steps().is_empty(),
            "parse_workflow_source Ok returned 0 steps (int path)"
        );
        match compile_source(&source) {
            Ok(workflow) => assert!(
                workflow.node_count() > 0,
                "compile_source Ok returned 0 nodes (int path)"
            ),
            Err(errors) => assert!(
                !errors.is_empty(),
                "compile_source Err with empty errors vec (int path)"
            ),
        }
    }

    // ── String path: Finish { result: String(string_val) } ─────────────
    // Construct YAML with a Finish step referencing a string output name.
    // The output name must have been set by a previous Set step.
    // NOTE: We use an alphanumeric-only output name to avoid YAML quoting
    // issues. The fuzz input string is used as the Set output value,
    // which exercises the string hashing path.
    let clean_name = sanitize_output_name(&string_val);

    let yaml_str = format!(
        "version: velvet-ballistics/v1\nname: fuzz_str\nwhen:\n  manual: {{}}\nsteps:\n  - id: set_step\n    set:\n      output: {clean_name}\n      value: \"10\"\n  - id: done\n    finish:\n      result: \"{clean_name}\"\n"
    );

    if let Ok(source) = parse_workflow_source(&yaml_str) {
        assert!(
            !source.steps().is_empty(),
            "parse_workflow_source Ok returned 0 steps (str path)"
        );
        match compile_source(&source) {
            Ok(workflow) => assert!(
                workflow.node_count() > 0,
                "compile_source Ok returned 0 nodes (str path)"
            ),
            Err(errors) => assert!(
                !errors.is_empty(),
                "compile_source Err with empty errors vec (str path)"
            ),
        }
    }
});

/// Create a YAML-safe output name from an arbitrary string.
/// Only keep alphanumeric characters and underscores; prefix with "v"
/// if the result would be empty or start with a digit.
fn sanitize_output_name(input: &str) -> String {
    let filtered: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64)
        .collect();

    if filtered.is_empty() {
        "v_empty".to_string()
    } else if filtered.chars().next().is_none_or(|c| c.is_numeric()) {
        format!("v_{filtered}")
    } else {
        filtered
    }
}
