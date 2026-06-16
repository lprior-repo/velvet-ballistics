// Verification artifact: proptest_choose_fallthrough.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-002 — Body edge nodes chain to common_next
// Command: cargo test -p vb_compile --test proptest_choose_fallthrough -- --nocapture
//
// GOD RULE 1: Uses proptest strategies.
// GOD RULE 2: Binds to compile_workflow.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{CompiledNodeKind, StepIdx};

fn make_choose_yaml(branch_body_counts: &[u8]) -> String {
    let mut yaml =
        String::from("version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n");
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
    prop::collection::vec(0u8..=4u8, 1..=4)
        .prop_filter("at least one body step", |v| v.iter().any(|&c| c > 0))
}

proptest! {
    #[test]
    fn body_nodes_have_next_pointer(body_counts in body_counts_strategy()) {
        let yaml = make_choose_yaml(&body_counts);
        let result = vb_compile::compile_workflow(yaml.as_bytes());
        prop_assert!(result.is_ok(), "compile_workflow failed: {:?}", result.err());

        if let Ok(workflow) = result {
            let nc = workflow.node_count();
            let mut found_choose = false;
            for i in 0..nc {
                let node = workflow.node(StepIdx::new(i));
                if let Some(n) = node
                    && matches!(n.kind, CompiledNodeKind::ChooseSlot { .. })
                {
                    found_choose = true;
                    let next_i = i.checked_add(1).expect("i < nc so i+1 fits in usize");
                    for j in next_i..nc {
                        if let Some(bn) = workflow.node(StepIdx::new(j)) {
                            match &bn.kind {
                                CompiledNodeKind::Finish { .. } => break,
                                _ => {
                                    prop_assert!(bn.next.is_some(),
                                        "body node {} must have next pointer", j);
                                }
                            }
                        }
                    }
                    break;
                }
            }
            prop_assert!(found_choose, "must find ChooseSlot node");
        }
    }
}
