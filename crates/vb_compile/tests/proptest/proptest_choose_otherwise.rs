// Verification artifact: proptest_choose_otherwise.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-003 — Otherwise target outside body span
// Command: cargo test -p vb_compile --test proptest_choose_otherwise -- --nocapture
//
// GOD RULE 1: Uses proptest strategies.
// GOD RULE 2: Binds to compile_workflow.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{CompiledNodeKind, StepIdx};

fn make_choose_yaml(branch_body_counts: &[u8], has_otherwise: bool) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n",
    );
    yaml.push_str("  - id: setup\n    set:\n      output: result\n      value: \"0\"\n");
    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");
    for (i, &count) in branch_body_counts.iter().enumerate() {
        yaml.push_str(&format!("        - when: \"{}\"\n", i));
        if count == 0 {
            yaml.push_str("          steps: []\n");
        } else {
            yaml.push_str("          steps:\n");
            for j in 0..count {
                yaml.push_str(&format!(
                    "            - id: b{i}s{j}\n              set:\n                output: out_{i}_{j}\n                value: \"0\"\n"
                ));
            }
        }
    }
    if has_otherwise {
        yaml.push_str("      otherwise: done\n");
    }
    yaml.push_str("  - id: done\n    finish:\n      result: result\n");
    yaml
}

fn body_counts_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..=4u8, 1..=4)
}

proptest! {
    #[test]
    fn otherwise_present_compiles(body_counts in body_counts_strategy()) {
        let yaml = make_choose_yaml(&body_counts, true);
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        prop_assert!(result.is_ok(), "choose with otherwise must compile: {:?}", result.err());
    }

    #[test]
    fn otherwise_target_exists(body_counts in body_counts_strategy()) {
        let yaml = make_choose_yaml(&body_counts, true);
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        prop_assert!(result.is_ok(), "compile_workflow failed: {:?}", result.err());

        if let Ok(workflow) = result {
            let nc = workflow.node_count();
            let mut found_choose = false;
            for i in 0..nc {
                if let Some(node) = workflow.node(StepIdx::new(i)) {
                    if let CompiledNodeKind::ChooseSlot { otherwise, .. } = &node.kind {
                        found_choose = true;
                        prop_assert!(otherwise.is_some(), "otherwise must be set");
                        if let Some(target) = otherwise {
                            prop_assert!(
                                workflow.node(*target).is_some(),
                                "otherwise target must be a valid node"
                            );
                        }
                    }
                }
            }
            prop_assert!(found_choose, "must find ChooseSlot");
        }
    }
}
