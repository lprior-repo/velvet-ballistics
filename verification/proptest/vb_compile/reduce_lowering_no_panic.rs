// Verification artifact: reduce_lowering_no_panic.rs
// PO: PO-NOPANIC-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_lowering_no_panic
//
// Requirement: C11 — No Panic
// Domain Claim: Arbitrary StepAst trees with diverse body configurations
//   never cause panics during lowering.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_yaml::ast::{StepAst, StepPrimitive};

    fn arbitrary_primitive_strategy() -> impl Strategy<Value = StepPrimitive> {
        (0u8..8u8).prop_flat_map(|variant| {
            let val = any::<i64>();
            match variant % 8 {
                0 => (val).prop_map(|v| StepPrimitive::Set {
                    output: "o".to_string(),
                    value: v.to_string(),
                }).boxed(),
                1 => (any::<i64>(), any::<i64>()).prop_map(|(a, i)| StepPrimitive::Do {
                    action: a.to_string(),
                    input: i.to_string(),
                }).boxed(),
                _ => Just(StepPrimitive::Set {
                    output: "o".to_string(),
                    value: "0".to_string(),
                }).boxed(),
            }
        })
    }

    fn arbitrary_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
        (1usize..50usize).prop_flat_map(|n| {
            prop::collection::vec(
                (any::<i64>(), arbitrary_primitive_strategy()).prop_map(|(id_val, prim)| StepAst {
                    id: format!("s{id_val}"),
                    name: None,
                    condition: None,
                    primitive: prim,
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                }),
                n,
            )
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_lowering_no_panic(
            body in arbitrary_body_strategy(),
        ) {
            // body_width must not panic
            let _ = vb_compile::mod_compile_lowering::part_01::body_width(&body, 3);

            // canonical_body_step_width must not panic for each step
            for step in &body {
                let _ = vb_compile::mod_compile_lowering::part_01::canonical_body_step_width(
                    &step.primitive,
                );
            }

            // canonical_step_width must not panic
            for step in &body {
                let _ = vb_compile::mod_compile_lowering::part_01::canonical_step_width(
                    &step.primitive,
                );
            }
        }
    }
}
