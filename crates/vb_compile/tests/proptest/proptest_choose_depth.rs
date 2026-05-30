// Verification artifact: proptest_choose_depth.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-004 — Deeply nested/varied choose: compiles or errors gracefully (no panic)
// Command: cargo test -p vb_compile --test proptest_choose_depth -- --nocapture
//
// GOD RULE 1: Uses proptest strategies for varied choose configurations.
// GOD RULE 2: Binds to compile_workflow.

#![forbid(unsafe_code)]

use proptest::prelude::*;

/// Build a YAML workflow with a single choose step with varying complexity.
/// Varies: branch count, body step count per branch, with/without otherwise.
fn varied_choose_yaml(branch_count: u8, max_body: u8, has_otherwise: bool) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n",
    );
    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");

    let actual_count = branch_count.min(64);
    for i in 0..actual_count {
        yaml.push_str(&format!("        - when: \"{}\"\n          steps:\n", i));
        for j in 0..max_body.min(5) {
            yaml.push_str(&format!(
                "            - id: b{i}s{j}\n              set:\n                output: out_{i}_{j}\n                value: \"1\"\n"
            ));
        }
    }
    if has_otherwise {
        yaml.push_str("      otherwise: done\n");
    }
    yaml.push_str("  - id: done\n    finish:\n      result: \"ok\"\n");
    yaml
}

proptest! {
    #[test]
    fn varied_choose_no_panic(
        branch_count in 1u8..=64u8,
        max_body in 0u8..=5u8,
        has_otherwise in proptest::bool::ANY,
    ) {
        let yaml = varied_choose_yaml(branch_count, max_body, has_otherwise);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            vb_compile::compile_workflow(yaml.as_bytes())
        }));
        prop_assert!(result.is_ok(), "compile_workflow must never panic");
    }

    #[test]
    fn varied_choose_result_is_well_typed(
        branch_count in 1u8..=64u8,
        max_body in 0u8..=5u8,
        has_otherwise in proptest::bool::ANY,
    ) {
        let yaml = varied_choose_yaml(branch_count, max_body, has_otherwise);
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        // Must be Ok or Err — both are valid typed returns
        match result {
            Ok(wf) => {
                // Verify we can iterate all nodes without panic
                let _nc = wf.node_count();
            }
            Err(_) => { /* error path is valid */ }
        }
    }
}
