//! Cross-crate codec wire equivalence: vb_compile and vb_storage must emit
//! identical payload bytes for equivalent CompiledWorkflow inputs.
//!
//! Test matrix:
//! - legacy_none_0: V1 zero-symbol legacy, no outer version
//! - legacy_none_1: V1 single-symbol legacy, no outer version
//! - v2_some_empty: V2 explicitly bound empty payload, no outer version
//! - v2_ordered: V2 multi-symbol ordered payload, no outer version
//! - sc_t1_bound_payload_mismatch: correct encode + wrong-payload checksum error

#![forbid(unsafe_code)]

use vb_compile::{emit_compiled_artifact, encode_compiled_ir as compile_encode};
use vb_core::workflow::VersionedWorkflowParts;
use vb_storage::{
    compat::SchemaVersion, decode_compiled_ir as storage_decode,
    encode_compiled_ir as storage_encode,
};

#[path = "vb_versioned_ir_codec_equivalence/fixture.rs"]
mod fixture;
use fixture::{bound, checked_bound, checked_legacy, independent_wire, legacy, stamp};

fn assert_encode_triple(
    workflow: &vb_core::workflow::CompiledWorkflow,
    expected: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_box = expected.into_boxed_slice();
    let compile_bytes = compile_encode(workflow)?;
    let emit_bytes = emit_compiled_artifact(workflow)?;
    let storage_bytes = storage_encode(workflow)?;

    assert_eq!(
        compile_bytes, expected_box,
        "vb_compile::encode_compiled_ir"
    );
    assert_eq!(
        emit_bytes, expected_box,
        "vb_compile::emit_compiled_artifact"
    );
    assert_eq!(
        storage_bytes, expected_box,
        "vb_storage::encode_compiled_ir"
    );

    assert_decode(&compile_bytes, workflow)?;
    assert_decode(&emit_bytes, workflow)?;
    assert_decode(&storage_bytes, workflow)
}

fn assert_decode(
    bytes: &[u8],
    workflow: &vb_core::workflow::CompiledWorkflow,
) -> Result<(), Box<dyn std::error::Error>> {
    let decoded = storage_decode(bytes, None)?;
    assert_eq!(&decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), workflow.symbol_payloads());
    let version = match workflow.to_versioned_parts() {
        VersionedWorkflowParts::V1(_) => SchemaVersion::new(1, 2, 0),
        VersionedWorkflowParts::V2(_) => SchemaVersion::new(2, 1, 0),
    };
    let decoded_versioned = storage_decode(bytes, Some(version))?;
    assert_eq!(
        decoded_versioned, *workflow,
        "decoded workflow equality (versioned)"
    );
    Ok(())
}

fn test_equivalence(versioned: VersionedWorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let stamped = stamp(versioned)?;
    let expected = independent_wire(&stamped)?;
    let workflow = match &stamped {
        VersionedWorkflowParts::V1(parts) => checked_legacy(parts.clone())?,
        VersionedWorkflowParts::V2(ir) => checked_bound(&VersionedWorkflowParts::V2(ir.clone()))?,
    };

    assert_encode_triple(&workflow, expected)
}

#[test]
fn legacy_none_0() -> Result<(), Box<dyn std::error::Error>> {
    let versioned = legacy(0)?;
    test_equivalence(versioned)?;
    Ok(())
}

#[test]
fn legacy_none_1() -> Result<(), Box<dyn std::error::Error>> {
    let versioned = legacy(1)?;
    test_equivalence(versioned)?;
    Ok(())
}

#[test]
fn v2_some_empty() -> Result<(), Box<dyn std::error::Error>> {
    let versioned = bound(&[])?;
    test_equivalence(versioned)?;
    Ok(())
}

#[test]
fn v2_ordered() -> Result<(), Box<dyn std::error::Error>> {
    let values = ["key", "é", " "];
    let versioned = bound(&values)?;
    test_equivalence(versioned)?;
    Ok(())
}

#[test]
fn sc_t1_bound_payload_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let values = ["key"];
    let versioned = bound(&values)?;
    let stamped = stamp(versioned)?;
    let expected = independent_wire(&stamped)?;
    let workflow = checked_bound(&stamped)?;
    assert_encode_triple(&workflow, expected)?;

    let wrong_workflow = altered_key_workflow(&workflow)?;
    assert_eq!(wrong_workflow.to_parts(), workflow.to_parts());
    match storage_encode(&wrong_workflow) {
        Err(vb_storage::JournalError::ArtifactChecksumMismatch) => Ok(()),
        other => Err(format!("expected checksum mismatch, got {other:?}").into()),
    }
}

fn altered_key_workflow(
    original: &vb_core::workflow::CompiledWorkflow,
) -> Result<vb_core::workflow::CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(
        vb_core::workflow::CompiledWorkflow::try_from_parts_with_symbols(
            original.to_parts(),
            Box::from([Box::from("Key")]),
        )?,
    )
}
