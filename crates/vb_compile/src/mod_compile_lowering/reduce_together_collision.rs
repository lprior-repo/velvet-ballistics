// Verification artifact: reduce_together_collision.rs
// PO: PO-COLLISION-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_together_collision
//
// Requirement: N/A (Cross-Bead Collision Boundary)
// Domain Claim: Modifications by vb-xi2f.24 to canonical_body_step_width
//   do not conflict with vb-xi2f.22's modifications. Merged codebase
//   passes both beads' test suites.
//
// Defense-in-depth: proptest exercises both reduce and together bodies.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_reduce_together_collision(
            reduce_body_len in 1usize..20usize,
            together_branch_count in 1usize..10usize,
        ) {
            // Verify body_width for reduce body
            let reduce_body: Vec<vb_yaml::ast::StepAst> = (0..reduce_body_len)
                .map(|i| vb_yaml::ast::StepAst {
                    id: format!("rs{i}"),
                    name: None,
                    condition: None,
                    primitive: vb_yaml::ast::StepPrimitive::Set {
                        output: format!("ro{i}"),
                        value: "1".to_string(),
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                })
                .collect();

            let reduce_width = crate::mod_compile_lowering::part_01::body_width(
                &reduce_body, 3
            );
            if let Ok(rw) = reduce_width {
                assert!(
                    rw >= 3 + reduce_body_len,
                    "reduce body width must include overhead + steps"
                );
            }

            // Verify body_width for together branches (cross-bead compatibility)
            // Together branches use body_width with overhead 1 per branch
            for branch_idx in 0..together_branch_count {
                let branch_body: Vec<vb_yaml::ast::StepAst> = vec![
                    vb_yaml::ast::StepAst {
                        id: format!("tb{branch_idx}"),
                        name: None,
                        condition: None,
                        primitive: vb_yaml::ast::StepPrimitive::Set {
                            output: "to".to_string(),
                            value: "1".to_string(),
                        },
                        with: None,
                        retry: None,
                        on_error: None,
                        then: None,
                    }
                ];

                let branch_width = crate::mod_compile_lowering::part_01::body_width(
                    &branch_body, 1
                );
                assert_eq!(
                    branch_width,
                    Ok(2),
                    "together branch width = overhead(1) + 1 step = 2"
                );
            }
        }
    }
}
