use super::support::*;

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_missing() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Missing;

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_accepts_review_status_approved_when_other_fields_valid() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::Approved);
    let expected_record = record.clone();

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(
        result,
        Ok(validated_with_records_and_status(
            vec![expected_record],
            "approved"
        ))
    );
}

#[test]
fn validate_inventory_accepts_review_status_waived_when_waiver_reference_exists() {
    let mut record = valid_record(BoundaryClass::ExternalBinary, "scripts/run-verifier.sh");
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    record.waiver = FieldState::Present(evidence(".beads/vb-y1zq/contract-verification-review.md"));
    let expected_record = record.clone();

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(
        result,
        Ok(validated_with_records_and_status(
            vec![expected_record],
            "waived"
        ))
    );
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_waived_without_waiver() {
    let mut record = valid_record(BoundaryClass::ExternalBinary, "scripts/run-verifier.sh");
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    record.waiver = FieldState::Missing;

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_accepts_external_sha256_provenance_as_evidence_path() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "external:https://example.test/fuzz-report#sha256=abcdef",
    )));
    let expected_record = record.clone();

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(
        result,
        Ok(validated_with_records_and_status(
            vec![expected_record],
            "approved"
        ))
    );
}

#[test]
fn validate_inventory_accepts_bead_id_provenance_as_evidence_path() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "vb-y1zq",
    )));
    let expected_record = record.clone();

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(
        result,
        Ok(validated_with_records_and_status(
            vec![expected_record],
            "approved"
        ))
    );
}

#[test]
fn validate_inventory_returns_invalid_evidence_path_when_external_provenance_lacks_digest() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
        "external:https://example.test/fuzz-report",
    )));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_inventory_returns_invalid_evidence_path_when_repo_local_path_does_not_exist() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from("missing/evidence/report.md"),
        EvidenceKind::Fuzz,
    ));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_inventory_returns_invalid_evidence_path_when_evidence_contains_parent_dir() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from("../formal-verification-report.md"),
        EvidenceKind::Fuzz,
    ));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_inventory_rejects_absolute_and_parent_evidence_paths_independently() {
    let mut absolute = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    absolute.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        EvidenceKind::Fuzz,
    ));
    let mut parent = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    parent.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from("../vb-y1zq/formal-verification-report.md"),
        EvidenceKind::Fuzz,
    ));

    let absolute_result =
        validate_inventory(inventory_with(absolute), workspace("complete_workspace"));
    let parent_result = validate_inventory(inventory_with(parent), workspace("complete_workspace"));

    assert_eq!(
        absolute_result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
    assert_eq!(
        parent_result,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_empty() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"ipc-empty-source","class":"ipc","source_path":""}]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn validate_inventory_returns_workspace_not_discoverable_when_source_path_surface_cannot_be_read() {
    let record = valid_record(BoundaryClass::Ipc, "missing-surface/frame.rs");

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_blocked_follow_up() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("blocked_follow_up"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_empty() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized(""));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_pending() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("pending"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_blocked() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("blocked"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_uppercase_approved() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("APPROVED"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_titlecase_approved() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("Approved"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}

#[test]
fn validate_inventory_returns_review_status_invalid_when_review_status_is_unknown_reviewed() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.review_status = FieldState::Present(ReviewStatus::from_serialized("reviewed"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::ReviewStatusInvalid));
}
