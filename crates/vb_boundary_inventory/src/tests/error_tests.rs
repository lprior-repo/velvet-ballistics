//! Error variant tests for vb_boundary_inventory
//!
//! Tests all 13 BoundaryInventoryError variants

use crate::boundary_inventory::BoundaryInventoryError;

// =============================================================================
// BoundaryInventoryError - all 13 variants
// =============================================================================

#[test]
fn error_variant_workspace_not_discoverable() {
    let err = BoundaryInventoryError::WorkspaceNotDiscoverable;
    assert_eq!(format!("{:?}", err), "WorkspaceNotDiscoverable");
}

#[test]
fn error_variant_incomplete_discovery_input() {
    let err = BoundaryInventoryError::IncompleteDiscoveryInput;
    assert_eq!(format!("{:?}", err), "IncompleteDiscoveryInput");
}

#[test]
fn error_variant_unknown_boundary_class() {
    let err = BoundaryInventoryError::UnknownBoundaryClass;
    assert_eq!(format!("{:?}", err), "UnknownBoundaryClass");
}

#[test]
fn error_variant_unsafe_forbidden_violation() {
    let err = BoundaryInventoryError::UnsafeForbiddenViolation;
    assert_eq!(format!("{:?}", err), "UnsafeForbiddenViolation");
}

#[test]
fn error_variant_missing_owner() {
    let err = BoundaryInventoryError::MissingOwner;
    assert_eq!(format!("{:?}", err), "MissingOwner");
}

#[test]
fn error_variant_missing_threat() {
    let err = BoundaryInventoryError::MissingThreat;
    assert_eq!(format!("{:?}", err), "MissingThreat");
}

#[test]
fn error_variant_missing_evidence_path() {
    let err = BoundaryInventoryError::MissingEvidencePath;
    assert_eq!(format!("{:?}", err), "MissingEvidencePath");
}

#[test]
fn error_variant_invalid_evidence_path() {
    let err = BoundaryInventoryError::InvalidEvidencePath;
    assert_eq!(format!("{:?}", err), "InvalidEvidencePath");
}

#[test]
fn error_variant_stale_evidence() {
    let err = BoundaryInventoryError::StaleEvidence;
    assert_eq!(format!("{:?}", err), "StaleEvidence");
}

#[test]
fn error_variant_duplicate_boundary_id() {
    let err = BoundaryInventoryError::DuplicateBoundaryId;
    assert_eq!(format!("{:?}", err), "DuplicateBoundaryId");
}

#[test]
fn error_variant_inventory_parse_failure() {
    let err = BoundaryInventoryError::InventoryParseFailure;
    assert_eq!(format!("{:?}", err), "InventoryParseFailure");
}

#[test]
fn error_variant_schema_version_unsupported() {
    let err = BoundaryInventoryError::SchemaVersionUnsupported;
    assert_eq!(format!("{:?}", err), "SchemaVersionUnsupported");
}

#[test]
fn error_variant_review_status_invalid() {
    let err = BoundaryInventoryError::ReviewStatusInvalid;
    assert_eq!(format!("{:?}", err), "ReviewStatusInvalid");
}

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
    let cloned = err.clone();
    assert_eq!(err, cloned);
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
fn error_hash_consistency() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let err1 = BoundaryInventoryError::WorkspaceNotDiscoverable;
    let err2 = BoundaryInventoryError::WorkspaceNotDiscoverable;
    set.insert(err1);
    assert!(set.contains(&err2));
}

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

// =============================================================================
// Error Debug
// =============================================================================

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
