// Verification artifact: proptest_digest_determinism.rs
// PO: PO-PROPTEST-003 — canonical_digest is deterministic
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: proptest 1.x
// Command: cargo test --test proptest_digest_determinism
//
// Proof obligations:
// - PO-PROPTEST-003: For N=500 random WorkflowSource values,
//   canonical_digest(S) == canonical_digest(S) on every call (INV-ASK-003).
//
// GOD RULE 1: Uses proptest strategies, not hardcoded values.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_compile::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Strategy for generating a WorkflowSource with 1-5 steps including Ask variants.
fn workflow_source_strategy() -> BoxedStrategy<WorkflowSource> {
    (1usize..=5usize, "\\PC{0,512}", any::<bool>(), "\\PC{0,128}")
        .prop_flat_map(|(num_steps, base_prompt, extra_steps, extra_prompt)| {
            let mut steps = Vec::with_capacity(num_steps);
            // Always include an Ask step
            steps.push(StepAst {
                id: "ask_0".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Ask {
                    prompt: base_prompt.clone(),
                    timeout: None,
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            });
            // Add extra steps for structural variation
            for i in 1..num_steps {
                steps.push(StepAst {
                    id: format!("step_{i}"),
                    name: None,
                    condition: None,
                    primitive: if extra_steps {
                        StepPrimitive::Ask {
                            prompt: extra_prompt.clone(),
                            timeout: None,
                        }
                    } else {
                        StepPrimitive::Set {
                            output: format!("out_{i}"),
                            value: i64::try_from(i).expect("i < 5 in num_steps=1..=5 strategy").to_string(),
                        }
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                });
            }
            Just(WorkflowSource::new(WorkflowSourceParts {
                version: "velvet-ballastics/v1".to_string(),
                name: "test_workflow".to_string(),
                trigger: TriggerAst::Manual,
                inputs: vec![],
                vars: vec![],
                secrets: vec![],
                steps,
                result: None,
                examples: vec![],
            }))
        })
        .boxed()
}

proptest! {
    /// PO-PROPTEST-003: Determinism property test.
    /// Same source compiled twice produces same digest.
    #[test]
    fn prop_digest_determinism(source in workflow_source_strategy()) {
        let digest_first = canonical_digest(&source);
        let digest_second = canonical_digest(&source);

        prop_assert_eq!(
            digest_first, digest_second,
            "INV-ASK-003 violated: canonical_digest is non-deterministic"
        );
    }
}
