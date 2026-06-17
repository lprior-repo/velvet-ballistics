// Verification artifact: proptest_choose_width.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-PROPTEST-001 — choose_width result >= 1 and matches expected node count
// Command: cargo test -p vb_compile --test proptest_choose_width -- --nocapture
//
// GOD RULE 1: Uses proptest strategies for random branch configurations.
// GOD RULE 2: Binds to production compile_workflow (public API).

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;

fn choose_workflow_yaml(branch_body_counts: &[u8]) -> String {
    let mut yaml =
        String::from("version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n");

    // Setup step: create an output that the finish step can reference
    yaml.push_str("  - id: setup\n    set:\n      output: result\n      value: \"0\"\n");

    // Choose step
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

    // Finish step
    yaml.push_str("  - id: done\n    finish:\n      result: result\n");

    yaml
}

fn body_counts_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..=5u8, 1..=4)
}

proptest! {
    #[test]
    fn choose_width_matches_node_count(body_counts in body_counts_strategy()) {
        let yaml = choose_workflow_yaml(&body_counts);
        let result = vb_compile::compile_workflow(yaml.as_bytes());

        let total_body: u16 = body_counts.iter().map(|&c| u16::from(c)).sum();
        let expected = 3u16.checked_add(total_body).expect("3 + total_body fits u16 (total_body bounded by 4*4=16)");
        prop_assert!(
            matches!(result, Ok(ref wf) if wf.node_count() == expected),
            "node_count {:?} must equal expected {} (Setup + ChooseSlot + body + Finish), result={:?}",
            result.as_ref().map(|w| w.node_count()),
            expected,
            result
        );
    }

    #[test]
    fn choose_width_always_at_least_one(body_counts in body_counts_strategy()) {
        let yaml = choose_workflow_yaml(&body_counts);
        let result = vb_compile::compile_workflow(yaml.as_bytes());

        prop_assert!(
            matches!(result, Ok(ref wf) if wf.node_count() >= 3),
            "must have at least Setup + ChooseSlot + Finish nodes, got {:?}",
            result
        );
    }
}
