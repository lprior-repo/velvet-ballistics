// Verification artifact: proptest_digest_ask_prompt_sensitivity.rs
// PO: PO-PROPTEST-001 — Changing an Ask prompt changes the canonical digest
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: proptest 1.x
// Command: cargo test --test proptest_digest_ask_prompt_sensitivity
//
// Proof obligations:
// - PO-PROPTEST-001: For N=1000 random prompt pairs, digest(A) != digest(B)
//   when prompts differ (INV-ASK-001).
//
// GOD RULE 1: Uses proptest strategies, not hardcoded WorkflowSource values.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_compile::canonical_digest;
use vb_compile::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

fn source_with_ask_prompt(prompt: String) -> WorkflowSource {
    let steps = vec![StepAst {
        id: "ask_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Ask {
            prompt,
            timeout: None,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

proptest! {
    /// PO-PROPTEST-001: Prompt sensitivity property test.
    /// For any pair of different prompt strings, the resulting digests must differ.
    #[test]
    fn prop_digest_prompt_sensitivity(
        prompt_a in "\\PC{0,1024}",
        prompt_b in "\\PC{0,1024}",
    ) {
        // Only test when prompts actually differ
        prop_assume!(prompt_a != prompt_b);

        let source_a = source_with_ask_prompt(prompt_a);
        let source_b = source_with_ask_prompt(prompt_b);

        let digest_a = canonical_digest(&source_a);
        let digest_b = canonical_digest(&source_b);

        prop_assert_ne!(
            digest_a, digest_b,
            "INV-ASK-001 violated: different Ask prompts produced identical canonical digests"
        );
    }
}
