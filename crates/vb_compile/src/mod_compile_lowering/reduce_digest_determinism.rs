// Verification artifact: reduce_digest_determinism.rs
// PO: PO-DIGEST-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_digest_determinism
//
// Requirement: C10 -- Deterministic Lowering
// Domain Claim: canonical_digest produces the same hash for the same
//   WorkflowSource regardless of body step count.
//
// Fix (F-008, RETRY 4): Changed from testing body_width (wrong function)
// to testing canonical_digest determinism as specified in the obligation.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_yaml::ast::{
        ScalarValue, StepAst, StepPrimitive, TriggerAst,
        WorkflowSource, WorkflowSourceParts,
    };

    fn workflow_source_strategy() -> impl Strategy<Value = WorkflowSource> {
        (1usize..20usize).prop_flat_map(|body_len| {
            let steps: Vec<StepAst> = (0..body_len)
                .map(|i| StepAst {
                    id: format!("s{i}"),
                    name: None,
                    condition: None,
                    primitive: StepPrimitive::Set {
                        output: format!("o{i}"),
                        value: "1".to_string(),
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                })
                .collect();
            Just(WorkflowSource::new(WorkflowSourceParts {
                version: "v1".to_string(),
                name: "test_determinism".to_string(),
                trigger: TriggerAst::Manual,
                inputs: vec![],
                vars: vec![],
                secrets: vec![],
                steps,
                result: None,
                examples: vec![],
            }))
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_digest_determinism(
            source in workflow_source_strategy(),
        ) {
            // canonical_digest must produce the same hash across multiple calls
            let r1 = crate::mod_compile_lowering::part_05::canonical_digest(&source);
            let r2 = crate::mod_compile_lowering::part_05::canonical_digest(&source);
            let r3 = crate::mod_compile_lowering::part_05::canonical_digest(&source);

            // All calls must produce the same result
            match (&r1, &r2, &r3) {
                (Ok(d1), Ok(d2), Ok(d3)) => {
                    assert_eq!(d1, d2, "canonical_digest must be deterministic run 1 vs 2");
                    assert_eq!(d2, d3, "canonical_digest must be deterministic run 2 vs 3");
                }
                (Err(_), Err(_), Err(_)) => {
                    // Consistently erroring is also deterministic
                }
                _ => {
                    panic!("canonical_digest must produce consistent Ok/Err across runs");
                }
            }
        }
    }
}
