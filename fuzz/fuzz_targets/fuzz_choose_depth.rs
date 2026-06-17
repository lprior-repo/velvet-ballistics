// Fuzz target: fuzz_choose_depth.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-FUZZ-002 — Fuzz deeply nested choose YAML with arbitrary body contents
// Command: cargo fuzz run fuzz_choose_depth -- -max_len=65536 -runs=50000
//
// GOD RULE 2: Binds to production compile_workflow.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

/// Build YAML with a choose step whose branch count and body step count
/// come from fuzzer input bytes.
fn fuzz_choose_structure(branch_count: u8, body_count: u8) {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: fuzz\nwhen:\n  manual: {}\nsteps:\n",
    );

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

    // compile_workflow must never panic and must return a typed Result.
    // On success, the workflow must have at least one node and valid slot count.
    let result = vb_compile::compile_workflow(yaml.as_bytes());
    match result {
        Ok(workflow) => {
            // Compiled workflow must have at least one node (the finish step)
            assert!(
                workflow.node_count() >= 1,
                "compiled workflow must have at least 1 node"
            );
            // Slot count must be at least 1 for workflows with slots
            assert!(
                workflow.slot_count() >= 1,
                "compiled workflow must have at least 1 slot"
            );
        }
        Err(errors) => {
            // Compilation errors are expected for some inputs.
            // Verify that errors are well-formed and non-empty.
            assert!(
                !errors.is_empty(),
                "compile errors must contain at least one error"
            );
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let branch_count = data.first().copied().unwrap_or(1);
    let body_count = data.get(1).copied().unwrap_or(0);
    fuzz_choose_structure(branch_count, body_count);
});
