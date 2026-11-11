use crate::artifact::verify_compiled_artifact;
use crate::{compute_compiled_ir_digest, encode_compiled_ir};
use vb_core::workflow::VersionedWorkflowParts;
use vb_core::{CompiledWorkflow, WorkflowDigest};

#[path = "compiled_ir_test_support.rs"]
mod support;
use support::{TestResult, ir, parts, stamped, v2_bytes};

#[test]
fn v1_encodes_as_raw_parts_bytes() -> TestResult {
    let original = parts(0);
    let expected = postcard::to_allocvec(&original)?;
    let workflow = CompiledWorkflow::try_from_parts(original)?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(encode_compiled_ir(&workflow)?.as_ref(), expected.as_slice());
    assert_eq!(
        crate::emit_compiled_artifact(&workflow)?.as_ref(),
        expected.as_slice()
    );
    Ok(())
}

#[test]
fn some_empty_payloads_yields_v2() -> TestResult {
    let workflow = stamped(ir(&[])?)?;
    let mut original = ir(&[])?;
    original.parts.digest = workflow.digest();
    let expected = v2_bytes(&original)?;
    assert_eq!(workflow.symbol_payloads(), Some([].as_slice()));
    assert_eq!(encode_compiled_ir(&workflow)?.as_ref(), expected.as_slice());
    assert_eq!(
        verify_compiled_artifact(&workflow, &[])?
            .workflow()
            .symbol_payloads(),
        Some([].as_slice())
    );
    Ok(())
}

#[test]
fn valid_nonzero_legacy_bytes_remain_raw() -> TestResult {
    let mut original = parts(1);
    original.constants = Box::new([vb_core::ConstValue::Symbol(vb_core::SymbolId::new(0))]);
    original.digest = WorkflowDigest::from_bytes([0; 32]);
    original.digest =
        WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&original)?).into());
    let expected = postcard::to_allocvec(&original)?;
    let workflow = CompiledWorkflow::try_from_parts(original)?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(encode_compiled_ir(&workflow)?.as_ref(), expected.as_slice());
    assert_eq!(
        verify_compiled_artifact(&workflow, &[])?.payload(),
        expected.as_slice()
    );
    Ok(())
}

#[test]
fn v2_prefix_literal() -> TestResult {
    let original = ir(&["key"])?;
    let workflow =
        CompiledWorkflow::try_from_parts_with_symbols(original.parts, original.symbol_payloads)?;
    let bytes = encode_compiled_ir(&workflow)?;
    assert_eq!(bytes.get(..4), Some([0x01, 0xff, 0x56, 0x42].as_slice()));
    Ok(())
}

#[test]
fn ordered_symbol_payloads() -> TestResult {
    let workflow = stamped(ir(&["key", "é", " "])?)?;
    let mut expected = ir(&["key", "é", " "])?;
    expected.parts.digest = workflow.digest();
    assert_eq!(
        encode_compiled_ir(&workflow)?.as_ref(),
        v2_bytes(&expected)?.as_slice()
    );
    let verified = verify_compiled_artifact(&workflow, &[])?;
    assert_eq!(
        verified.workflow().symbol_payloads(),
        workflow.symbol_payloads()
    );
    assert_eq!(verified.payload(), v2_bytes(&expected)?.as_slice());
    Ok(())
}

#[test]
fn canonical_digest_v1_matches_independent_preimage() -> TestResult {
    let original = parts(0);
    let mut independent = original.clone();
    independent.digest = WorkflowDigest::from_bytes([0; 32]);
    let expected =
        WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&independent)?).into());
    assert_eq!(
        compute_compiled_ir_digest(&VersionedWorkflowParts::V1(original))?,
        expected
    );
    Ok(())
}

#[test]
fn canonical_digest_v2_matches_independent_preimage() -> TestResult {
    let original = ir(&["hello"])?;
    let mut independent = original.clone();
    independent.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let expected = WorkflowDigest::from_bytes(blake3::hash(&v2_bytes(&independent)?).into());
    assert_eq!(
        compute_compiled_ir_digest(&VersionedWorkflowParts::V2(original))?,
        expected
    );
    Ok(())
}

#[test]
fn same_length_payload_mutation_changes_digest() -> TestResult {
    let original = ir(&["hello", "there"])?;
    let mut altered = original.clone();
    altered.symbol_payloads = support::symbols(&["world", "there"]);
    assert_eq!(original.parts, altered.parts);
    support::assert_changed_digest(original, altered)
}

#[test]
fn payload_reordering_changes_digest() -> TestResult {
    let original = ir(&["hello", "there"])?;
    let mut altered = original.clone();
    altered.symbol_payloads = support::symbols(&["there", "hello"]);
    assert_eq!(original.parts, altered.parts);
    support::assert_changed_digest(original, altered)
}

#[test]
fn stale_digest_rejected_by_verify_compiled_artifact() -> TestResult {
    let workflow = stamped(ir(&["hello", "there"])?)?;
    support::assert_stale_digest(ir(&["world", "there"])?, workflow.digest())
}

#[test]
fn reordered_stale_digest_rejected() -> TestResult {
    let workflow = stamped(ir(&["hello", "there"])?)?;
    support::assert_stale_digest(ir(&["there", "hello"])?, workflow.digest())
}

#[test]
fn ordinary_source_compilation_stays_legacy_v1() -> TestResult {
    let workflow = crate::compile_workflow(support::SOURCE)?;
    assert_eq!(workflow.symbol_payloads(), None);
    let expected = postcard::to_allocvec(&workflow.to_parts())?;
    assert_eq!(
        crate::emit_compiled_artifact(&workflow)?.as_ref(),
        expected.as_slice()
    );
    let mut independent = workflow.to_parts();
    independent.digest = WorkflowDigest::from_bytes([0; 32]);
    assert_eq!(
        workflow.digest(),
        WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&independent)?).into())
    );
    Ok(())
}
