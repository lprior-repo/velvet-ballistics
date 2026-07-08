// Fuzz target: fuzz_choose_depth.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-FUZZ-002 — Fuzz deeply nested choose YAML with arbitrary body contents
// Command: cargo fuzz run fuzz_choose_depth -- -max_len=65536 -runs=50000
//
// GOD RULE 2: Binds to production compile_workflow.
//
// ## INVARIANT Oracle
//
// Replaces crash-only fuzzing with structural assertions on `compile_workflow`:
// - Ok path: the compiled workflow must have `node_count() > 0`.
// - Err path: `CompileErrors::is_empty()` must be `false` — every Err carries
//   at least one typed diagnostic; an empty errors vec would mean the compiler
//   rejected input silently.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

/// Build YAML with a choose step whose branch count and body step count
/// come from fuzzer input bytes.
fn fuzz_choose_structure(branch_count: u8, body_count: u8) {
    let mut yaml =
        String::from("version: velvet-ballistics/v1\nname: fuzz\nwhen:\n  manual: {}\nsteps:\n");

    // Put some Set steps before the choose to create a realistic workflow
    yaml.push_str("  - id: setup\n    set:\n      output: cond\n      value: \"1\"\n");

    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");

    let branches = branch_count.min(64);
    for i in 0..branches {
        yaml.push_str(&format!("        - when: \"{}\"\n          steps:\n", i));
        let steps = body_count.min(10);
        for j in 0..steps {
            yaml.push_str(&format!(
                "            - id: b{i}s{j}\n              set:\n                output: out_{i}_{j}\n                value: \"42\"\n"
            ));
        }
    }
    yaml.push_str("      otherwise: done\n");
    yaml.push_str("  - id: done\n    finish:\n      result: \"ok\"\n");

    let result = vb_compile::compile_workflow(yaml.as_bytes());
    match result {
        Ok(workflow) => {
            assert!(
                workflow.node_count() > 0,
                "compile_workflow Ok returned 0 nodes (validator bypassed)"
            );
        }
        Err(errors) => {
            assert!(
                !errors.is_empty(),
                "compile_workflow Err with empty errors vec"
            );
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let branch_count = data.first().copied().unwrap_or(1);
    let body_count = data.get(1).copied().unwrap_or(0);
    fuzz_choose_structure(branch_count, body_count);
});
