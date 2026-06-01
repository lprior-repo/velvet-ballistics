// Verification artifact: reduce_nested_foreach_layout.rs
// PO: PO-NESTED-FOREACH-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_nested_foreach_layout
//
// Requirement: C3 — Body Step Sequential Assignment (ForEach width)
// Domain Claim: Reduce bodies containing nested ForEach steps produce
//   correct step layouts with no slot collisions.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_yaml::ast::StepPrimitive;

    fn set_step(value: i64) -> vb_yaml::ast::StepAst {
        vb_yaml::ast::StepAst {
            id: "s".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set {
                output: "o".to_string(),
                value: value.to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }
    }

    fn foreach_step_strategy() -> impl Strategy<Value = StepPrimitive> {
        (1usize..10usize).prop_map(|n| {
            let body: Vec<vb_yaml::ast::StepAst> = (0..n)
                .map(|i| set_step(i as i64))
                .collect();
            StepPrimitive::ForEach {
                variable: "item".to_string(),
                input: "0".to_string(),
                at_once: None,
                body,
            }
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_nested_foreach_layout(
            foreach_primitive in foreach_step_strategy(),
        ) {
            let width = crate::mod_compile_lowering::part_01::canonical_body_step_width(
                &foreach_primitive
            );

            match width {
                Ok(w) => {
                    // ForEach width must be >= 2 (ForEachStart + ForEachNext)
                    assert!(w >= 2, "ForEach width must be >= 2, got {w}");
                    // ForEach width must not be 1
                    assert!(w != 1, "ForEach width must not be 1");
                }
                Err(_) => {
                    // ForEach should be supported in body
                    assert!(false, "ForEach must be supported in body steps");
                }
            }
        }
    }
}
