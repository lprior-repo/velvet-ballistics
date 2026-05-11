use super::support::*;

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_missing() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"ipc-missing-source","class":"ipc"}]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_document_is_not_json_object() {
    let bytes = br#"[]"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_schema_version_unsupported_when_schema_version_is_zero() {
    let bytes = br#"{"schema_version":0,"boundaries":[]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(
        result,
        Err(BoundaryInventoryError::SchemaVersionUnsupported)
    );
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_boundaries_field_is_missing() {
    let bytes = br#"{"schema_version":1}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_boundaries_field_is_not_array() {
    let bytes = br#"{"schema_version":1,"boundaries":{"id":"not-array"}}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_boundary_entry_is_not_object() {
    let bytes = br#"{"schema_version":1,"boundaries":[7]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_boundary_id_is_not_string() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":7,"class":"ipc","source_path":"crates/vb_ipc/src/frame.rs"}]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_unknown_boundary_class_when_class_is_unknown() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"unknown-boundary","class":"unknown","source_path":"crates/unknown/src/lib.rs"}]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
}

#[test]
fn parse_inventory_returns_inventory_parse_failure_when_class_is_unrecognized() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"bad-class","class":"socket","source_path":"crates/vb_ipc/src/frame.rs"}]}"#;

    let result = parse_inventory(bytes);

    assert_eq!(result, Err(BoundaryInventoryError::InventoryParseFailure));
}

#[test]
fn parse_inventory_returns_boundary_inventory_when_decoder_record_is_valid_json() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"decoder-boundary","class":"decoder","source_path":"crates/vb_yaml/src/decode.rs"}]}"#;
    let expected = BoundaryInventory::new(
        Some(1),
        vec![BoundaryRecord::new(BoundaryRecordParts {
            id: String::from("decoder-boundary"),
            class: BoundaryClass::Decoder,
            source_path: PathBuf::from("crates/vb_yaml/src/decode.rs"),
            owner: FieldState::Missing,
            threat: FieldState::Missing,
            evidence: FieldState::Missing,
            freshness: FreshnessMarker::new(1, 1, 1),
            review_status: FieldState::Missing,
            waiver: FieldState::Missing,
        })],
        None,
    );

    let result = parse_inventory(bytes);

    assert_eq!(result, Ok(expected));
}

#[test]
fn parse_inventory_returns_c_abi_class_when_json_class_is_c_abi() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"c-abi-boundary","class":"c_abi","source_path":"crates/ffi/src/c_abi.rs"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::CAbi]));
}

#[test]
fn parse_inventory_returns_ffi_class_when_json_class_is_ffi() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"ffi-boundary","class":"ffi","source_path":"crates/ffi/src/lib.rs"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::Ffi]));
}

#[test]
fn parse_inventory_returns_ipc_class_when_json_class_is_ipc() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"ipc-boundary","class":"ipc","source_path":"crates/vb_ipc/src/frame.rs"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::Ipc]));
}

#[test]
fn parse_inventory_returns_external_binary_class_when_json_class_is_external_binary() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"external-binary-boundary","class":"external_binary","source_path":"scripts/run-verifier.sh"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::ExternalBinary]));
}

#[test]
fn parse_inventory_returns_generated_code_class_when_json_class_is_generated_code() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"generated-code-boundary","class":"generated_code","source_path":"crates/vb_codegen/src/generated/interface.rs"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::GeneratedCode]));
}

#[test]
fn parse_inventory_returns_unsafe_adjacent_dependency_class_when_json_class_matches() {
    let bytes = br#"{"schema_version":1,"boundaries":[{"id":"unsafe-adjacent-dependency-boundary","class":"unsafe_adjacent_dependency","source_path":"Cargo.toml"}]}"#;

    let result = parse_inventory(bytes).map(|inventory| record_classes(&inventory));

    assert_eq!(result, Ok(vec![BoundaryClass::UnsafeAdjacentDependency]));
}

#[test]
fn validate_evidence_reference_bytes_returns_invalid_evidence_path_when_bytes_are_not_utf8() {
    let result = validate_evidence_reference_bytes(&[255, 254, 253]);

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn validate_evidence_reference_bytes_returns_external_provenance_when_external_digest_exists() {
    let bytes = b"external:https://example.test/report#sha256=abc123";

    let result = validate_evidence_reference_bytes(bytes);

    assert_eq!(
        result,
        Ok(EvidenceReference::ExternalProvenance(String::from(
            "external:https://example.test/report#sha256=abc123"
        )))
    );
}

#[test]
fn validate_evidence_reference_bytes_rejects_partial_external_provenance_markers() {
    let external_without_digest =
        validate_evidence_reference_bytes(b"external:https://example.test/report");
    let digest_without_external = validate_evidence_reference_bytes(b"report#sha256=abc123");

    assert_eq!(
        external_without_digest,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
    assert_eq!(
        digest_without_external,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
}

#[test]
fn validate_evidence_reference_bytes_returns_external_provenance_when_bead_id_is_valid() {
    let result = validate_evidence_reference_bytes(b"vb-y1zq");

    assert_eq!(
        result,
        Ok(EvidenceReference::ExternalProvenance(String::from(
            "vb-y1zq"
        )))
    );
}

#[test]
fn validate_evidence_reference_bytes_rejects_empty_and_non_lowercase_bead_suffixes() {
    let empty_suffix = validate_evidence_reference_bytes(b"vb-");
    let uppercase_suffix = validate_evidence_reference_bytes(b"vb-Y1ZQ");

    assert_eq!(
        empty_suffix,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
    assert_eq!(
        uppercase_suffix,
        Err(BoundaryInventoryError::InvalidEvidencePath)
    );
}

#[test]
fn validate_evidence_reference_bytes_returns_repo_local_provenance_when_path_exists() {
    let evidence_path = "tests/fixtures/vb_y1zq/complete_workspace/formal-verification-report.md";
    let result = validate_evidence_reference_bytes(evidence_path.as_bytes());

    assert_eq!(
        result,
        Ok(EvidenceReference::repo_local(
            PathBuf::from(evidence_path),
            EvidenceKind::Provenance,
        ))
    );
}

#[test]
fn validate_evidence_reference_bytes_returns_invalid_evidence_path_when_path_missing() {
    let result = validate_evidence_reference_bytes(b"missing/evidence/report.md");

    assert_eq!(result, Err(BoundaryInventoryError::InvalidEvidencePath));
}

#[test]
fn review_status_serialized_returns_original_value_when_status_is_other() {
    let status = ReviewStatus::from_serialized("reviewed_elsewhere");

    assert_eq!(status.serialized(), "reviewed_elsewhere");
}
