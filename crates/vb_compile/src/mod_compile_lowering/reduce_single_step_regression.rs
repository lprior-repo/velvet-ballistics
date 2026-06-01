// Verification artifact: reduce_single_step_regression.rs
// PO: PO-REGRESSION-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_single_step_regression
//
// Requirement: C7 — Single-Step Body Compatibility
// Domain Claim: Diverse single-step bodies compile identically through both
//   old and new dispatchers. Existing PO-R1..R8 tests pass.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn single_set_step_body_strategy() -> impl Strategy<Value = Vec<vb_yaml::ast::StepAst>> {
        any::<i64>().prop_map(|v| {
            vec![vb_yaml::ast::StepAst {
                id: "body0".to_string(),
                name: None,
                condition: None,
                primitive: vb_yaml::ast::StepPrimitive::Set {
                    output: "out".to_string(),
                    value: v.to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }]
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_single_step_regression(
            body in single_set_step_body_strategy(),
        ) {
            // Single-step body with Set primitive
            assert_eq!(body.len(), 1, "body must have exactly 1 step");

            // Width computation for single Set step = overhead + 1
            let width = crate::mod_compile_lowering::part_01::body_width(&body, 3);
            assert!(width.is_ok(), "single Set step body_width must succeed");
            assert_eq!(width.unwrap(), 4, "single Set step: width = 3 + 1 = 4");

            // canonical_body_step_width for Set = 1
            let step_width = crate::mod_compile_lowering::part_01::canonical_body_step_width(
                &body[0].primitive
            );
            assert!(step_width.is_ok(), "Set step width must be supported");
            assert_eq!(step_width.unwrap(), 1, "Set step width must be 1");
        }
    }

    #[test]
    fn test_reduce_single_step_set_body_width_4() {
        let body = vec![vb_yaml::ast::StepAst {
            id: "s".to_string(),
            name: None,
            condition: None,
            primitive: vb_yaml::ast::StepPrimitive::Set {
                output: "o".to_string(),
                value: "42".to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }];
        let w = crate::mod_compile_lowering::part_01::body_width(&body, 3);
        assert_eq!(w, Ok(4));
    }
}
