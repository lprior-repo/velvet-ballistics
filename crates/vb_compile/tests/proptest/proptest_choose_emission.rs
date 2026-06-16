// Verification artifact: proptest_choose_emission.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-005 — Emitted node count equals choose_width(branches)
// Command: cargo test -p vb_compile --test proptest_choose_emission -- --nocapture
//
// GOD RULE 1: Uses proptest strategies.
// GOD RULE 2: Binds to compile_workflow.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{CompiledNodeKind, StepIdx};

fn choose_yaml(branch_body_counts: &[u8]) -> String {
    let mut yaml =
        String::from("version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n");
    yaml.push_str("  - id: setup\n    set:\n      output: result\n      value: \"1\"\n");
    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");
    for (i, &count) in branch_body_counts.iter().enumerate() {
        yaml.push_str(&format!("        - when: \"{}\"\n", i));
        if count == 0 {
            yaml.push_str("          steps: []\n");
        } else {
            yaml.push_str("          steps:\n");
            for j in 0..count {
                yaml.push_str(&format!(
                    "            - id: b{i}s{j}\n              set:\n                output: out_{i}_{j}\n                value: \"42\"\n"
                ));
            }
        }
    }
    yaml.push_str("      otherwise: done\n");
    yaml.push_str("  - id: done\n    finish:\n      result: result\n");
    yaml
}

fn body_counts_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..=4u8, 1..=6)
}

proptest! {
    #[test]
    fn emission_count_matches_body_steps(body_counts in body_counts_strategy()) {
        let yaml = choose_yaml(&body_counts);
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        prop_assert!(result.is_ok(), "compile_workflow failed: {:?}", result.err());

        if let Ok(workflow) = result {
            let total_body: u16 = body_counts.iter().map(|&c| u16::from(c)).sum();
            let expected = 3u16.checked_add(total_body).expect("3 + total_body fits u16 (total_body bounded by 6*4=24)"); // Setup + ChooseSlot + body + Finish
            prop_assert_eq!(
                workflow.node_count(), expected,
                "node_count must equal Setup + ChooseSlot + sum(body) + Finish"
            );
        }
    }

    #[test]
    fn first_node_after_setup_is_choose_slot(body_counts in body_counts_strategy()) {
        let yaml = choose_yaml(&body_counts);
        if let Ok(workflow) = vb_compile::compile_workflow(yaml.as_bytes()) {
            // After the Setup step (node 0), the next emitted node should be ChooseSlot
            let choose_node = workflow.node(StepIdx::new(1));
            prop_assert!(choose_node.is_some(), "node 1 must exist");
            if let Some(n) = choose_node {
                prop_assert!(
                    matches!(n.kind, CompiledNodeKind::ChooseSlot { .. }),
                    "node 1 must be ChooseSlot, got {:?}", n.kind
                );
            }
        }
    }
}
