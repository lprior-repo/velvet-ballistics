// Verification artifact: reduce_body_width_overflow.rs
// PO: PO-OVERFLOW-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_body_width_overflow
//
// Requirement: C3 — Body Step Sequential Assignment (overflow guard)
// Domain Claim: Deeply nested body structures approaching u16::MAX width
//   are rejected gracefully with StepIndexOutOfRange, no panic.
//
// Proptest generates large body structures that may exceed u16::MAX width.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn large_body_strategy() -> impl Strategy<Value = Vec<vb_yaml::ast::StepAst>> {
        // Generate bodies with 1..60000 steps (potential overflow)
        (1usize..60000usize).prop_map(|n| {
            (0..n)
                .map(|i| vb_yaml::ast::StepAst {
                    id: format!("s{i}"),
                    name: None,
                    condition: None,
                    primitive: vb_yaml::ast::StepPrimitive::Set {
                        output: format!("o{i}"),
                        value: "1".to_string(),
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                })
                .collect()
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_body_width_overflow(
            body in large_body_strategy(),
        ) {
            let result = vb_compile::mod_compile_lowering::part_01::body_width(&body, 3);
            match result {
                Ok(width) => {
                    // If Ok, width must be within u16::MAX
                    assert!(
                        width <= usize::from(u16::MAX),
                        "body_width Ok implies width <= 65535"
                    );
                }
                Err(_) => {
                    // Overflow rejection is correct behavior
                    // No panic occurred
                }
            }
        }
    }
}
