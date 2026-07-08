// Verification artifact: kani_digest_step_primitive_no_panic.rs
// PO: PO-KANI-006 — digest_step_primitive never panics on any valid StepPrimitive::Ask
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_digest_step_primitive_no_panic --unwind 10
//
// Proof obligations:
// - PO-KANI-006: No panic, unwrap, expect, or unreachable paths in digest_step_primitive
//   for any valid Ask variant within bounded prompt/timeout lengths (TC-007).
//
// GOD RULE 1: Uses kani::any() for prompt/timeout byte generation within bounded lengths.
// GOD RULE 2: Binds to actual Rust digest_step_primitive() and canonical_digest()
//   implementations in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};
use crate::lwr::{canonical_digest, digest_step_primitive};

/// Maximum prompt length for bounded panic-freedom checking.
const MAX_PROMPT_LEN: usize = 256;
/// Maximum timeout length for bounded panic-freedom checking.
const MAX_TIMEOUT_LEN: usize = 128;

/// PO-KANI-006 H1: digest_step_primitive does not panic on Ask variant.
#[kani::proof]
#[kani::unwind(10)]
fn check_digest_step_primitive_no_panic() {
    let prompt_len: usize = kani::any();
    kani::assume(prompt_len <= MAX_PROMPT_LEN);
    let mut prompt_bytes = vec![0u8; prompt_len];
    for i in 0..prompt_len {
        prompt_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let prompt = match String::from_utf8(prompt_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    let has_timeout: bool = kani::any();
    let timeout: Option<String> = if has_timeout {
        let timeout_len: usize = kani::any();
        kani::assume(timeout_len <= MAX_TIMEOUT_LEN);
        let mut timeout_bytes = vec![0u8; timeout_len];
        for i in 0..timeout_len {
            timeout_bytes[i] = kani::any();
        }
        // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
        Some(match String::from_utf8(timeout_bytes) {
            Ok(s) => s,
            Err(_) => {
                kani::assume(false); // exclude invalid UTF-8 from verification domain
                unreachable!();
            }
        })
    } else {
        None
    };

    // Test that digest_step_primitive does not panic on any Ask variant
    let primitive = StepPrimitive::Ask { prompt, timeout };
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, &primitive);
    // If we reach here, digest_step_primitive did not panic
    kani::cover!(true, "digest_step_primitive Ask arm reached without panic");
}

/// PO-KANI-006 H2: canonical_digest does not panic for Ask-containing sources.
#[kani::proof]
#[kani::unwind(10)]
fn check_canonical_digest_no_panic() {
    let prompt_len: usize = kani::any();
    kani::assume(prompt_len <= MAX_PROMPT_LEN);
    let mut prompt_bytes = vec![0u8; prompt_len];
    for i in 0..prompt_len {
        prompt_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let prompt = match String::from_utf8(prompt_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    let has_timeout: bool = kani::any();
    let timeout: Option<String> = if has_timeout {
        let timeout_len: usize = kani::any();
        kani::assume(timeout_len <= MAX_TIMEOUT_LEN);
        let mut timeout_bytes = vec![0u8; timeout_len];
        for i in 0..timeout_len {
            timeout_bytes[i] = kani::any();
        }
        // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
        Some(match String::from_utf8(timeout_bytes) {
            Ok(s) => s,
            Err(_) => {
                kani::assume(false); // exclude invalid UTF-8 from verification domain
                unreachable!();
            }
        })
    } else {
        None
    };

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
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    });

    let _digest = canonical_digest(&source);
    // If we reach here, canonical_digest did not panic
    kani::cover!(
        true,
        "canonical_digest completed without panic for Ask source"
    );
}
