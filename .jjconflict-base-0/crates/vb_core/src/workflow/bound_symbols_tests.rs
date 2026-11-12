use super::{CompiledWorkflow, VersionedWorkflowParts, WorkflowError, WorkflowIrV2, WorkflowParts};
use crate::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::source_metadata::SourceMetadataError;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract,
};

fn payloads(values: &[&str]) -> Box<[Box<str>]> {
    values
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect()
}

fn value_node() -> CompiledNode {
    crate::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    )
}

fn finish_node(step: u16, result: u16) -> CompiledNode {
    crate::workflow::node(
        StepIdx::new(step),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
        None,
        None,
    )
}

fn parts(symbols_count: u32) -> WorkflowParts {
    WorkflowParts {
        name: "bound-fixture".into(),
        digest: WorkflowDigest::from_bytes([7; 32]),
        semantic_source_identity: None,
        nodes: vec![value_node(), finish_node(1, 0)].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(7)].into_boxed_slice(),
        slot_count: 1,
        symbols_count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("set"), Box::from("finish")].into_boxed_slice(),
        input_slots: Box::new([]),
    }
}

#[test]
fn bound_symbols_legacy_preserves_parts_and_wire() -> Result<(), Box<dyn std::error::Error>> {
    let original = parts(0);
    let bytes = postcard::to_allocvec(&original)?;
    let workflow = CompiledWorkflow::try_from_parts(original.clone())?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    assert_eq!(postcard::to_allocvec(&workflow.to_parts())?, bytes);
    Ok(())
}

#[test]
fn bound_symbols_legacy_nonzero_stays_unbound() -> Result<(), WorkflowError> {
    let mut original = parts(1);
    original.constants = vec![ConstValue::Symbol(SymbolId::new(0))].into_boxed_slice();
    let workflow = CompiledWorkflow::try_from_parts(original.clone())?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

#[test]
fn bound_symbols_empty_table_is_explicit_v2() -> Result<(), WorkflowError> {
    let original = parts(0);
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(original.clone(), Box::new([]))?;
    assert_eq!(workflow.symbol_payloads(), Some(payloads(&[]).as_ref()));
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(WorkflowIrV2 {
            wire_version: 2,
            parts: original,
            symbol_payloads: Box::new([]),
        })
    );
    Ok(())
}

#[test]
fn bound_symbols_projection_retains_order_and_payloads() -> Result<(), WorkflowError> {
    let mut original = parts(3);
    original.constants = vec![ConstValue::Symbol(SymbolId::new(1))].into_boxed_slice();
    let workflow =
        CompiledWorkflow::try_from_parts_with_symbols(original.clone(), payloads(&["z", "é", ""]))?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(payloads(&["z", "é", ""]).as_ref())
    );
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(WorkflowIrV2 {
            wire_version: 2,
            parts: original,
            symbol_payloads: payloads(&["z", "é", ""]),
        })
    );
    assert_eq!(
        workflow.clone().symbol_payloads(),
        workflow.symbol_payloads()
    );
    Ok(())
}

#[test]
fn bound_symbols_count_mismatch_is_typed() {
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(parts(2), payloads(&["key"])),
        Err(WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::SymbolCountMismatch {
                declared: 2,
                actual: 1,
            }
        })
    );
}

#[test]
fn bound_symbols_duplicate_payload_is_typed() {
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(parts(2), payloads(&["key", "key"])),
        Err(WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::DuplicateSymbol {
                first: SymbolId::new(0),
                duplicate: SymbolId::new(1),
            }
        })
    );
}

#[test]
fn bound_symbols_invalid_constant_reference_is_typed() {
    let mut original = parts(1);
    original.constants = vec![ConstValue::Symbol(SymbolId::new(1))].into_boxed_slice();
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, payloads(&["key"])),
        Err(WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1)
        })
    );
}

#[test]
fn bound_symbols_invalid_accessor_reference_is_typed() -> Result<(), WorkflowError> {
    let mut original = parts(1);
    original.slot_count = 1;
    original.accessors = vec![AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice(),
    }]
    .into_boxed_slice();
    CompiledWorkflow::try_from_parts_with_symbols(original.clone(), payloads(&["key"]))?;
    original.accessors = vec![AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice(),
    }]
    .into_boxed_slice();
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, payloads(&["key"])),
        Err(WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1)
        })
    );
    Ok(())
}

fn object_node(key: SymbolId) -> CompiledNode {
    crate::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::BuildObject {
            fields: vec![(key, SlotIdx::new(0))].into_boxed_slice(),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    )
}

fn object_parts(key: SymbolId) -> WorkflowParts {
    let mut original = parts(1);
    original.slot_count = 2;
    original.nodes = vec![value_node(), object_node(key), finish_node(2, 1)].into_boxed_slice();
    original.step_names =
        vec![Box::from("set"), Box::from("build"), Box::from("finish")].into_boxed_slice();
    original
}

#[test]
fn bound_symbols_invalid_object_reference_is_typed() -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts_with_symbols(
        object_parts(SymbolId::new(0)),
        payloads(&["key"]),
    )?;
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(
            object_parts(SymbolId::new(1)),
            payloads(&["key"])
        ),
        Err(WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1)
        })
    );
    Ok(())
}

#[test]
fn bound_symbols_oversized_payload_is_typed() {
    let symbol = "x".repeat(4_097).into_boxed_str();
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(parts(1), vec![symbol].into_boxed_slice()),
        Err(WorkflowError::InvalidSymbolMetadata {
            source: SourceMetadataError::SymbolTooLong {
                id: SymbolId::new(0),
                bytes: 4_097,
                max: 4_096,
            }
        })
    );
}

#[test]
fn bound_symbols_exact_byte_limit_is_accepted() -> Result<(), WorkflowError> {
    let symbol = "é".repeat(2_048).into_boxed_str();
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(
        parts(1),
        vec![symbol.clone()].into_boxed_slice(),
    )?;
    assert_eq!(workflow.symbol_payloads(), Some(vec![symbol].as_slice()));
    Ok(())
}

#[test]
fn bound_symbols_preserves_existing_parts_validation() {
    let mut original = parts(0);
    original.nodes = Box::new([]);
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, Box::new([])),
        Err(WorkflowError::EmptyNodes)
    );
}

#[cfg(feature = "test-util")]
#[test]
fn bound_symbols_unchecked_legacy_constructor_stays_unbound() {
    let workflow = CompiledWorkflow::from_parts_unchecked(parts(0));
    assert_eq!(workflow.symbol_payloads(), None);
    assert!(matches!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(_)
    ));
}
