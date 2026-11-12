use super::{CompiledWorkflow, VersionedWorkflowParts, WorkflowError, parts, payloads};
use crate::ids::{SlotIdx, SymbolId, WorkflowDigest};
use crate::workflow::source_metadata::{
    CompiledSecretRequirement, SourceMetadataError, WorkflowSourceMetadata,
};
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, InputDefaultValue, InputSlotKind, WorkflowIrV2,
    WorkflowParts,
};

fn secret_metadata(
    symbols: &[&str],
    requirements: &[CompiledSecretRequirement],
) -> Result<WorkflowSourceMetadata, SourceMetadataError> {
    WorkflowSourceMetadata::try_new(payloads(symbols), Box::from(requirements))
}

fn secret_input(slot: SlotIdx) -> CompiledInputSlot {
    CompiledInputSlot {
        slot,
        kind: InputSlotKind::Symbol,
        constraint: CompiledInputConstraint {
            secretness: true.into(),
            default_cold: Some(InputDefaultValue::Text("REQ_A".into())),
            ..CompiledInputConstraint::required()
        },
    }
}

fn secret_parts() -> Result<WorkflowParts, SourceMetadataError> {
    let mut original = parts(2);
    original.slot_count = 2;
    original.input_slots = Box::new([secret_input(SlotIdx::new(1))]);
    let requirement =
        CompiledSecretRequirement::with_slot(SymbolId::new(0), SymbolId::new(1), SlotIdx::new(1));
    original.source_metadata = Some(Box::new(secret_metadata(
        &["alias", "REQ_A"],
        &[requirement],
    )?));
    Ok(original)
}

#[test]
fn metadata_mismatch_payloads_are_typed() -> Result<(), SourceMetadataError> {
    let mut original = parts(2);
    original.source_metadata = Some(Box::new(secret_metadata(&["alpha", "beta"], &[])?));
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, payloads(&["alpha", "gamma"])),
        Err(WorkflowError::SourceMetadata {
            detail: "bound symbol payloads do not match source metadata".into(),
        })
    );
    Ok(())
}

#[test]
fn metadata_order_mismatch_is_typed() -> Result<(), SourceMetadataError> {
    let mut original = parts(2);
    original.source_metadata = Some(Box::new(secret_metadata(&["alpha", "beta"], &[])?));
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, payloads(&["beta", "alpha"])),
        Err(WorkflowError::SourceMetadata {
            detail: "bound symbol payloads do not match source metadata".into(),
        })
    );
    Ok(())
}

#[test]
fn original_validation_runs_before_symbol_check() -> Result<(), SourceMetadataError> {
    let mut original = parts(2);
    original.nodes = Box::new([]);
    original.source_metadata = Some(Box::new(secret_metadata(&["alpha", "beta"], &[])?));
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, payloads(&["beta", "alpha"])),
        Err(WorkflowError::EmptyNodes)
    );
    Ok(())
}

#[test]
fn legacy_unbound_preserves_original() -> Result<(), WorkflowError> {
    let original = parts(0);
    let workflow = CompiledWorkflow::try_from_parts(original.clone())?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

#[test]
fn empty_metadata_via_original_constructor_stays_unbound() -> Result<(), WorkflowError> {
    let mut original = parts(0);
    original.source_metadata = Some(Box::new(WorkflowSourceMetadata::empty()));
    let workflow = CompiledWorkflow::try_from_parts(original.clone())?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

#[test]
fn explicit_empty_payload_table_yields_v2() -> Result<(), WorkflowError> {
    let mut original = parts(0);
    original.source_metadata = Some(Box::new(WorkflowSourceMetadata::empty()));
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(original.clone(), Box::new([]))?;
    assert_eq!(workflow.symbol_payloads(), Some(payloads(&[]).as_ref()));
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

fn project_v2(workflow: &CompiledWorkflow) -> Result<WorkflowIrV2, Box<dyn std::error::Error>> {
    match workflow.to_versioned_parts() {
        VersionedWorkflowParts::V2(v2) => Ok(v2),
        VersionedWorkflowParts::V1(_) => Err("expected an explicitly bound V2 workflow".into()),
    }
}

fn postcard_roundtrip(v2: &WorkflowIrV2) -> Result<WorkflowIrV2, postcard::Error> {
    postcard::from_bytes(&postcard::to_allocvec(v2)?)
}

fn expected_secret_v2(original: WorkflowParts) -> WorkflowIrV2 {
    WorkflowIrV2 {
        wire_version: 2,
        parts: original,
        symbol_payloads: payloads(&["alias", "REQ_A"]),
    }
}

#[test]
fn secret_workflow_roundtrips_via_v2_dto() -> Result<(), Box<dyn std::error::Error>> {
    let original = secret_parts()?;
    let workflow = CompiledWorkflow::try_from_parts_with_symbols(
        original.clone(),
        payloads(&["alias", "REQ_A"]),
    )?;
    let expected = expected_secret_v2(original.clone());
    assert_eq!(
        workflow.source_metadata(),
        original.source_metadata.as_deref()
    );
    let v2 = project_v2(&workflow)?;
    assert_eq!(v2, expected);
    let decoded = postcard_roundtrip(&v2)?;
    assert_eq!(decoded, expected);
    assert_eq!(decoded.parts.digest, WorkflowDigest::from_bytes([7; 32]));
    let rebuilt =
        CompiledWorkflow::try_from_parts_with_symbols(decoded.parts, decoded.symbol_payloads)?;
    assert_eq!(
        rebuilt.to_versioned_parts(),
        VersionedWorkflowParts::V2(expected)
    );
    Ok(())
}
