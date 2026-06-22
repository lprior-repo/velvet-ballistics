//! Validation tests for vb_boundary_inventory
//!
//! Tests: validate_evidence_reference_bytes
// `panic!(...)` markers appear inside `let-else` arms and match arms whose
// opposite arm already established the variant. The panic is the documented
// "this branch is unreachable" marker with a clear diagnostic message. Tests
// are allowed to panic at the workspace level, so we use `panic!` (not
// panic markers) for clarity.
#![allow(clippy::panic, clippy::assertions_on_constants)]

use crate::boundary_inventory::{
    BoundaryInventoryError, EvidenceReference, validate_evidence_reference_bytes,
};

// =============================================================================
// validate_evidence_reference_bytes - external provenance
// =============================================================================

#[test]
fn validate_external_with_sha256() {
    let text = "external:vb-abc123#sha256=abcdef123456";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("validate_evidence_reference_bytes must succeed"),
    };
    match reference {
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
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("validate_evidence_reference_bytes must succeed"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

#[test]
fn validate_bead_id_uppercase_rejected() {
    // Uppercase not allowed in bead ID suffix
    let text = "vb-ABC123";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_bead_id_with_hyphens() {
    let text = "vb-a1b2c3";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("vb-a1b2c3 is valid bead ID"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance for valid bead ID"),
    }
}

#[test]
fn validate_bead_id_numbers_only() {
    let text = "vb-12345";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("vb-12345 is valid bead ID"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance for valid bead ID"),
    }
}

// =============================================================================
// validate_evidence_reference_bytes - invalid inputs
// =============================================================================

#[test]
fn validate_external_without_sha256_rejected() {
    let text = "external:some-reference";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_absolute_path_rejected() {
    let text = "/absolute/path/to/file.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_parent_dir_reference_rejected() {
    let text = "../parent/path.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_invalid_utf8_rejected() {
    // Invalid UTF-8 sequence
    let bytes: &[u8] = &[0xFF, 0xFE, 0xFD];
    let result = validate_evidence_reference_bytes(bytes);
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_free_text_rejected() {
    // Free text is not valid evidence reference
    let text = "some arbitrary text description";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_path_with_dots_at_start() {
    let text = "./test.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_external_multiple_colons() {
    let text = "external:many:colons#sha256=abc123";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("multiple-colon external is valid"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance for external provenance"),
    }
}

// =============================================================================
// validate_evidence_reference_bytes — empty and boundary inputs
// =============================================================================

#[test]
fn validate_empty_bytes_resolves_as_repo_local() {
    let result = validate_evidence_reference_bytes(b"");
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("validate_evidence_reference_bytes must succeed"),
    };
    match reference {
        EvidenceReference::RepoLocal { path, .. } => {
            assert!(path.as_os_str().is_empty());
        }
        _ => panic!("Expected RepoLocal"),
    }
}

#[test]
fn validate_single_byte_rejected() {
    let result = validate_evidence_reference_bytes(b"x");
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_null_byte_in_middle() {
    let bytes = b"vb-abc\0def";
    let result = validate_evidence_reference_bytes(bytes);
    // Null byte means it's not valid utf8 for str::from_utf8 -> InvalidEvidencePath
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_very_long_bead_id() {
    let suffix = "a".repeat(500);
    let text = format!("vb-{}", suffix);
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("long bead ID must be valid"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
            assert_eq!(val.len(), 503); // "vb-" (3) + 500 chars
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

// =============================================================================
// validate_evidence_reference_bytes — bead ID edge cases
// =============================================================================

#[test]
fn validate_bead_id_empty_suffix_rejected() {
    let text = "vb-";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_bead_id_no_hyphen_rejected() {
    let text = "vbabc";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_bead_id_multiple_hyphens_rejected() {
    let text = "vb-a-b";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_bead_id_single_letter_suffix() {
    let text = "vb-a";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("single-letter bead ID suffix must be valid"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

#[test]
fn validate_bead_id_single_digit_suffix() {
    let text = "vb-7";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    let reference = match result {
        Ok(v) => v,
        Err(e) => panic!("single-digit bead ID suffix must be valid"),
    };
    match reference {
        EvidenceReference::ExternalProvenance(val) => {
            assert_eq!(val, text);
        }
        _ => panic!("Expected ExternalProvenance"),
    }
}

#[test]
fn validate_bead_id_suffix_with_uppercase_rejected() {
    let text = "vb-AbCd";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_bead_id_suffix_with_special_chars_rejected() {
    // Special chars (@, #, etc.) not allowed in bead ID suffix
    let text = "vb-ab@cd";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

// =============================================================================
// validate_evidence_reference_bytes — external provenance edge cases
// =============================================================================

#[test]
fn validate_external_without_hash_rejected() {
    let text = "external:something";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_external_empty_after_colon_rejected() {
    let text = "external:";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_external_only_prefix_rejected() {
    let text = "external";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

// =============================================================================
// validate_evidence_reference_bytes — absolute and relative path edge cases
// =============================================================================

#[test]
fn validate_root_relative_path_rejected() {
    let text = "crates/test/src/lib.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    // Root-relative path (not workspace-relative) is invalid evidence
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_current_dir_path_rejected() {
    let text = "./crates/test/src/lib.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_double_parent_dir_rejected() {
    let text = "../../etc/passwd";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}

#[test]
fn validate_path_with_embedded_dot_dot_rejected() {
    let text = "crates/../malicious/src/lib.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    assert!(matches!(
        result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    ));
}
