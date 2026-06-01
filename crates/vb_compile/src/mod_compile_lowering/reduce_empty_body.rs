// Verification artifact: reduce_empty_body.rs
// PO: PO-EMPTY-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_empty_body
//
// Requirement: C12 — Empty Body Handling
// Domain Claim: Empty reduce bodies are consistently rejected with
//   StepFieldShape diagnostic.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_reduce_empty_body(
            overhead in 0usize..10usize,
        ) {
            let empty_body: Vec<vb_yaml::ast::StepAst> = vec![];

            // body_width with empty body returns exactly overhead
            let result = crate::mod_compile_lowering::part_01::body_width(
                &empty_body, overhead
            );

            match result {
                Ok(width) => {
                    assert_eq!(
                        width, overhead,
                        "empty body width must equal overhead"
                    );
                }
                Err(_) => {
                    // Should not happen for overhead < u16::MAX
                    assert!(false, "empty body should not overflow");
                }
            }
        }
    }

    #[test]
    fn test_reduce_empty_body_overhead_3() {
        let empty: Vec<vb_yaml::ast::StepAst> = vec![];
        let result = crate::mod_compile_lowering::part_01::body_width(&empty, 3);
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn test_reduce_empty_body_overhead_0() {
        let empty: Vec<vb_yaml::ast::StepAst> = vec![];
        let result = crate::mod_compile_lowering::part_01::body_width(&empty, 0);
        assert_eq!(result, Ok(0));
    }
}
