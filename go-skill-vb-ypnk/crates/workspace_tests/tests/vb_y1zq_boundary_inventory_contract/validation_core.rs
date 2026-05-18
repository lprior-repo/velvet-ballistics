use super::support::*;

#[test]
fn validate_inventory_returns_missing_owner_when_owner_absent() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.owner = FieldState::Missing;

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::MissingOwner));
}

#[test]
fn validate_inventory_returns_missing_threat_when_threat_absent() {
    let mut record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    record.threat = FieldState::Missing;

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::MissingThreat));
}

#[test]
fn validate_inventory_returns_missing_evidence_path_when_risky_boundary_lacks_evidence() {
    let mut record = valid_record(BoundaryClass::ExternalBinary, "scripts/run-verifier.sh");
    record.evidence = FieldState::Missing;

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::MissingEvidencePath));
}

#[test]
fn validate_inventory_returns_invalid_evidence_path_when_evidence_is_free_text() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence =
        FieldState::Present(EvidenceReference::free_text("we should fuzz this later"));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_inventory_returns_invalid_evidence_path_when_evidence_is_absolute_outside_repo() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from("/tmp/evidence/report.md"),
        EvidenceKind::Fuzz,
    ));

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_inventory_returns_stale_evidence_when_evidence_version_precedes_boundary_version() {
    let mut record = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    record.freshness = FreshnessMarker::new(100, 100, 99);

    let result = validate_inventory(inventory_with(record), workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::StaleEvidence));
}

#[test]
fn validate_inventory_rejects_evidence_stale_against_source_or_schema_independently() {
    let mut source_stale = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    source_stale.freshness = FreshnessMarker::new(10, 1, 5);
    let mut schema_stale = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    schema_stale.freshness = FreshnessMarker::new(1, 10, 5);

    let source_result = validate_inventory(
        inventory_with(source_stale),
        workspace("complete_workspace"),
    );
    let schema_result = validate_inventory(
        inventory_with(schema_stale),
        workspace("complete_workspace"),
    );

    assert_eq!(source_result, Err(BoundaryInventoryError::StaleEvidence));
    assert_eq!(schema_result, Err(BoundaryInventoryError::StaleEvidence));
}

#[test]
fn validate_inventory_accepts_equal_freshness_and_rejects_source_regression() {
    let mut equal = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    equal.freshness = FreshnessMarker::new(10, 10, 10);
    let expected_equal = equal.clone();
    let mut stale = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    stale.freshness = FreshnessMarker::new(11, 10, 10);

    let equal_result = validate_inventory(inventory_with(equal), workspace("complete_workspace"));
    let stale_result = validate_inventory(inventory_with(stale), workspace("complete_workspace"));

    assert_eq!(
        equal_result,
        Ok(validated_with_records_and_status(
            vec![expected_equal],
            "approved"
        ))
    );
    assert_eq!(stale_result, Err(BoundaryInventoryError::StaleEvidence));
}

#[test]
fn validate_inventory_accepts_equal_freshness_and_rejects_schema_regression() {
    let mut equal = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    equal.freshness = FreshnessMarker::new(10, 10, 10);
    let expected_equal = equal.clone();
    let mut stale = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    stale.freshness = FreshnessMarker::new(10, 11, 10);

    let equal_result = validate_inventory(inventory_with(equal), workspace("complete_workspace"));
    let stale_result = validate_inventory(inventory_with(stale), workspace("complete_workspace"));

    assert_eq!(
        equal_result,
        Ok(validated_with_records_and_status(
            vec![expected_equal],
            "approved"
        ))
    );
    assert_eq!(stale_result, Err(BoundaryInventoryError::StaleEvidence));
}

#[test]
fn validate_inventory_returns_duplicate_boundary_id_when_distinct_sources_share_id() {
    let first = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let mut second = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    second.id = first.id.clone();
    let inventory = BoundaryInventory::new(
        Some(1),
        vec![first, second],
        Some(evidence("proof-obligations.jsonl")),
    );

    let result = validate_inventory(inventory, workspace("complete_workspace"));

    assert_eq!(result, Err(BoundaryInventoryError::DuplicateBoundaryId));
}

#[test]
fn validate_inventory_returns_schema_version_unsupported_when_schema_version_missing() {
    let record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let inventory = BoundaryInventory::new(
        None,
        vec![record],
        Some(evidence("proof-obligations.jsonl")),
    );

    let result = validate_inventory(inventory, workspace("complete_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::SchemaVersionUnsupported)
    );
}

#[test]
fn validate_inventory_accepts_schema_version_one_when_other_fields_valid() {
    let record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
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
fn validate_inventory_accepts_empty_inventory_when_schema_version_is_supported() {
    let inventory = BoundaryInventory::new(Some(1), Vec::new(), None);

    let result = validate_inventory(inventory, workspace("complete_workspace"));

    assert_eq!(
        result,
        Ok(ValidatedBoundaryInventory::with_schema_version(1))
    );
}
