#![forbid(unsafe_code)]
//! Integration tests for vb_boundary_inventory evidence validation edge cases.
//!
//! Tests the evidence validation logic through the public API (validate_inventory).

use std::path::PathBuf;

use xtask::boundary_inventory::{
    BoundaryClass, BoundaryInventory, BoundaryInventoryError, BoundaryRecordDraft, EvidenceKind,
    EvidenceReference, FieldState, FreshnessMarker, Owner, ReviewStatus, ThreatStatement,
    WorkspaceRoot, validate_inventory,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_workspace_root(subdir: &str) -> WorkspaceRoot {
    WorkspaceRoot::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(subdir),
    )
}

fn valid_record_base() -> BoundaryRecordDraft {
    BoundaryRecordDraft {
        id: String::from("vb-test-boundary"),
        class: BoundaryClass::Ipc,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        owner: FieldState::Present(Owner(String::from("test-owner"))),
        threat: FieldState::Present(ThreatStatement(String::from("test-threat"))),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("crates/vb_ipc/src/frame.rs"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    }
}

// ---------------------------------------------------------------------------
// FreshnessMarker edge cases
// ---------------------------------------------------------------------------

/// FreshnessMarker: evidence_version < source_version → StaleEvidence.
#[test]
fn validate_record_returns_stale_evidence_when_evidence_version_behind_source() {
    let mut record = valid_record_base();
    // source_version = 2, evidence_version = 1 → stale
    record.freshness = FreshnessMarker::new(2, 1, 1);
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::StaleEvidence));
}

/// FreshnessMarker: evidence_version < schema_version → StaleEvidence.
#[test]
fn validate_record_returns_stale_evidence_when_evidence_version_behind_schema() {
    let mut record = valid_record_base();
    // schema_version = 3, evidence_version = 1 → stale
    record.freshness = FreshnessMarker::new(1, 3, 1);
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::StaleEvidence));
}

/// FreshnessMarker: evidence_version >= source_version and schema_version → valid.
#[test]
fn validate_record_accepts_fresh_evidence_when_versions_match() {
    let mut record = valid_record_base();
    // All versions equal → fresh
    record.freshness = FreshnessMarker::new(5, 5, 5);
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(result.is_ok(), "freshness should be valid: {:?}", result);
}

/// FreshnessMarker: all zeros is valid (zero versions).
#[test]
fn validate_record_accepts_zero_versions() {
    let mut record = valid_record_base();
    record.freshness = FreshnessMarker::new(0, 0, 0);
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "zero versions should be valid: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// EvidenceReference edge cases
// ---------------------------------------------------------------------------

/// EvidenceReference::FreeText rejected by validate_evidence_reference.
#[test]
fn validate_record_rejects_free_text_evidence() {
    let mut record = valid_record_base();
    record.evidence = FieldState::Present(EvidenceReference::FreeText(String::from(
        "just some free text evidence",
    )));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

/// EvidenceReference::ExternalProvenance with valid sha256 accepted.
#[test]
fn validate_record_accepts_external_provenance_with_sha256() {
    let mut record = valid_record_base();
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "external:https://example.com/fuzz-report#sha256=abcdef123456",
    )));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "external provenance with sha256 should be valid: {:?}",
        result
    );
}

/// EvidenceReference::ExternalProvenance without sha256 rejected.
#[test]
fn validate_record_rejects_external_provenance_without_sha256() {
    let mut record = valid_record_base();
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "external:https://example.com/fuzz-report",
    )));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

/// EvidenceReference::ExternalProvenance with valid bead ID accepted.
#[test]
fn validate_record_accepts_bead_id_as_external_provenance() {
    let mut record = valid_record_base();
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "vb-y1zq",
    )));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "bead ID should be valid external provenance: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// ReviewStatus edge cases
// ---------------------------------------------------------------------------

/// ReviewStatus::Waived requires waiver FieldState::Present.
#[test]
fn validate_record_rejects_waived_without_waiver() {
    let mut record = valid_record_base();
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    record.waiver = FieldState::Missing;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

/// ReviewStatus::Waived with waiver FieldState::Present accepted.
#[test]
fn validate_record_accepts_waived_with_valid_waiver() {
    let mut record = valid_record_base();
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    record.waiver = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "vb-y1zq",
    )));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "waived with waiver should be valid: {:?}",
        result
    );
}

/// ReviewStatus::Other rejected (only Approved or Waived allowed).
#[test]
fn validate_record_rejects_review_status_other() {
    let mut record = valid_record_base();
    record.review_status = FieldState::Present(ReviewStatus::Other(String::from("pending")));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

// ---------------------------------------------------------------------------
// Owner and Threat edge cases
// ---------------------------------------------------------------------------

/// Empty owner string rejected as MissingOwner.
#[test]
fn validate_record_rejects_empty_owner() {
    let mut record = valid_record_base();
    record.owner = FieldState::Present(Owner(String::from("")));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::MissingOwner));
}

/// Empty threat string rejected as MissingThreat.
#[test]
fn validate_record_rejects_empty_threat() {
    let mut record = valid_record_base();
    record.threat = FieldState::Present(ThreatStatement(String::from("")));
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::MissingThreat));
}

/// Missing owner field rejected as MissingOwner.
#[test]
fn validate_record_rejects_missing_owner() {
    let mut record = valid_record_base();
    record.owner = FieldState::Missing;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::MissingOwner));
}

/// Missing threat field rejected as MissingThreat.
#[test]
fn validate_record_rejects_missing_threat() {
    let mut record = valid_record_base();
    record.threat = FieldState::Missing;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::MissingThreat));
}

// ---------------------------------------------------------------------------
// Source path edge cases
// ---------------------------------------------------------------------------

/// Source path not starting with allowed prefix → WorkspaceNotDiscoverable.
#[test]
fn validate_record_rejects_non_workspace_source_path() {
    let mut record = valid_record_base();
    record.source_path = PathBuf::from("src/main.rs");
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

/// Empty source path → InventoryParseFailure.
#[test]
fn validate_record_rejects_empty_source_path() {
    let mut record = valid_record_base();
    record.source_path = PathBuf::from("");
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

// ---------------------------------------------------------------------------
// BoundaryClass edge cases
// ---------------------------------------------------------------------------

/// BoundaryClass::Unknown → UnknownBoundaryClass.
#[test]
fn validate_record_rejects_unknown_boundary_class() {
    let mut record = valid_record_base();
    record.class = BoundaryClass::Unknown;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
}

/// BoundaryClass::CAbi is valid (allowed class).
#[test]
fn validate_record_accepts_cabi_boundary_class() {
    let mut record = valid_record_base();
    record.class = BoundaryClass::CAbi;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(result.is_ok(), "CAbi should be valid: {:?}", result);
}

/// BoundaryClass::GeneratedCode is valid.
#[test]
fn validate_record_accepts_generated_code_boundary_class() {
    let mut record = valid_record_base();
    record.class = BoundaryClass::GeneratedCode;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "GeneratedCode should be valid: {:?}",
        result
    );
}

/// BoundaryClass::UnsafeAdjacentDependency is valid.
#[test]
fn validate_record_accepts_unsafe_adjacent_dependency_boundary_class() {
    let mut record = valid_record_base();
    record.class = BoundaryClass::UnsafeAdjacentDependency;
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "UnsafeAdjacentDependency should be valid: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// EvidenceKind edge cases
// ---------------------------------------------------------------------------

/// EvidenceKind::Fuzz is valid for repo_local evidence.
#[test]
fn evidence_kind_fuzz_is_valid() {
    let ref_ = EvidenceReference::repo_local(
        PathBuf::from("crates/vb_ipc/src/frame.rs"),
        EvidenceKind::Fuzz,
    );
    assert!(matches!(
        ref_,
        EvidenceReference::RepoLocal {
            kind: EvidenceKind::Fuzz,
            ..
        }
    ));
}

/// EvidenceKind::ManualQa is valid.
#[test]
fn evidence_kind_manual_qa_is_valid() {
    let ref_ = EvidenceReference::repo_local(
        PathBuf::from("crates/vb_ipc/src/frame.rs"),
        EvidenceKind::ManualQa,
    );
    assert!(matches!(
        ref_,
        EvidenceReference::RepoLocal {
            kind: EvidenceKind::ManualQa,
            ..
        }
    ));
}

/// EvidenceKind::Isolation is valid.
#[test]
fn evidence_kind_isolation_is_valid() {
    let ref_ = EvidenceReference::repo_local(
        PathBuf::from("crates/vb_ipc/src/frame.rs"),
        EvidenceKind::Isolation,
    );
    assert!(matches!(
        ref_,
        EvidenceReference::RepoLocal {
            kind: EvidenceKind::Isolation,
            ..
        }
    ));
}

/// EvidenceKind::Provenance is valid.
#[test]
fn evidence_kind_provenance_is_valid() {
    let ref_ = EvidenceReference::repo_local(
        PathBuf::from("crates/vb_ipc/src/frame.rs"),
        EvidenceKind::Provenance,
    );
    assert!(matches!(
        ref_,
        EvidenceReference::RepoLocal {
            kind: EvidenceKind::Provenance,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// ReviewStatus serialization edge cases
// ---------------------------------------------------------------------------

/// ReviewStatus::from_serialized("approved") → Approved.
#[test]
fn review_status_from_serialized_approved() {
    let status = ReviewStatus::from_serialized("approved");
    assert!(matches!(status, ReviewStatus::Approved));
}

/// ReviewStatus::from_serialized("waived") → Waived.
#[test]
fn review_status_from_serialized_waived() {
    let status = ReviewStatus::from_serialized("waived");
    assert!(matches!(status, ReviewStatus::Waived));
}

/// ReviewStatus::from_serialized("unknown") → Other("unknown").
#[test]
fn review_status_from_serialized_other() {
    let status = ReviewStatus::from_serialized("some_review_status");
    assert!(matches!(status, ReviewStatus::Other(s) if s == "some_review_status"));
}

/// ReviewStatus::serialized round-trip for Approved.
#[test]
fn review_status_serialized_roundtrip_approved() {
    let status = ReviewStatus::Approved;
    assert_eq!(status.serialized(), "approved");
}

/// ReviewStatus::serialized round-trip for Waived.
#[test]
fn review_status_serialized_roundtrip_waived() {
    let status = ReviewStatus::Waived;
    assert_eq!(status.serialized(), "waived");
}

/// ReviewStatus::serialized for Other includes the custom value.
#[test]
fn review_status_serialized_for_other_includes_value() {
    let status = ReviewStatus::Other(String::from("custom_status"));
    assert_eq!(status.serialized(), "custom_status");
}

// ---------------------------------------------------------------------------
// Complete pipeline: all-valid record passes
// ---------------------------------------------------------------------------

/// Full valid record passes all validation checks.
#[test]
fn validate_record_accepts_fully_valid_record() {
    let record = valid_record_base();
    let workspace = make_workspace_root(".");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);

    let result = validate_inventory(inventory, workspace);
    assert!(
        result.is_ok(),
        "fully valid record should pass: {:?}",
        result
    );
}
