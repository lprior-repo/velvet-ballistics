use super::*;

#[test]
fn symbol_count_mismatch_retains_typed_source_before_digest() -> TestResult {
    let mut ir = bound_ir(&["key"])?;
    ir.parts.symbols_count = 2;
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::SymbolCountMismatch {
                declared: 2,
                actual: 1,
            },
        },
    )
}

#[test]
fn duplicate_payload_retains_typed_source() -> TestResult {
    let ir = bound_ir(&["key", "key"])?;
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::DuplicateSymbol {
                first: SymbolId::new(0),
                duplicate: SymbolId::new(1),
            },
        },
    )
}

#[test]
fn invalid_constant_symbol_retains_typed_source() -> TestResult {
    let mut ir = bound_ir(&["key"])?;
    ir.parts.constants = Box::from([ConstValue::Symbol(SymbolId::new(1))]);
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1),
        },
    )
}

#[test]
fn invalid_accessor_symbol_retains_typed_source() -> TestResult {
    let mut ir = bound_ir(&["key"])?;
    ir.parts.accessors = Box::from([AccessorProgram {
        root: SlotIdx::ZERO,
        path: Box::from([PathSegment::Field(SymbolId::new(1))]),
    }]);
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1),
        },
    )
}

fn object_node(key: SymbolId) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::BuildObject {
            fields: Box::from([(key, SlotIdx::ZERO)]),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    )
}

fn object_ir(key: SymbolId) -> TestResult<WorkflowIrV2> {
    let mut ir = bound_ir(&["key"])?;
    ir.parts.slot_count = 2;
    ir.parts.nodes = Box::from([value_node(), object_node(key), finish_node(2, 1)]);
    ir.parts.step_names = Box::from([Box::from("set"), Box::from("object"), Box::from("finish")]);
    stamp_v2(ir)
}

#[test]
fn invalid_object_symbol_retains_typed_source() -> TestResult {
    let valid = object_ir(SymbolId::new(0))?;
    CompiledWorkflow::try_from_parts_with_symbols(valid.parts, valid.symbol_payloads)?;
    assert_reconstruction(
        &wire_v2(&object_ir(SymbolId::new(1))?)?,
        WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1),
        },
    )
}

#[test]
fn oversized_symbol_retains_typed_source() -> TestResult {
    let max = vb_core::limits::MAX_SYMBOL_BYTES_PER_VALUE;
    let bytes = max.checked_add(1).ok_or("symbol length overflow")?;
    let value = "x".repeat(bytes);
    let ir = bound_ir(&[&value])?;
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::SymbolTooLong {
                id: SymbolId::new(0),
                bytes,
                max,
            },
        },
    )
}

#[test]
fn excessive_symbol_count_retains_typed_source() -> TestResult {
    let max = vb_core::limits::MAX_VALUES_PER_RUN;
    let count = max.checked_add(1).ok_or("symbol count overflow")?;
    let ir = WorkflowIrV2 {
        wire_version: 2,
        parts: symbol_parts(u32::try_from(count)?),
        symbol_payloads: vec![Box::<str>::from(""); count].into_boxed_slice(),
    };
    assert_reconstruction(
        &wire_v2(&ir)?,
        WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::TooManySymbols { count, max },
        },
    )
}
