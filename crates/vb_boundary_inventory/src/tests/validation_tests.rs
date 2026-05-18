//! Validation tests for vb_boundary_inventory
//!
//! Tests: validate_evidence_reference_bytes

use crate::boundary_inventory::{BoundaryInventoryError, EvidenceKind, EvidenceReference, validate_evidence_reference_bytes};

// =============================================================================
// validate_evidence_reference_bytes - external provenance
// =============================================================================

#[test]
fn validate_external_with_sha256() {
    let text = "external:vb-abc123#sha256=abcdef123456";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(result.is_ok());
    match result.unwrap() {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

#[test]
fn validate_bead_id_valid() {
    // Valid bead ID format: vb-{suffix}
    let text = "vb-abc123";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(result.is_ok());
    match result.unwrap() {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

#[test]
fn validate_bead_id_uppercase_rejected() {
    // Uppercase not allowed in suffix - falls through to repo_local path
    let text = "vb-ABC123";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    // This should error because the path doesn't exist
    assert!(result.is_err());
}

#[test]
fn validate_bead_id_with_hyphens() {
    let text = "vb-a1b2c3";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(result.is_ok());
}

#[test]
fn validate_bead_id_numbers_only() {
    let text = "vb-12345";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(result.is_ok());
}

// =============================================================================
// validate_evidence_reference_bytes - invalid inputs
// =============================================================================

#[test]
fn validate_external_without_sha256_rejected() {
    let text = "external:some-reference";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_absolute_path_rejected() {
    let text = "/absolute/path/to/file.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_parent_dir_reference_rejected() {
    let text = "../parent/path.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_invalid_utf8_rejected() {
    // Invalid UTF-8 sequence
    let bytes: &[u8] = &[0xFF, 0xFE, 0xFD];
    let result = validate_evidence_reference_bytes(bytes);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_free_text_rejected() {
    // Free text is not valid evidence reference
    let text = "some arbitrary text description";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_path_with_dots_at_start() {
    let text = "./test.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
}

#[test]
fn validate_external_multiple_colons() {
    let text = "external:many:colons#sha256=abc123";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(result.is_ok());
}

