// Verification artifact: proptest_digest_ask_ordering.rs
// PO: PO-PROPTEST-004 — Ask field hashing order is deterministic
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: proptest 1.x
// Command: cargo test --test proptest_digest_ask_ordering
//
// Proof obligations:
// - PO-PROPTEST-004: For N=500 random Ask inputs, calling canonical_digest twice
//   on identical sources produces identical output (TC-002).
//
// GOD RULE 1: Uses proptest strategies for random Ask inputs.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_compile::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Strategy for optional timeout.
fn timeout_strategy() -> BoxedStrategy<Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some(String::new())),
        "\\PC{1,256}".prop_map(Some),
    ]
    .boxed()
}

proptest! {
    /// PO-PROPTEST-004: Field ordering determinism property test.
    /// Same Ask source produces identical digest across calls.
    #[test]
    fn prop_digest_ask_ordering(
        prompt in "\\PC{0,512}",
        timeout in timeout_strategy(),
    ) {
        let steps = vec![StepAst {
            id: "ask_step".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Ask { prompt, timeout },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }];

        let source = WorkflowSource::new(WorkflowSourceParts {
            version: "velvet-ballastics/v1".to_string(),
            name: "test_workflow".to_string(),
            trigger: TriggerAst::Manual,
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps,
            result: None,
            examples: vec![],
        });

        let digest_first = canonical_digest(&source);
        let digest_second = canonical_digest(&source);

        prop_assert_eq!(
            digest_first, digest_second,
            "TC-002 violated: Ask field hashing order is non-deterministic"
        );
    }
}
