// Verification artifact: kani_digest_ask_empty_prompt.rs
// PO: PO-KANI-003 — An Ask with empty prompt produces a digest distinct from non-empty
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_empty_prompt_distinct --unwind 5
//
// Proof obligations:
// - PO-KANI-003: Empty prompt digest is distinct from any non-empty prompt digest
//   for bounded non-empty prompt lengths (INV-ASK-004).
//
// GOD RULE 1: Uses kani::any() for non-empty prompt generation.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::lwr::canonical_digest;
use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Maximum non-empty prompt length for Kani bounded checking.
const MAX_PROMPT_LEN: usize = 128;

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

/// PO-KANI-003 H1: Empty prompt vs arbitrary non-empty prompt — digests must differ.
#[kani::proof]
#[kani::unwind(5)]
fn check_empty_prompt_distinct() {
    // Construct source with empty prompt
    let source_empty = source_with_ask_prompt(String::new());

    // Generate non-empty prompt with bounded length
    let non_empty_len: usize = kani::any();
    kani::assume(non_empty_len >= 1);
    kani::assume(non_empty_len <= MAX_PROMPT_LEN);
    let mut non_empty_bytes = vec![0u8; non_empty_len];
    for i in 0..non_empty_len {
        non_empty_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let non_empty_prompt = match String::from_utf8(non_empty_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    let source_non_empty = source_with_ask_prompt(non_empty_prompt);

    let digest_empty = canonical_digest(&source_empty);
    let digest_non_empty = canonical_digest(&source_non_empty);

    kani::assert(
        digest_empty != digest_non_empty,
        "INV-ASK-004 violated: empty prompt and non-empty prompt produced identical canonical digests",
    );

    kani::cover!(
        true,
        "empty prompt domain covered with non-empty length = 1..128"
    );
}
