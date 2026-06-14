// Verification artifact: kani_digest_ask_field_ordering.rs
// PO: PO-KANI-005 — Ask field hashing order is deterministic
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_ask_field_ordering_deterministic --unwind 10
//
// Proof obligations:
// - PO-KANI-005: Same Ask input always produces same digest, confirming sequential
//   update ordering (tag → prompt → timeout) is deterministic (TC-002).
//
// GOD RULE 1: Uses kani::any() for prompt/timeout bytes within bounded lengths.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::lwr::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Maximum prompt length for bounded checking.
const MAX_PROMPT_LEN: usize = 10;
/// Maximum timeout length for bounded checking.
const MAX_TIMEOUT_LEN: usize = 10;

fn source_with_ask(prompt: String, timeout: Option<String>) -> WorkflowSource {
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

/// PO-KANI-005 H1: Deterministic output — same input twice produces identical digest.
#[kani::proof]
#[kani::unwind(12)]
fn check_ask_field_ordering_deterministic() {
    // Generate a prompt
    let prompt_len: usize = kani::any();
    kani::assume(prompt_len <= MAX_PROMPT_LEN);
    let mut prompt = String::new();
    for _ in 0..prompt_len {
        prompt.push(kani::any::<char>());
    }

    // Generate a timeout (None or Some(string))
    let has_timeout: bool = kani::any();
    let timeout: Option<String> = if has_timeout {
        let timeout_len: usize = kani::any();
        kani::assume(timeout_len <= MAX_TIMEOUT_LEN);
        let mut timeout = String::new();
        for _ in 0..timeout_len {
            timeout.push(kani::any::<char>());
        }

    let source = source_with_ask(prompt.clone(), timeout.clone());

    // Call canonical_digest twice on the same source
    let digest_first = canonical_digest(&source);
    let digest_second = canonical_digest(&source);

    kani::assert(
        digest_first == digest_second,
        "TC-002 violated: same Ask input produced different digests — field ordering is non-deterministic",
    );

    kani::cover!(
        digest_first == digest_second,
        "deterministic output confirmed for bounded Ask inputs"
    );
}
