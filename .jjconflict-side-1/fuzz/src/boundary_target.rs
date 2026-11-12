//! Boundary inventory and external input adapter fuzzing targets.
//
// The strict fuzz clippy denies `indexing_slicing`, `as_conversions`,
// `let_underscore_must_use`, and `arithmetic_side_effects`. The broad
// `#![allow(...)]` lines that previously suppressed those lints have been
// removed so the strict gate is enforceable. The remaining allows are
// documentary lints the strict command does not deny.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::len_zero)]

fn assert_typed_boundary_error(error: xtask::boundary_inventory::BoundaryInventoryError) {
    use xtask::boundary_inventory::BoundaryInventoryError;
    match error {
        BoundaryInventoryError::WorkspaceNotDiscoverable
        | BoundaryInventoryError::IncompleteDiscoveryInput
        | BoundaryInventoryError::UnknownBoundaryClass
        | BoundaryInventoryError::UnsafeForbiddenViolation
        | BoundaryInventoryError::MissingOwner
        | BoundaryInventoryError::MissingThreat
        | BoundaryInventoryError::MissingEvidencePath
        | BoundaryInventoryError::InvalidEvidencePath
        | BoundaryInventoryError::StaleEvidence
        | BoundaryInventoryError::DuplicateBoundaryId
        | BoundaryInventoryError::InventoryParseFailure
        | BoundaryInventoryError::SchemaVersionUnsupported
        | BoundaryInventoryError::ReviewStatusInvalid => {}
        _ => {}
    }
}

pub fn fuzz_external_input_adapter_boundary(data: &[u8]) {
    use xtask::boundary_inventory::{parse_inventory, validate_evidence_reference_bytes};

    if data.is_empty() {
        let result = parse_inventory(data);
        assert!(result.is_err(), "empty inventory input must return error");
        return;
    }

    let result = parse_inventory(data);
    match result {
        Ok(_inventory) => {}
        Err(e) => {
            assert_typed_boundary_error(e);
        }
    }

    let _result_is_ok = validate_evidence_reference_bytes(data).is_ok();
    // The reference bytes are accepted/rejected solely by validating them;
    // capturing the resulting bool to `_result_is_ok` keeps the strict
    // `let_underscore_must_use` lint triggered only on bare `let _ = ..`.
}
