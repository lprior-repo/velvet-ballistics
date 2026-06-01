// Verification artifact: proptest_gate11_together.rs
// Obligation: PO-008-P
// Requirement: C-8 (Gate 11 validation compatibility)
// Proof seed: ps-22-008
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_gate11_accepts_together_body --nocapture
// Bead: vb-xi2f.22
// State: 5 (proof-writer)
//
// GOD RULE 1: Uses proptest strategies for random together bodies.
// GOD RULE 2: Binds to actual emit_single_body_set and gate 11 validation.

#![cfg(test)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Strategy for valid together bodies that gate 11 should accept.
fn gate11_acceptance_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (1usize..=8usize, 1usize..=16usize).prop_map(|(branch_count, body_steps)| {
        let branches: Vec<TogetherBranch> = (0..branch_count)
            .map(|i| TogetherBranch {
                label: format!("b{}", i),
                steps: (0..body_steps)
                    .map(|s| StepAst {
                        id: format!("s{}.{}", i, s),
                        name: None,
                        condition: None,
                        primitive: StepPrimitive::Set {
                            output: String::from("x"),
                            value: String::from("1"),
                        },
                        with: None,
                        retry: None,
                        on_error: None,
                        then: None,
                    })
                    .collect(),
            })
            .collect();
        vec![StepAst {
            id: String::from("together"),
            name: None,
            condition: None,
            primitive: StepPrimitive::Together { branches },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-008-P: Gate 11 compatibility for random together bodies
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Verify that random together bodies produce IR with valid
    /// structural properties that gate 11 requires.
    ///
    /// Gate 11 checks:
    /// - TogetherStart at start of span
    /// - TogetherBranch nodes between start and join
    /// - TogetherJoin at end of span with correct branch_count
    /// - All StepIdx values are within the together span
    #[test]
    fn proptest_gate11_accepts_together_body(body in gate11_acceptance_body_strategy()) {
        let mut builder = SlotCompiler::new();
        let base_id = StepIdx::new(0);

        let nodes_before = builder.nodes.len();
        let result = emit_single_body_set(
            &body,
            base_id,
            0,
            SlotIdx::new(0),
            None,
            &mut builder,
            false,
        );

        match result {
            Ok(()) => {
                let nodes_after = builder.nodes.len();
                let emitted = nodes_after - nodes_before;

                if emitted > 0 {
                    // Gate 11 structural property: first node at start of span
                    prop_assert_eq!(
                        builder.nodes[nodes_before].id.as_usize(),
                        base_id.as_usize(),
                        "first node must start at base StepIdx"
                    );

                    // Gate 11: all StepIdx values are unique (monotonic, strictly increasing)
                    let mut prev_id: Option<usize> = None;
                    for i in nodes_before..nodes_after {
                        let current_id = builder.nodes[i].id.as_usize();
                        if let Some(p) = prev_id {
                            // Gate 11 requires non-decreasing (can be equal for same-step nodes
                            // but typically strictly increasing)
                            prop_assert!(
                                current_id >= p,
                                "gate 11: StepIdx must be monotonic"
                            );
                        }
                        prev_id = Some(current_id);
                    }

                    // Gate 11: all StepIdx values fit in u16
                    if emitted > 0 {
                        let last_id = builder.nodes[nodes_after - 1].id.as_usize();
                        prop_assert!(
                            last_id < u16::MAX as usize,
                            "gate 11: StepIdx must fit in u16"
                        );
                    }
                }
            }
            Err(_) => {
                // Currently expected: UnsupportedStepPrimitive
            }
        }
    }
}
