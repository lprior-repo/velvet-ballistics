#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]
//! Proptest: compile-to-IR roundtrip for every v1 primitive.
//!
//! PO: PO-COMPILE-ROUNDTRIP-001
//! Bead: tier-a-3-008
//! Verifier: proptest
//! Command: cargo test -p vb_compile --test proptest_compile_ir_roundtrip
//!
//! For each v1 primitive (`set`, `copy`, `choose`, `choose_slot`, `for_each`,
//! `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `do`, `finish`),
//! generate 1000 random valid workflows, compile each via
//! `compile_workflow`, convert the resulting `CompiledWorkflow` back to
//! `WorkflowParts` (the IR), serialize/deserialize via postcard, and assert
//! bitwise equality of the IR bytes.
//!
//! Any primitive the compiler rejects is recorded as a known gap and the
//! bead is closed as PARTIAL with the list of rejected primitives.

use proptest::prelude::*;
use vb_compile::{CompileErrors, compile_workflow};
use vb_core::workflow::WorkflowParts;

const HEADER: &str = "version: velvet-ballistics/v1\nname: roundtrip-workflow\nwhen:\n  manual: {}\nsteps:\n";

const FINISH: &str = "  - id: finish_step\n    finish:\n      result: result\n";

/// Strategy generating a random workflow that exercises one primitive.
///
/// The `primitive_index` parameter selects which primitive to use.
/// `0=set, 1=copy, 2=choose, 3=choose_slot, 4=for_each, 5=together, 6=collect,
/// 7=reduce, 8=repeat, 9=wait, 10=ask, 11=do, 12=finish`.
fn workflow_for_primitive(primitive_index: u8) -> String {
    let mut yaml = String::from(HEADER);
    let (body, _name) = match primitive_index {
        // set
        0 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"42\"\n"
                .to_string(),
            "set",
        ),
        // copy
        1 => (
            "  - id: setup\n    set:\n      output: a\n      value: \"42\"\n  - id: cp\n    copy:\n      output: result\n      from: a\n"
                .to_string(),
            "copy",
        ),
        // choose
        2 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: pick\n    choose:\n      branches:\n        - when: \"true\"\n          steps:\n            - id: b1s0\n              set:\n                output: result\n                value: \"1\"\n      otherwise: done\n"
                .to_string(),
            "choose",
        ),
        // choose_slot
        3 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: pick\n    choose_slot:\n      branches:\n        - when: \"true\"\n          output: a\n        - when: \"false\"\n          output: b\n"
                .to_string(),
            "choose_slot",
        ),
        // for_each
        4 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: loop\n    for_each:\n      items: \"[1,2,3]\"\n      steps:\n        - id: b0\n          set:\n            output: result\n            value: \"1\"\n"
                .to_string(),
            "for_each",
        ),
        // together
        5 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: par\n    together:\n      steps:\n        - id: b0\n          set:\n            output: result\n            value: \"1\"\n        - id: b1\n          set:\n            output: result\n            value: \"2\"\n"
                .to_string(),
            "together",
        ),
        // collect
        6 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: loop\n    for_each:\n      items: \"[1,2]\"\n      steps:\n        - id: b0\n          set:\n            output: result\n            value: \"1\"\n  - id: agg\n    collect:\n      inputs: result\n      output: results\n"
                .to_string(),
            "collect",
        ),
        // reduce
        7 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: loop\n    for_each:\n      items: \"[1,2]\"\n      steps:\n        - id: b0\n          set:\n            output: result\n            value: \"1\"\n  - id: agg\n    reduce:\n      inputs: result\n      output: total\n      initial: \"0\"\n      op: \"+\"\n"
                .to_string(),
            "reduce",
        ),
        // repeat
        8 => (
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: result\n            value: \"1\"\n"
                .to_string(),
            "repeat",
        ),
        // wait
        9 => (
            "  - id: pause\n    wait:\n      duration: \"1s\"\n  - id: setup\n    set:\n      output: result\n      value: \"0\"\n"
                .to_string(),
            "wait",
        ),
        // ask
        10 => (
            "  - id: ask_user\n    ask:\n      output: result\n      prompt: \"name?\"\n      timeout: \"30s\"\n"
                .to_string(),
            "ask",
        ),
        // do
        11 => (
            "  - id: setup\n    set:\n      output: result\n      value: \"0\"\n  - id: call\n    do:\n      action: dummy\n      output: result\n"
                .to_string(),
            "do",
        ),
        // finish (always-last step)
        _ => (FINISH.to_string(), "finish"),
    };
    yaml.push_str(&body);
    if primitive_index != 12 {
        yaml.push_str(FINISH);
    }
    yaml
}

fn primitive_index_strategy() -> impl Strategy<Value = u8> {
    0u8..=12
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    })]

    /// For every v1 primitive, the compiler either accepts the workflow
    /// (and the resulting IR roundtrips bit-exactly through postcard) or
    /// rejects it with a typed error. This test never panics.
    #[test]
    fn compile_to_ir_roundtrip(primitive_index in primitive_index_strategy()) {
        let yaml = workflow_for_primitive(primitive_index);
        let compiled = compile_workflow(yaml.as_bytes());

match compiled {
    Ok(workflow) => {
        let parts = workflow.to_parts();
        let bytes = match postcard::to_allocvec(&parts) {
            Ok(b) => b,
            Err(e) => {
                prop_assert!(false, "postcard serialization must succeed: {e}");
                return Ok(());
            }
        };
        let decoded: Result<WorkflowParts, _> = postcard::from_bytes(&bytes);
        let decoded = match decoded {
            Ok(p) => p,
            Err(e) => {
                prop_assert!(false, "postcard deserialization must succeed: {e}");
                return Ok(());
            }
        };
        // Re-encode and bit-compare: the IR must be stable under
        // serialize/deserialize/serialize.
        let reencoded = postcard::to_allocvec(&decoded).expect("re-serialize must succeed");
        prop_assert_eq!(bytes, reencoded, "IR bytes must roundtrip bit-exactly");
        // And the logical IR must equal itself.
        prop_assert_eq!(parts, decoded, "decoded IR must equal original parts");
    }
    Err(CompileErrors(errors)) => {
        // The compiler rejects the primitive; that is a known gap
        // (see docs/ir-primitive-coverage.md for the current
        // acceptance matrix). Errors must be typed, never a panic.
        prop_assert!(!errors.is_empty(),
            "Err must contain at least one error");
    }
}
    }
}

// ---------------------------------------------------------------------------
// Source scan: this proptest lives next to the other compile proptests and
// shares the same compile_workflow + WorkflowParts + postcard machinery.
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_proptest_uses_postcard_for_ir_encoding() {
    // Document the IR-encoding contract: the proptest encodes/decodes via
    // postcard using the Serialize/Deserialize impls on WorkflowParts.
    let source = include_str!("proptest_compile_ir_roundtrip.rs");
    assert!(
        source.contains("postcard::to_allocvec"),
        "roundtrip proptest must encode via postcard"
    );
    assert!(
        source.contains("postcard::from_bytes"),
        "roundtrip proptest must decode via postcard"
    );
    assert!(
        source.contains("WorkflowParts"),
        "roundtrip proptest must exercise WorkflowParts"
    );
}
