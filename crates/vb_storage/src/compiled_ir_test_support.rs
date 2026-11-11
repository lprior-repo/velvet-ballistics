use super::*;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn unexpected(value: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("unexpected codec result: {value:?}").into()
}

pub(super) fn value_node() -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    )
}

pub(super) fn finish_node(step: u16, slot: u16) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(step),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(slot),
        },
        None,
        None,
    )
}

pub(super) fn parts(symbols_count: u32) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("codec-fixture"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: Box::from([value_node(), finish_node(1, 0)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(7)]),
        slot_count: 1,
        symbols_count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([Box::from("set"), Box::from("finish")]),
        input_slots: Box::new([]),
    }
}

pub(super) fn symbol_parts(count: u32) -> WorkflowParts {
    let mut value = parts(count);
    value.constants = Box::from([ConstValue::Symbol(SymbolId::new(0))]);
    value
}

pub(super) fn payloads(values: &[&str]) -> Box<[Box<str>]> {
    values
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect()
}

pub(super) fn stamp_v1(mut value: WorkflowParts) -> TestResult<WorkflowParts> {
    value.digest = WorkflowDigest::from_bytes([0; 32]);
    value.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&value)?).into());
    Ok(value)
}

pub(super) fn wire_v2(ir: &WorkflowIrV2) -> TestResult<Vec<u8>> {
    let mut wire = vec![0x01, 0xff, 0x56, 0x42];
    wire.extend_from_slice(&postcard::to_allocvec(ir)?);
    Ok(wire)
}

pub(super) fn stamp_v2(mut ir: WorkflowIrV2) -> TestResult<WorkflowIrV2> {
    ir.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&wire_v2(&ir)?).into());
    Ok(ir)
}

pub(super) fn bound_ir(values: &[&str]) -> TestResult<WorkflowIrV2> {
    let count = u32::try_from(values.len())?;
    let value = if values.is_empty() {
        parts(count)
    } else {
        symbol_parts(count)
    };
    stamp_v2(WorkflowIrV2 {
        wire_version: 2,
        parts: value,
        symbol_payloads: payloads(values),
    })
}

pub(super) fn legacy(count: u32) -> TestResult<CompiledWorkflow> {
    let value = if count == 0 {
        parts(count)
    } else {
        symbol_parts(count)
    };
    Ok(CompiledWorkflow::try_from_parts(stamp_v1(value)?)?)
}

pub(super) fn assert_v1(count: u32, version: Option<SchemaVersion>) -> TestResult {
    let workflow = legacy(count)?;
    let original = workflow.to_parts();
    let expected = postcard::to_allocvec(&original)?;
    assert_eq!(encode_compiled_ir(&workflow)?.as_ref(), expected.as_slice());
    let decoded = decode_compiled_ir(&expected, version)?;
    assert_eq!(decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), None);
    assert_eq!(
        decoded.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

pub(super) fn assert_v2(values: &[&str], version: Option<SchemaVersion>) -> TestResult {
    let ir = bound_ir(values)?;
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts.clone(),
        ir.symbol_payloads.clone(),
    )?;
    let expected = wire_v2(&ir)?;
    assert_eq!(encode_compiled_ir(&workflow)?.as_ref(), expected.as_slice());
    let decoded = decode_compiled_ir(&expected, version)?;
    assert_eq!(decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), Some(payloads(values).as_ref()));
    assert_eq!(decoded.to_versioned_parts(), VersionedWorkflowParts::V2(ir));
    Ok(())
}

pub(super) fn assert_malformed<T: std::fmt::Debug>(result: Result<T, JournalError>) -> TestResult {
    match result {
        Err(JournalError::ArtifactMalformed) => Ok(()),
        other => Err(unexpected(other)),
    }
}

pub(super) fn assert_checksum<T: std::fmt::Debug>(result: Result<T, JournalError>) -> TestResult {
    match result {
        Err(JournalError::ArtifactChecksumMismatch) => Ok(()),
        other => Err(unexpected(other)),
    }
}

pub(super) fn assert_reconstruction(bytes: &[u8], expected: WorkflowError) -> TestResult {
    match decode_compiled_ir(bytes, None) {
        Err(JournalError::WorkflowReconstruction(actual)) => {
            assert_eq!(actual, expected);
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}

pub(super) fn assert_too_large<T: std::fmt::Debug>(
    result: Result<T, JournalError>,
    expected: u32,
) -> TestResult {
    match result {
        Err(JournalError::PayloadTooLarge { len, max }) => {
            assert_eq!(len, expected);
            assert_eq!(max, MAX_COMPILED_IR_BYTES);
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}

pub(super) fn assert_eof(bytes: &[u8]) -> TestResult {
    match decode_compiled_ir(bytes, None) {
        Err(JournalError::PostcardDecodeFailed(postcard::Error::DeserializeUnexpectedEnd)) => {
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}
