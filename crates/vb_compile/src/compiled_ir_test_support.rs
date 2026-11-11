use crate::artifact::{ArtifactVerificationError, CompilationBinding};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, VersionedWorkflowParts, WorkflowIrV2, node,
};
use vb_core::{CompiledWorkflow, ConstValue, ResourceContract, WorkflowParts};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: codec\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: \"$seed\"\n";

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

pub(super) fn parts(count: u32) -> WorkflowParts {
    WorkflowParts {
        name: "encode-test".into(),
        digest: WorkflowDigest::from_bytes([7; 32]),
        semantic_source_identity: Some(WorkflowDigest::from_bytes([9; 32])),
        nodes: vec![set_node(), finish_node()].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("set"), Box::from("finish")].into_boxed_slice(),
        input_slots: Box::new([]),
    }
}

pub(super) fn symbols(values: &[&str]) -> Box<[Box<str>]> {
    values
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect()
}

pub(super) fn ir(values: &[&str]) -> TestResult<WorkflowIrV2> {
    Ok(WorkflowIrV2 {
        wire_version: 2,
        parts: parts(u32::try_from(values.len())?),
        symbol_payloads: symbols(values),
    })
}

pub(super) fn v2_bytes(ir: &WorkflowIrV2) -> Result<Vec<u8>, postcard::Error> {
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(ir)?);
    Ok(bytes)
}

pub(super) fn stamped(mut ir: WorkflowIrV2) -> TestResult<CompiledWorkflow> {
    ir.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&v2_bytes(&ir)?).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(super) fn assert_changed_digest(original: WorkflowIrV2, altered: WorkflowIrV2) -> TestResult {
    let mut a = original.clone();
    let mut b = altered.clone();
    a.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    b.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let expected_a = WorkflowDigest::from_bytes(blake3::hash(&v2_bytes(&a)?).into());
    let expected_b = WorkflowDigest::from_bytes(blake3::hash(&v2_bytes(&b)?).into());
    assert_eq!(
        crate::compute_compiled_ir_digest(&VersionedWorkflowParts::V2(original))?,
        expected_a
    );
    assert_eq!(
        crate::compute_compiled_ir_digest(&VersionedWorkflowParts::V2(altered))?,
        expected_b
    );
    assert_ne!(expected_a, expected_b);
    Ok(())
}

pub(super) fn assert_stale_digest(mut ir: WorkflowIrV2, claimed: WorkflowDigest) -> TestResult {
    ir.parts.digest = claimed;
    let mut independent = ir.clone();
    independent.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let computed = WorkflowDigest::from_bytes(blake3::hash(&v2_bytes(&independent)?).into());
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(ir.parts, ir.symbol_payloads)?;
    match crate::verify_compiled_artifact(&workflow, &[]) {
        Err(ArtifactVerificationError::DigestMismatch {
            binding,
            claimed: actual_claimed,
            computed: actual_computed,
        }) => {
            assert_eq!(binding, CompilationBinding::Workflow);
            assert_eq!(actual_claimed, claimed);
            assert_eq!(actual_computed, computed);
            assert_ne!(claimed, computed);
        }
        other => return Err(format!("unexpected result: {other:?}").into()),
    }
    Ok(())
}
