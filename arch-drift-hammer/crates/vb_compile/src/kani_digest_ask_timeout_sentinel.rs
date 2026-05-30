// Verification artifact: kani_digest_ask_timeout_sentinel.rs
// PO: PO-KANI-004 — timeout None vs Some("") produce distinct digest contributions
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
// Verifier: Kani 0.67.0
// Command: cargo kani --package vb_compile --harness check_timeout_sentinel_distinction --unwind 5
//
// Proof obligations:
// - PO-KANI-004: Sentinel b"no_timeout" (for None) and b"timeout" + b"" (for Some(""))
//   produce distinct hash states with no collision (INV-ASK-005).
//
// GOD RULE 1: Generates concrete Ask variants (None, Some("")) directly — no kani::any() needed for
//   this specific sentinel distinction check.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation in crate::lwr.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::lwr::canonical_digest;
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

/// PO-KANI-004 H1: Sentinel distinction — None vs Some("") must produce different digests.
#[kani::proof]
#[kani::unwind(5)]
fn check_timeout_sentinel_distinction() {
    let source_none = source_with_ask_timeout(None);
    let source_empty = source_with_ask_timeout(Some(String::new()));

    let digest_none = canonical_digest(&source_none);
    let digest_empty = canonical_digest(&source_empty);

    kani::assert(
        digest_none != digest_empty,
        "INV-ASK-005 violated: b\"no_timeout\" sentinel and b\"timeout\" + b\"\" produced identical digests",
    );

    kani::cover!(
        digest_none != digest_empty,
        "sentinel distinction verified: None and Some(\"\") have distinct digest contributions"
    );
}
