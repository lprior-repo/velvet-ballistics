use super::*;
use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowIrV2, WorkflowParts, node};
use vb_core::{CompiledNode, CompiledNodeKind, ConstIdx, SlotIdx, StepIdx, SymbolId};

pub(super) fn nodes() -> Box<[CompiledNode]> {
    Box::from([
        node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        ),
    ])
}

pub(super) fn parts(count: u32) -> WorkflowParts {
    WorkflowParts {
        name: "versioned-admission".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        input_slots: Box::new([]),
        nodes: nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(7)]),
        slot_count: 1,
        symbols_count: count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub(super) fn legacy(count: u32) -> TestResult<CompiledWorkflow> {
    let mut value = parts(count);
    if count != 0 {
        value.constants = Box::from([ConstValue::Symbol(SymbolId::new(0))]);
    }
    value.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&value)?).into());
    Ok(CompiledWorkflow::try_from_parts(value)?)
}

pub(super) fn v2_wire(ir: &WorkflowIrV2) -> TestResult<Vec<u8>> {
    let mut wire = vec![0x01, 0xff, 0x56, 0x42];
    wire.extend_from_slice(&postcard::to_allocvec(ir)?);
    Ok(wire)
}

pub(super) fn bound_ir(values: &[&str]) -> TestResult<WorkflowIrV2> {
    let mut value = parts(u32::try_from(values.len())?);
    if !values.is_empty() {
        value.constants = Box::from([ConstValue::Symbol(SymbolId::new(0))]);
    }
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts: value,
        symbol_payloads: values
            .iter()
            .map(|value| Box::<str>::from(*value))
            .collect(),
    };
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&v2_wire(&ir)?).into());
    Ok(ir)
}

pub(super) fn bound(values: &[&str]) -> TestResult<CompiledWorkflow> {
    let ir = bound_ir(values)?;
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(super) fn artifact(workflow: &CompiledWorkflow) -> TestResult<AcceptedArtifact> {
    Ok(prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Strict,
        &[],
    )?)
}

pub(super) fn replace_ir(artifact: &mut AcceptedArtifact, ir: Vec<u8>) -> TestResult {
    artifact.ir = ir;
    bind_artifact_digest(artifact)?;
    Ok(())
}

pub(super) fn read(journal: &crate::FjallJournal, expected: &AcceptedArtifact) -> TestResult {
    let record = journal
        .compiled_ir(expected.digest)?
        .ok_or("missing artifact")?;
    let actual: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    assert_eq!(record.digest, expected.digest);
    assert_eq!(&actual, expected);
    Ok(())
}

pub(super) fn assert_error(
    artifact: &AcceptedArtifact,
    check: impl FnOnce(JournalError) -> TestResult,
) -> TestResult {
    match verify_accepted_artifact(artifact) {
        Err(error) => check(error),
        Ok(value) => Err(format!("unexpected accepted workflow: {value:?}").into()),
    }
}

pub(super) fn malformed(error: JournalError) -> TestResult {
    match error {
        JournalError::ArtifactMalformed => Ok(()),
        other => Err(format!("expected ArtifactMalformed, found {other:?}").into()),
    }
}

pub(super) fn checksum(error: JournalError) -> TestResult {
    match error {
        JournalError::ArtifactChecksumMismatch => Ok(()),
        other => Err(format!("expected ArtifactChecksumMismatch, found {other:?}").into()),
    }
}
