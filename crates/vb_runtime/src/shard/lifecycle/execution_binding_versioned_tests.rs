use super::compare_execution_tuple;
use crate::{RuntimeError, RuntimeResult};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowIrV2, node};
use vb_core::{CompiledWorkflow, ConstValue, ResourceContract, RuntimePolicy, WorkflowParts};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn set_node() -> CompiledNode {
    node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    )
}

fn finish_node() -> CompiledNode {
    node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

fn parts() -> WorkflowParts {
    WorkflowParts {
        name: "tuple".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: vec![set_node(), finish_node()].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("set"), Box::from("finish")].into_boxed_slice(),
        input_slots: Box::new([]),
    }
}

fn legacy() -> TestResult<CompiledWorkflow> {
    let mut parts = parts();
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn bound(values: &[&str]) -> TestResult<CompiledWorkflow> {
    let mut parts = parts();
    parts.symbols_count = u32::try_from(values.len())?;
    let symbols = values.iter().map(|s| Box::<str>::from(*s)).collect();
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: symbols,
    };
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(&ir)?);
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

fn tuple_fixture(workflow: &CompiledWorkflow) -> TestResult<vb_storage::AcceptedArtifact> {
    // This fixture exercises the tuple comparison, not envelope admission.
    let mut artifact =
        vb_storage::admission::prepare_compiled_artifact(&legacy()?, RuntimePolicy::Relaxed, &[])?;
    artifact.source_digest = workflow.digest();
    artifact.ir = vb_storage::encode_compiled_ir(workflow)?.into_vec();
    artifact.artifact_version = vb_storage::compat::SchemaVersion::new(2, 0, 0);
    Ok(artifact)
}

fn assert_invalid(result: RuntimeResult<()>, expected: WorkflowDigest) -> TestResult {
    match result {
        Err(RuntimeError::AdmissionArtifactInvalid { digest }) => assert_eq!(digest, expected),
        other => return Err(format!("unexpected tuple result: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn versioned_tuple_keeps_some_empty() -> TestResult {
    let workflow = bound(&[])?;
    let artifact = tuple_fixture(&workflow)?;
    assert!(artifact.ir.starts_with(&[0x01, 0xff, 0x56, 0x42]));
    compare_execution_tuple(&artifact, &workflow, &[])?;
    Ok(())
}

#[test]
fn versioned_tuple_matches_ordered_payloads() -> TestResult {
    let workflow = bound(&["key", "é", " "])?;
    compare_execution_tuple(&tuple_fixture(&workflow)?, &workflow, &[])?;
    Ok(())
}

#[test]
fn versioned_tuple_rejects_parts_projection() -> TestResult {
    let workflow = bound(&["key"])?;
    let mut artifact = tuple_fixture(&workflow)?;
    artifact.ir = postcard::to_allocvec(&workflow.to_parts())?;
    assert_invalid(
        compare_execution_tuple(&artifact, &workflow, &[]),
        workflow.digest(),
    )
}

#[test]
fn versioned_tuple_rejects_payload_reordering() -> TestResult {
    let workflow = bound(&["key", "other"])?;
    let mut artifact = tuple_fixture(&workflow)?;
    artifact.ir = vb_storage::encode_compiled_ir(&bound(&["other", "key"])?)?.into_vec();
    assert_invalid(
        compare_execution_tuple(&artifact, &workflow, &[]),
        workflow.digest(),
    )
}

#[test]
fn versioned_tuple_preserves_identity_precedence() -> TestResult {
    let workflow = bound(&["key"])?;
    let mut artifact = tuple_fixture(&workflow)?;
    let wrong = WorkflowDigest::from_bytes([99; 32]);
    artifact.source_digest = wrong;
    artifact.ir.clear();
    match compare_execution_tuple(&artifact, &workflow, &[]) {
        Err(RuntimeError::AdmissionArtifactDigestMismatch { requested, found }) => {
            assert_eq!(requested, workflow.digest());
            assert_eq!(found, wrong);
        }
        other => return Err(format!("unexpected identity result: {other:?}").into()),
    }
    Ok(())
}
