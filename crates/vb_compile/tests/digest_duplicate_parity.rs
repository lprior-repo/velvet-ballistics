// Duplicate implementation parity tests (PO-UT-003, REPAIR-2).
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-006: The active canonical_digest (part_05.rs, public via
// vb_compile) and the legacy/dead-code canonical_digest (compile/mod.rs, not
// mounted in lib.rs) produce identical WorkflowDigest values for the same
// WorkflowSource input.
//
// The dead-code functions are replicated locally so the integration test can
// compile and run without depending on the unmounted compile/mod.rs module.

#![forbid(unsafe_code)]

mod common;
use vb_compile::canonical_digest as public_canonical_digest;
use vb_core::WorkflowDigest;

// ── Local mirror of compile/mod.rs: canonical_digest + helpers ──
//
// These are an exact replica of the functions at:
//   crates/vb_compile/src/compile/mod.rs, lines 203–274
//
// The only structural adaptation is the `_ => "unknown"` catch-all in
// canonical_primitive_name and digest_step_primitive, because vb_yaml's
// StepPrimitive is #[non_exhaustive] and may gain new variants.
//
// For the parity test scenarios (Set, Finish, Ask with Manual trigger) the
// catch-all is never reached, so the exhaustive-match vs catch-all difference
// is not observable here.

fn private_canonical_primitive_name(primitive: &vb_compile::StepPrimitive) -> &'static str {
    match primitive {
        vb_compile::StepPrimitive::Set { .. } => "set",
        vb_compile::StepPrimitive::Save { .. } => "save",
        vb_compile::StepPrimitive::Do { .. } => "do",
        vb_compile::StepPrimitive::Choose { .. } => "choose",
        vb_compile::StepPrimitive::ForEach { .. } => "for_each",
        vb_compile::StepPrimitive::Together { .. } => "parallel",
        vb_compile::StepPrimitive::Collect { .. } => "collect",
        vb_compile::StepPrimitive::Aggregate { .. } => "aggregate",
        vb_compile::StepPrimitive::Repeat { .. } => "repeat",
        vb_compile::StepPrimitive::Wait { .. } => "wait",
        vb_compile::StepPrimitive::Ask { .. } => "ask",
        vb_compile::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}

fn private_digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &vb_compile::StepPrimitive,
) {
    match primitive {
        vb_compile::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_compile::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_compile::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_compile::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
                _ => hasher.update(b"unsupported"),
            };
        }
        vb_compile::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            hasher.update(prompt.as_bytes());
            match timeout {
                Some(t) => {
                    hasher.update(b"timeout");
                    hasher.update(t.as_bytes());
                }
                None => {
                    hasher.update(b"no_timeout");
                }
            }
        }
        other => {
            hasher.update(private_canonical_primitive_name(other).as_bytes());
        }
    }
}

fn private_canonical_digest(source: &vb_compile::WorkflowSource) -> WorkflowDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.version().as_bytes());
    hasher.update(source.name().as_bytes());
    match source.trigger() {
        vb_compile::TriggerAst::Manual => hasher.update(b"manual"),
        vb_compile::TriggerAst::Schedule { cron } => {
            hasher.update(b"schedule");
            hasher.update(cron.as_bytes())
        }
        vb_compile::TriggerAst::Event { event_type } => {
            hasher.update(b"event");
            hasher.update(event_type.as_bytes())
        }
        vb_compile::TriggerAst::Webhook => hasher.update(b"webhook"),
        _ => hasher.update(b"unknown"),
    };
    for step in source.steps() {
        hasher.update(step.id.as_bytes());
        private_digest_step_primitive(&mut hasher, &step.primitive);
    }
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

// ── PO-UT-003 parity tests ──

/// PO-UT-003 T1: Ask workflows — both implementations produce identical digests
/// when the Ask step includes a Some timeout.
#[test]
fn ask_prompt_some_timeout_parity() {
    let source = common::ask_source("test prompt", Some("30s"));
    let digest_public = public_canonical_digest(&source).expect("valid test input");
    let digest_private = private_canonical_digest(&source);
    assert_eq!(
        digest_public, digest_private,
        "PO-UT-003: public and private canonical_digest diverge for Ask(Some timeout)"
    );
}

/// PO-UT-003 T2: Ask workflow with None timeout — parity.
#[test]
fn ask_prompt_none_timeout_parity() {
    let source = common::ask_source("another prompt", None);
    let digest_public = public_canonical_digest(&source).expect("valid test input");
    let digest_private = private_canonical_digest(&source);
    assert_eq!(
        digest_public, digest_private,
        "PO-UT-003: public and private canonical_digest diverge for Ask(None timeout)"
    );
}

/// PO-UT-003 T3: Ask workflow with empty prompt — parity.
#[test]
fn ask_empty_prompt_parity() {
    let source = common::ask_source("", None);
    let digest_public = public_canonical_digest(&source).expect("valid test input");
    let digest_private = private_canonical_digest(&source);
    assert_eq!(
        digest_public, digest_private,
        "PO-UT-003: public and private canonical_digest diverge for Ask(empty prompt)"
    );
}

/// PO-UT-003 T4: Set+Finish workflow — parity (exercises the reachable path
/// through compile_source, which calls private canonical_digest).
#[test]
fn set_finish_parity() {
    let source = common::set_finish_source();
    let digest_public = public_canonical_digest(&source).expect("valid test input");
    let digest_private = private_canonical_digest(&source);
    assert_eq!(
        digest_public, digest_private,
        "PO-UT-003: public and private canonical_digest diverge for Set+Finish"
    );
}
