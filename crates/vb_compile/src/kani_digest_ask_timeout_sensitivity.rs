// Verification artifact: kani_digest_ask_timeout_sensitivity.rs
// PO: PO-KANI-002 — Changing an Ask timeout changes the canonical digest
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_ask_timeout_sensitivity --unwind 10
//
// Proof obligations:
// - PO-KANI-002: Two WorkflowSource values identical except for Ask.timeout
//   produce distinct canonical digests (INV-ASK-002).
//
// GOD RULE 1: Uses kani::any() for timeout string generation within bounded lengths.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::lwr::canonical_digest;
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

/// Maximum timeout value length for Kani bounded checking.
const MAX_TIMEOUT_LEN: usize = 256;

/// Construct a minimal WorkflowSource with a single Ask step and the given timeout.
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

/// PO-KANI-002 H1: Timeout sensitivity — None vs Some produce distinct digests.
#[kani::proof]
#[kani::unwind(10)]
fn check_ask_timeout_sensitivity() {
    // Case 1: None vs Some(arbitrary string)
    let timeout_len: usize = kani::any();
    kani::assume(timeout_len <= MAX_TIMEOUT_LEN);
    let mut timeout_bytes = vec![0u8; timeout_len];
    for i in 0..timeout_len {
        timeout_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let timeout_str = match String::from_utf8(timeout_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    let source_none = source_with_ask_timeout(None);
    let source_some = source_with_ask_timeout(Some(timeout_str));

    let digest_none = canonical_digest(&source_none);
    let digest_some = canonical_digest(&source_some);

    kani::assert(
        digest_none != digest_some,
        "INV-ASK-002 violated: None vs Some timeout produced identical canonical digests",
    );
}

/// PO-KANI-002 H2: Timeout sensitivity — Some(v1) vs Some(v2) produce distinct digests.
#[kani::proof]
#[kani::unwind(10)]
fn check_ask_timeout_sensitivity_different_values() {
    let timeout1_len: usize = kani::any();
    kani::assume(timeout1_len <= MAX_TIMEOUT_LEN);
    let mut timeout1_bytes = vec![0u8; timeout1_len];
    for i in 0..timeout1_len {
        timeout1_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let timeout1 = match String::from_utf8(timeout1_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    let timeout2_len: usize = kani::any();
    kani::assume(timeout2_len <= MAX_TIMEOUT_LEN);
    let mut timeout2_bytes = vec![0u8; timeout2_len];
    for i in 0..timeout2_len {
        timeout2_bytes[i] = kani::any();
    }
    // Restrict Kani to valid UTF-8 byte sequences to avoid harness-level panic
    let timeout2 = match String::from_utf8(timeout2_bytes) {
        Ok(s) => s,
        Err(_) => {
            kani::assume(false); // exclude invalid UTF-8 from verification domain
            unreachable!();
        }
    };

    kani::assume(timeout1 != timeout2);

    let source_a = source_with_ask_timeout(Some(timeout1));
    let source_b = source_with_ask_timeout(Some(timeout2));

    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);

    kani::assert(
        digest_a != digest_b,
        "INV-ASK-002 violated: different Some timeout values produced identical canonical digests",
    );
}
