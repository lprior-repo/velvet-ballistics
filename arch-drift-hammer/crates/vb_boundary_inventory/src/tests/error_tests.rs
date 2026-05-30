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

// =============================================================================
// Error in Result context
// =============================================================================

#[test]
fn error_in_result_workspace_not_discoverable() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::WorkspaceNotDiscoverable
    );
}

#[test]
fn error_in_result_inventory_parse_failure() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::InventoryParseFailure);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn error_in_result_schema_version_unsupported() {
    let result: Result<(), BoundaryInventoryError> =
        Err(BoundaryInventoryError::SchemaVersionUnsupported);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

// =============================================================================
// Error hash properties
// =============================================================================

#[test]
fn error_hash_all_13_variants_in_set() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(BoundaryInventoryError::WorkspaceNotDiscoverable);
    set.insert(BoundaryInventoryError::IncompleteDiscoveryInput);
    set.insert(BoundaryInventoryError::UnknownBoundaryClass);
    set.insert(BoundaryInventoryError::UnsafeForbiddenViolation);
    set.insert(BoundaryInventoryError::MissingOwner);
    set.insert(BoundaryInventoryError::MissingThreat);
    set.insert(BoundaryInventoryError::MissingEvidencePath);
    set.insert(BoundaryInventoryError::InvalidEvidencePath);
    set.insert(BoundaryInventoryError::StaleEvidence);
    set.insert(BoundaryInventoryError::DuplicateBoundaryId);
    set.insert(BoundaryInventoryError::InventoryParseFailure);
    set.insert(BoundaryInventoryError::SchemaVersionUnsupported);
    set.insert(BoundaryInventoryError::ReviewStatusInvalid);
    assert_eq!(set.len(), 13);
}

#[test]
fn error_hash_collision_free_across_13_variants() {
    use std::collections::HashSet;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let variants: [BoundaryInventoryError; 13] = [
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
    let mut hashes = Vec::new();
    for err in &variants {
        let mut hasher = DefaultHasher::new();
        err.hash(&mut hasher);
        hashes.push(hasher.finish());
    }
    let unique: HashSet<_> = hashes.iter().collect();
    assert_eq!(unique.len(), 13);
}

// =============================================================================
// Error formatting with all variants
// =============================================================================

#[test]
fn error_debug_formatting_includes_variant_name() {
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::MissingOwner),
        "MissingOwner"
    );
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::MissingThreat),
        "MissingThreat"
    );
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::StaleEvidence),
        "StaleEvidence"
    );
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::DuplicateBoundaryId),
        "DuplicateBoundaryId"
    );
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::UnsafeForbiddenViolation),
        "UnsafeForbiddenViolation"
    );
    assert_eq!(
        format!("{:?}", BoundaryInventoryError::InvalidEvidencePath),
        "InvalidEvidencePath"
    );
}

// =============================================================================
// Error size and alignment
// =============================================================================

#[test]
fn error_size_is_1_byte() {
    // Enum with 13 unit variants fits in 1 byte discriminant
    assert_eq!(size_of::<BoundaryInventoryError>(), 1);
}

#[test]
fn error_is_copy_and_clone() {
    let err = BoundaryInventoryError::WorkspaceNotDiscoverable;
    let copied: BoundaryInventoryError = err; // copy
    assert_eq!(err, copied);
    let cloned = err.clone();
    assert_eq!(err, cloned);
}
