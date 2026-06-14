// Verification artifact: kani_digest_ask_prompt_sensitivity.rs
// PO: PO-KANI-001 — Changing an Ask prompt changes the canonical digest
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_ask_prompt_sensitivity --unwind 10
//
// Proof obligations:
// - PO-KANI-001: Two WorkflowSource values identical except for Ask.prompt
//   produce distinct canonical digests (INV-ASK-001).
//
// GOD RULE 1: Uses kani::any() for all prompt byte generation within bounded lengths.
// GOD RULE 2: Binds to actual Rust canonical_digest() and digest_step_primitive()
//   implementations in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::lwr::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Maximum prompt length for Kani bounded checking.
const MAX_PROMPT_LEN: usize = 20;

/// Construct a minimal WorkflowSource with a single Ask step and the given prompt.
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

/// PO-KANI-001 H1: Prompt sensitivity — two different prompts produce distinct digests.
#[kani::proof]
#[kani::unwind(22)]
fn check_ask_prompt_sensitivity() {
    // Generate a bounded-length prompt A
    let prompt_a_len: usize = kani::any();
    kani::assume(prompt_a_len <= MAX_PROMPT_LEN);
    let mut prompt_a = String::new();
    for _ in 0..prompt_a_len {
        prompt_a.push(kani::any::<char>());
    }
    kani::cover!(!prompt_a.is_empty(), "non-empty prompt A");
    kani::cover!(prompt_a.is_empty(), "empty prompt A");

    // Generate a bounded-length prompt B
    let prompt_b_len: usize = kani::any();
    kani::assume(prompt_b_len <= MAX_PROMPT_LEN);
    let mut prompt_b = String::new();
    for _ in 0..prompt_b_len {
        prompt_b.push(kani::any::<char>());
    }

    // Require that prompts differ
    kani::assume(prompt_a != prompt_b);

    let source_a = source_with_ask_prompt(prompt_a);
    let source_b = source_with_ask_prompt(prompt_b);

    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);

    // The core invariant: different prompts yield different digests
    kani::assert(
        digest_a != digest_b,
        "INV-ASK-001 violated: different Ask prompts produced identical canonical digests",
    );
}
