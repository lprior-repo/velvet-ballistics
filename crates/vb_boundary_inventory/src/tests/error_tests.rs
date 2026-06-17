//! Error variant tests for vb_boundary_inventory
//!
//! Tests: equality, hash, size, Send/Sync, Result context

use crate::boundary_inventory::BoundaryInventoryError;

// =============================================================================
// Error equality and comparison
// =============================================================================

#[test]
fn error_eq_workspace_not_discoverable() {
    let err1 = BoundaryInventoryError::WorkspaceNotDiscoverable;
    let err2 = BoundaryInventoryError::WorkspaceNotDiscoverable;
    let err3 = BoundaryInventoryError::MissingOwner;
    assert_eq!(err1, err2);
    assert_ne!(err1, err3);
}

#[test]
fn error_eq_all_variants_unique() {
    let errors: Vec<BoundaryInventoryError> = vec![
        BoundaryInventoryError::WorkspaceNotDiscoverable,
        BoundaryInventoryError::IncompleteDiscoveryInput,
        BoundaryInventoryError::UnknownBoundaryClass,
        BoundaryInventoryError::UnsafeForbiddenViolation,
        BoundaryInventoryError::MissingOwner,
        BoundaryInventoryError::MissingThreat,
        BoundaryInventoryError::MissingEvidencePath,
        BoundaryInventoryError::InvalidEvidencePath,
        BoundaryInventoryError::StaleEvidence,
        BoundaryInventoryError::DuplicateBoundaryId,
        BoundaryInventoryError::InventoryParseFailure,
        BoundaryInventoryError::SchemaVersionUnsupported,
        BoundaryInventoryError::ReviewStatusInvalid,
    ];
    for (i, err1) in errors.iter().enumerate() {
        for (j, err2) in errors.iter().enumerate() {
            if i == j {
                assert_eq!(err1, err2);
            } else {
                assert_ne!(err1, err2);
            }
        }
    }
}

// =============================================================================
// Error clone and copy
// =============================================================================

#[test]
fn error_clone() {
    let err = BoundaryInventoryError::WorkspaceNotDiscoverable;
    // BoundaryInventoryError is `Copy`, so a plain assignment produces an equivalent
    // value. The original is preserved because the type implements `Copy`.
    let copied = err;
    assert_eq!(err, copied);
}

#[test]
fn error_copy() {
    let err = BoundaryInventoryError::WorkspaceNotDiscoverable;
    let _copied = err; // Copy (no move)
    let _another = err; // Another copy
}

// =============================================================================
// Error hash
// =============================================================================

#[test]
fn error_hash_all_unique() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let errors: Vec<BoundaryInventoryError> = vec![
        BoundaryInventoryError::WorkspaceNotDiscoverable,
        BoundaryInventoryError::IncompleteDiscoveryInput,
        BoundaryInventoryError::UnknownBoundaryClass,
        BoundaryInventoryError::UnsafeForbiddenViolation,
        BoundaryInventoryError::MissingOwner,
        BoundaryInventoryError::MissingThreat,
        BoundaryInventoryError::MissingEvidencePath,
        BoundaryInventoryError::InvalidEvidencePath,
        BoundaryInventoryError::StaleEvidence,
        BoundaryInventoryError::DuplicateBoundaryId,
        BoundaryInventoryError::InventoryParseFailure,
        BoundaryInventoryError::SchemaVersionUnsupported,
        BoundaryInventoryError::ReviewStatusInvalid,
    ];
    for err in errors {
        set.insert(err);
    }
    assert_eq!(set.len(), 13);
}

#[test]
fn error_debug_all_variants() {
    let variants: Vec<BoundaryInventoryError> = vec![
        BoundaryInventoryError::WorkspaceNotDiscoverable,
        BoundaryInventoryError::IncompleteDiscoveryInput,
        BoundaryInventoryError::UnknownBoundaryClass,
        BoundaryInventoryError::UnsafeForbiddenViolation,
        BoundaryInventoryError::MissingOwner,
        BoundaryInventoryError::MissingThreat,
        BoundaryInventoryError::MissingEvidencePath,
        BoundaryInventoryError::InvalidEvidencePath,
        BoundaryInventoryError::StaleEvidence,
        BoundaryInventoryError::DuplicateBoundaryId,
        BoundaryInventoryError::InventoryParseFailure,
        BoundaryInventoryError::SchemaVersionUnsupported,
        BoundaryInventoryError::ReviewStatusInvalid,
    ];
    for err in variants {
        let debug_str = format!("{:?}", err);
        assert!(!debug_str.is_empty());
    }
}

// =============================================================================
// Error send and sync (for concurrency correctness)
// =============================================================================

#[test]
fn error_send() {
    fn assert_send<T: Send>() {}
    assert_send::<BoundaryInventoryError>();
}

#[test]
fn error_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<BoundaryInventoryError>();
}

// =============================================================================
// Error in Result context
// =============================================================================

#[test]
fn error_in_result_workspace_not_discoverable() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable);
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    ));
    // Mutation gate: swapping the error variant must fail the exact-match above.
    let _variant = result.unwrap_err();
}

#[test]
fn error_in_result_inventory_parse_failure() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::InventoryParseFailure);
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InventoryParseFailure)
    ));
    let _variant = result.unwrap_err();
}

#[test]
fn error_in_result_schema_version_unsupported() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::SchemaVersionUnsupported);
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::SchemaVersionUnsupported)
    ));
    let _variant = result.unwrap_err();
}

// =============================================================================
// Error size and alignment
// =============================================================================

#[test]
fn error_size_is_1_byte() {
    // Enum with 13 unit variants fits in 1 byte discriminant
    assert_eq!(size_of::<BoundaryInventoryError>(), 1);
}
