#[path = "shipping_ir_profile/fixtures.rs"]
mod fixtures;

use fixtures::parts;
use vb_core::ids::StepIdx;
use vb_core::ids::SymbolId;
use vb_core::workflow::WorkflowIrV2;
use vb_core::workflow::source_metadata::SourceMetadataError;
use vb_core::workflow::{
    CompiledWorkflow, VersionedWorkflowParts, WorkflowError, WorkflowIrProfile,
};

#[test]
fn ordinary_legacy_parts_preserve_profile_and_projection() -> Result<(), WorkflowError> {
    let original = parts(0);
    let workflow = CompiledWorkflow::try_from_parts(original.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::LegacyV2);
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

#[test]
fn ordinary_concurrent_parts_preserve_profile_and_projection() -> Result<(), WorkflowError> {
    let original = parts(0);
    let workflow = CompiledWorkflow::try_from_concurrent_parts(original.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(original)
    );
    Ok(())
}

fn reject_new_under_legacy(literal: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let original = fixtures::malformed_concurrent(literal)?;
    let expected = Err(WorkflowError::ConcurrentProfileRequired {
        step: StepIdx::ZERO,
    });
    assert_eq!(CompiledWorkflow::try_from_parts(original.clone()), expected);
    assert_eq!(
        CompiledWorkflow::try_from_parts_with_symbols(original, Box::new([])),
        expected
    );
    Ok(())
}

#[test]
fn all_new_kind_literals_require_explicit_profile() -> Result<(), Box<dyn std::error::Error>> {
    reject_new_under_legacy(&[34, 0, 1, 3, 2, 1, 4])?;
    reject_new_under_legacy(&[35, 2, 1, 3, 5])?;
    reject_new_under_legacy(&[36, 2, 1, 3, 5])?;
    reject_new_under_legacy(&[37, 0, 0])?;
    reject_new_under_legacy(&[37, 0, 1, 2])?;
    reject_new_under_legacy(&[38, 0])?;
    Ok(())
}

fn reject_common_parts(original: vb_core::workflow::WorkflowParts, error: WorkflowError) {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(original.clone()),
        Err(error.clone())
    );
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts_with_symbols(original, Box::new([])),
        Err(error)
    );
}

#[test]
fn concurrent_constructors_reject_empty_nodes() {
    let mut original = parts(0);
    original.nodes = Box::new([]);
    reject_common_parts(original, WorkflowError::EmptyNodes);
}

#[test]
fn concurrent_constructors_reject_out_of_bounds_slot() {
    let mut original = parts(0);
    original.slot_count = 0;
    reject_common_parts(
        original,
        WorkflowError::SlotOutOfBounds {
            slot: vb_core::ids::SlotIdx::new(0),
        },
    );
}

#[test]
fn concurrent_constructors_reject_invalid_entry() {
    let original = parts(0);
    let mut bad = original.clone();
    bad.entry = StepIdx::new(99);
    reject_common_parts(
        bad,
        WorkflowError::EntryOutOfBounds {
            entry: StepIdx::new(99),
        },
    );
}

#[test]
fn concurrent_bound_projection_preserves_ordered_source() -> Result<(), Box<dyn std::error::Error>>
{
    use vb_core::workflow::WorkflowIrV2;
    let original = fixtures::source_parts()?;
    let table = fixtures::payloads(&["z", "é", ""]);
    let workflow =
        CompiledWorkflow::try_from_concurrent_parts_with_symbols(original.clone(), table.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(
        workflow.source_metadata(),
        original.source_metadata.as_deref()
    );
    assert_eq!(workflow.symbol_payloads(), Some(table.as_ref()));
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(WorkflowIrV2 {
            wire_version: 2,
            parts: original,
            symbol_payloads: table,
        })
    );
    Ok(())
}

fn reject_bound(
    original: vb_core::workflow::WorkflowParts,
    table: Box<[Box<str>]>,
    error: WorkflowError,
) {
    let r1 = CompiledWorkflow::try_from_parts_with_symbols(original.clone(), table.clone());
    assert_eq!(r1, Err(error.clone()));
    let r2 = CompiledWorkflow::try_from_concurrent_parts_with_symbols(original, table);
    assert_eq!(r2, Err(error));
}

#[test]
fn different_payload_bytes_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixtures::source_parts()?;
    let payloads = fixtures::payloads(&["z", "q", ""]);
    reject_bound(
        parts,
        payloads,
        WorkflowError::SourceMetadata {
            detail: "bound symbol payloads do not match source metadata".into(),
        },
    );
    Ok(())
}

#[test]
fn reordered_payloads_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixtures::source_parts()?;
    let payloads = fixtures::payloads(&["é", "z", ""]);
    reject_bound(
        parts,
        payloads,
        WorkflowError::SourceMetadata {
            detail: "bound symbol payloads do not match source metadata".into(),
        },
    );
    Ok(())
}

#[test]
fn duplicate_bound_payloads_are_rejected() {
    use vb_core::ids::SymbolId;
    use vb_core::workflow::source_metadata::SourceMetadataError;

    let error = WorkflowError::InvalidSymbolMetadata {
        source: SourceMetadataError::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(1),
        },
    };

    reject_bound(parts(2), fixtures::payloads(&["key", "key"]), error);
}

#[test]
fn bound_payload_count_is_checked() {
    use vb_core::workflow::source_metadata::SourceMetadataError;

    let parts = parts(2);
    let error = WorkflowError::InvalidSymbolMetadata {
        source: SourceMetadataError::SymbolCountMismatch {
            declared: 2,
            actual: 1,
        },
    };
    reject_bound(parts, fixtures::payloads(&["key"]), error);
}

#[test]
fn duplicate_metadata_precedes_empty_nodes() {
    let mut original = parts(2);
    original.nodes = Box::new([]);

    let error = WorkflowError::InvalidSymbolMetadata {
        source: SourceMetadataError::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(1),
        },
    };

    reject_bound(original, fixtures::payloads(&["key", "key"]), error);
}

#[test]
fn symbol_count_metadata_precedes_empty_nodes() {
    let original = parts(2);
    let mut original = original;
    original.nodes = Box::new([]);
    let error = WorkflowError::InvalidSymbolMetadata {
        source: SourceMetadataError::SymbolCountMismatch {
            declared: 2,
            actual: 1,
        },
    };
    reject_bound(original, fixtures::payloads(&["key"]), error);
}

#[test]
fn valid_metadata_empty_nodes_precedes_payload_mismatch() -> Result<(), Box<dyn std::error::Error>>
{
    let mut original = fixtures::source_parts()?;
    original.nodes = Box::new([]);
    reject_bound(
        original,
        fixtures::payloads(&["é", "z", ""]),
        WorkflowError::EmptyNodes,
    );
    Ok(())
}

#[test]
fn concurrent_bound_preserves_executable_secret_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let original = fixtures::secret_parts()?;
    let table = fixtures::payloads(&["alias", "REQ_A"]);
    let workflow =
        CompiledWorkflow::try_from_concurrent_parts_with_symbols(original.clone(), table.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        workflow.source_metadata(),
        original.source_metadata.as_deref()
    );
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(WorkflowIrV2 {
            wire_version: 2,
            parts: original,
            symbol_payloads: table,
        })
    );
    Ok(())
}

#[path = "shipping_ir_profile/requirements.rs"]
mod requirements;
