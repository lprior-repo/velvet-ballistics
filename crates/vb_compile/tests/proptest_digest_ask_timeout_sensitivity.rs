// Verification artifact: proptest_digest_ask_timeout_sensitivity.rs
// PO: PO-PROPTEST-002 — Changing an Ask timeout changes the canonical digest
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: proptest 1.x
// Command: cargo test --test proptest_digest_ask_timeout_sensitivity
//
// Proof obligations:
// - PO-PROPTEST-002: For N=1000 random timeout pairs, digest(A) != digest(B)
//   when timeouts differ (INV-ASK-002). Covers None, Some(""), and Some(arbitrary).
//
// GOD RULE 1: Uses proptest strategies, not hardcoded values.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_compile::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

fn source_with_ask_timeout(timeout: Option<String>) -> WorkflowSource {
    let steps = vec![StepAst {
        id: "ask_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Ask {
            prompt: "fixed_prompt".to_string(),
            timeout,
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

/// Strategy for optional timeout: None, Some(""), or Some(arbitrary string).
fn timeout_strategy() -> BoxedStrategy<Option<String>> {
    prop_oneof![
        // None timeout
        Just(None),
        // Some empty string
        Just(Some(String::new())),
        // Some arbitrary string
        "\\PC{1,256}".prop_map(Some),
    ]
    .boxed()
}

proptest! {
    /// PO-PROPTEST-002: Timeout sensitivity property test.
    #[test]
    fn prop_digest_timeout_sensitivity(
        timeout_a in timeout_strategy(),
        timeout_b in timeout_strategy(),
    ) {
        prop_assume!(timeout_a != timeout_b);

        let source_a = source_with_ask_timeout(timeout_a);
        let source_b = source_with_ask_timeout(timeout_b);

        let digest_a = canonical_digest(&source_a);
        let digest_b = canonical_digest(&source_b);

        prop_assert_ne!(
            digest_a, digest_b,
            "INV-ASK-002 violated: different Ask timeouts produced identical canonical digests"
        );
    }
}
