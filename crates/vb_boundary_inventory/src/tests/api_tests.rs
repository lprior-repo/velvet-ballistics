//! API function tests for vb_boundary_inventory
//!
//! Tests 5 pub fns: discover_boundaries, classify_boundary, required_evidence,
//! validate_inventory, inventory_completion_status

use std::fs;
use std::path::PathBuf;

use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryInventory, BoundaryInventoryError,
    BoundaryRecord, BoundaryRecordDraft, BoundaryRecordParts, BoundaryRisk, ClassifiedBoundary,
    ClassifiedBoundaryInput, EvidenceKind, EvidenceReference, EvidenceRequirement, FieldState,
    FreshnessMarker, Owner, ReviewStatus, ThreatStatement, UnsafeIsolationStatus,
    ValidatedBoundaryInventory, WorkspaceRoot, classify_boundary, discover_boundaries,
    inventory_completion_status, required_evidence, validate_inventory,
};

fn assert_io_ok(result: std::io::Result<()>, context: &str) {
    assert!(result.is_ok(), "{context}: {result:?}");
}

// =============================================================================
// discover_boundaries tests
// =============================================================================

#[test]
fn discover_boundaries_rejects_nonexistent_workspace() {
    let workspace = WorkspaceRoot::new(PathBuf::from("/nonexistent/path"));
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::WorkspaceNotDiscoverable
    );
}

#[test]
fn discover_boundaries_rejects_workspace_without_required_surfaces() {
    let temp_dir = test_tempdir();
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::WorkspaceNotDiscoverable
    );
}

#[test]
fn discover_boundaries_rejects_incomplete_workspace() {
    let temp_dir = test_tempdir();
    // Create only some required surfaces
    assert_io_ok(
        fs::create_dir(temp_dir.path().join("crates")),
        "create crates",
    );
    assert_io_ok(
        fs::write(temp_dir.path().join("Cargo.toml"), ""),
        "write Cargo.toml",
    );
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    // Missing fuzz, scripts directories
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::WorkspaceNotDiscoverable
    );
}

#[test]
fn discover_boundaries_finds_markers_in_crates() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    // Create a file with a boundary marker
    let marker_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    assert_io_ok(
        fs::create_dir_all(marker_file.parent().unwrap()),
        "create marker parent",
    );
    assert_io_ok(
        fs::write(&marker_file, "// extern-c-boundary"),
        "write marker file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "extern-c-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("crates/vb_core/src/lib.rs"));
}

#[test]
fn discover_boundaries_finds_markers_in_fuzz() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    let marker_file = temp_dir.path().join("fuzz/fuzz_target_1.rs");
    assert_io_ok(
        fs::create_dir_all(marker_file.parent().unwrap()),
        "create fuzz marker parent",
    );
    assert_io_ok(
        fs::write(&marker_file, "// foreign-function-boundary"),
        "write fuzz marker file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "foreign-function-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("fuzz/fuzz_target_1.rs"));
}

#[test]
fn discover_boundaries_finds_markers_in_scripts() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    let marker_file = temp_dir.path().join("scripts/build.sh");
    assert_io_ok(
        fs::create_dir_all(marker_file.parent().unwrap()),
        "create script marker parent",
    );
    assert_io_ok(
        fs::write(&marker_file, "# ipc-frame-boundary"),
        "write script marker file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "ipc-frame-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("scripts/build.sh"));
}

#[test]
fn discover_boundaries_returns_empty_on_no_markers() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    // Create files without markers
    let regular_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    assert_io_ok(
        fs::create_dir_all(regular_file.parent().unwrap()),
        "create regular file parent",
    );
    assert_io_ok(
        fs::write(&regular_file, "// regular code"),
        "write regular file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::IncompleteDiscoveryInput
    );
}

#[test]
fn discover_boundaries_detects_decoder_surface_omission() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    // Create boundary-surfaces.txt without decoder-byte-ingest-boundary
    assert_io_ok(
        fs::write(
            temp_dir.path().join("boundary-surfaces.txt"),
            "extern-c-boundary\nforeign-function-boundary\n",
        ),
        "write boundary surfaces",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::IncompleteDiscoveryInput
    );
}

#[test]
fn discover_boundaries_all_marker_types() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let markers = [
        ("extern-c-boundary", "crates/test/src/lib.rs"),
        ("foreign-function-boundary", "fuzz/fuzz_1.rs"),
        ("ipc-frame-boundary", "scripts/run.sh"),
        ("external-binary-boundary", "crates/vb_core/src/bin.rs"),
        ("decoder-byte-ingest-boundary", "crates/decoder/src/lib.rs"),
        ("generated-interface-boundary", "crates/gen/src/lib.rs"),
        (
            "unsafe-adjacent-dependency-boundary",
            "crates/unsafe_dep/src/lib.rs",
        ),
    ];

    for (marker, file_path) in &markers {
        let full_path = temp_dir.path().join(file_path);
        assert_io_ok(
            fs::create_dir_all(full_path.parent().unwrap()),
            "create marker file parent",
        );
        assert_io_ok(
            fs::write(&full_path, format!("// {marker}")),
            "write marker file",
        );
    }

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let mut actual: Vec<(String, PathBuf)> = discover_boundaries(workspace)
        .unwrap()
        .into_iter()
        .map(|candidate| (candidate.marker, candidate.source_path))
        .collect();
    actual.sort();

    let mut expected: Vec<(String, PathBuf)> = markers
        .iter()
        .map(|(marker, file_path)| ((*marker).to_string(), PathBuf::from(file_path)))
        .collect();
    expected.sort();

    assert_eq!(actual, expected);
}

// =============================================================================
// classify_boundary tests
// =============================================================================

#[test]
fn classify_boundary_c_abi() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::CAbi);
    assert_eq!(classified.id, "vb-y1zq-CAbi-crates-test-src-lib-rs");
}

#[test]
fn classify_boundary_ffi() {
    let candidate = BoundaryCandidate::new("fuzz/fuzz_1.rs", "foreign-function-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::Ffi);
}

#[test]
fn classify_boundary_ipc() {
    let candidate = BoundaryCandidate::new("scripts/run.sh", "ipc-frame-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::Ipc);
}

#[test]
fn classify_boundary_external_binary() {
    let candidate = BoundaryCandidate::new("crates/bin/src/main.rs", "external-binary-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::ExternalBinary);
}

#[test]
fn classify_boundary_decoder() {
    let candidate =
        BoundaryCandidate::new("crates/decoder/src/lib.rs", "decoder-byte-ingest-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::Decoder);
}

#[test]
fn classify_boundary_generated_code() {
    let candidate = BoundaryCandidate::new("crates/gen/src/lib.rs", "generated-interface-boundary");
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::GeneratedCode);
}

#[test]
fn classify_boundary_unsafe_adjacent() {
    let candidate = BoundaryCandidate::new(
        "crates/unsafe_dep/src/lib.rs",
        "unsafe-adjacent-dependency-boundary",
    );
    let result = classify_boundary(candidate);
    let classified = result.unwrap();
    assert_eq!(classified.class, BoundaryClass::UnsafeAdjacentDependency);
}

#[test]
fn classify_boundary_id_stability() {
    let candidate1 = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let candidate2 = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let classified1 = classify_boundary(candidate1).unwrap();
    let classified2 = classify_boundary(candidate2).unwrap();
    assert_eq!(classified1.id, classified2.id);
}

#[test]
fn classify_boundary_id_path_normalization() {
    let candidate1 = BoundaryCandidate::new("crates/test.src/lib.rs", "extern-c-boundary");
    let candidate2 = BoundaryCandidate::new("crates/test_src/lib.rs", "extern-c-boundary");
    let classified1 = classify_boundary(candidate1).unwrap();
    let classified2 = classify_boundary(candidate2).unwrap();
    assert_eq!(classified1.id, "vb-y1zq-CAbi-crates-test-src-lib-rs");
    assert_eq!(classified2.id, "vb-y1zq-CAbi-crates-test-src-lib-rs");
}

#[test]
fn classify_boundary_exposure_is_risky() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let classified = classify_boundary(candidate).unwrap();
    // Multiple risk classification
    assert_eq!(classified.exposure.risk, BoundaryRisk::Multiple);
}

// =============================================================================
// required_evidence tests
// =============================================================================

#[test]
fn required_evidence_unknown_class_error() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::Unknown,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    let result = required_evidence(classified);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn required_evidence_safe_boundary_error() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    let result = required_evidence(classified);
    // Safe boundary with no risk should not require evidence
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::MissingEvidencePath
    );
}

#[test]
fn required_evidence_risky_boundary_ok() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ExternalBytes),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_generated_code_risky() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::GeneratedCode,
        source_path: PathBuf::from("crates/gen/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_unsafe_adjacent_risky() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::UnsafeAdjacentDependency,
        source_path: PathBuf::from("crates/unsafe_dep/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

// =============================================================================
// validate_inventory tests
// =============================================================================

#[test]
fn validate_inventory_wrong_schema_version() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let inventory = BoundaryInventory::new(Some(99), Vec::new(), None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn validate_inventory_no_schema_version() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let inventory = BoundaryInventory::new(None, Vec::new(), None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn validate_inventory_duplicate_ids() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record = make_valid_record("same-id");
    let inventory = BoundaryInventory::new(Some(1), vec![record.clone(), record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::DuplicateBoundaryId
    );
}

#[test]
fn validate_inventory_unknown_class_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record = make_record_with_class(BoundaryClass::Unknown);
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn validate_inventory_missing_owner_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.owner = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingOwner);
}

#[test]
fn validate_inventory_missing_threat_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.threat = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingThreat);
}

#[test]
fn validate_inventory_missing_evidence_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::MissingEvidencePath
    );
}

#[test]
fn validate_inventory_stale_evidence_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.freshness = FreshnessMarker::new(2, 1, 1); // evidence_version < source_version
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::StaleEvidence);
}

#[test]
fn validate_inventory_valid_single_record() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record = make_valid_record("test-id");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.schema_version, 1);
    assert_eq!(validated.records.len(), 1);
    assert_eq!(validated.discovered_boundary_count, 1);
    assert_eq!(validated.review_status, Some("approved".to_string()));
}

#[test]
fn validate_inventory_valid_multiple_records() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record1 = make_valid_record("test-id-1");
    let record2 = make_valid_record("test-id-2");
    let inventory = BoundaryInventory::new(Some(1), vec![record1, record2], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.schema_version, 1);
    assert_eq!(validated.records.len(), 2);
    assert_eq!(validated.discovered_boundary_count, 2);
    assert_eq!(validated.review_status, Some("approved".to_string()));
}

// Additional tests for better coverage

#[test]
fn classify_boundary_preserves_source_path() {
    let path = "crates/test/src/lib.rs";
    let candidate = BoundaryCandidate::new(path, "extern-c-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.source_path, PathBuf::from(path));
}

#[test]
fn classify_boundary_generated_code_exposure() {
    let candidate = BoundaryCandidate::new("crates/gen/src/lib.rs", "generated-interface-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.exposure.risk, BoundaryRisk::Multiple);
}

#[test]
fn classify_boundary_decoder_exposure() {
    let candidate = BoundaryCandidate::new("crates/dec/src/lib.rs", "decoder-byte-ingest-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.exposure.risk, BoundaryRisk::Multiple);
}

#[test]
fn classify_boundary_external_binary_exposure() {
    let candidate = BoundaryCandidate::new("crates/bin/src/main.rs", "external-binary-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.exposure.risk, BoundaryRisk::Multiple);
}

#[test]
fn required_evidence_process_limit_boundary() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::Ffi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ProcessLimit),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_language_limit_boundary() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::Ffi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::LanguageLimit),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_external_bytes_boundary() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::Decoder,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ExternalBytes),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn validate_inventory_empty_records_valid() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let inventory = BoundaryInventory::new(Some(1), Vec::new(), None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.schema_version, 1);
    assert_eq!(validated.records.len(), 0);
    assert_eq!(validated.discovered_boundary_count, 0);
}

#[test]
fn inventory_completion_status_all_safe_boundaries() {
    let record1 = make_valid_record("test-id-1");
    let record2 = make_valid_record("test-id-2");
    let validated = ValidatedBoundaryInventory::from_records(vec![record1, record2]);

    match inventory_completion_status(validated).unwrap() {
        UnsafeIsolationStatus::Complete { boundary_count } => {
            assert_eq!(boundary_count, 2);
        }
    }
}

#[test]
fn classify_boundary_id_format() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.id, "vb-y1zq-CAbi-crates-test-src-lib-rs");
}

#[test]
fn classify_boundary_all_marker_types_have_risky_exposure() {
    let markers = [
        ("extern-c-boundary", BoundaryClass::CAbi),
        ("foreign-function-boundary", BoundaryClass::Ffi),
        ("ipc-frame-boundary", BoundaryClass::Ipc),
        ("external-binary-boundary", BoundaryClass::ExternalBinary),
        ("decoder-byte-ingest-boundary", BoundaryClass::Decoder),
        ("generated-interface-boundary", BoundaryClass::GeneratedCode),
        (
            "unsafe-adjacent-dependency-boundary",
            BoundaryClass::UnsafeAdjacentDependency,
        ),
    ];

    for (marker, _class) in markers {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker);
        let result = classify_boundary(candidate).unwrap();
        assert_eq!(result.class, _class);
        assert_eq!(result.exposure.risk, BoundaryRisk::Multiple);
    }
}

#[test]
fn required_evidence_safe_boundary_missing_path() {
    // Safe boundary with no risk should fail with MissingEvidencePath
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    let result = required_evidence(classified);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::MissingEvidencePath
    );
}

#[test]
fn validate_inventory_waived_requires_waiver() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    // Waived status without waiver should fail
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::ReviewStatusInvalid
    );
}

#[test]
fn validate_inventory_waived_with_waiver() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.review_status = FieldState::Present(ReviewStatus::Waived);
    record.waiver = FieldState::Present(EvidenceReference::ExternalProvenance(
        "external:vb-abc123#sha256=abc".to_string(),
    ));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.records.len(), 1);
    assert_eq!(validated.review_status, Some("waived".to_string()));
}

// =============================================================================
// inventory_completion_status tests
// =============================================================================

#[test]
fn inventory_completion_status_unknown_class_error() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    validated
        .records
        .push(make_record_with_class(BoundaryClass::Unknown));

    let result = inventory_completion_status(validated);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn inventory_completion_status_first_party_unsafe_error() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    let mut record = make_record_with_class(BoundaryClass::UnsafeAdjacentDependency);
    record.source_path = PathBuf::from("crates/vb_core/src/lib.rs"); // First party path
    validated.records.push(record);

    let result = inventory_completion_status(validated);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnsafeForbiddenViolation
    );
}

#[test]
fn inventory_completion_status_empty_with_nonzero_discovered_error() {
    let validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(5);

    let result = inventory_completion_status(validated);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::IncompleteDiscoveryInput
    );
}

#[test]
fn inventory_completion_status_complete_ok() {
    let record = make_valid_record("test-id");
    let validated = ValidatedBoundaryInventory::from_records(vec![record]);

    match inventory_completion_status(validated).unwrap() {
        UnsafeIsolationStatus::Complete { boundary_count } => {
            assert_eq!(boundary_count, 1);
        }
    }
}

#[test]
fn inventory_completion_status_third_party_unsafe_allowed() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    let mut record = make_record_with_class(BoundaryClass::UnsafeAdjacentDependency);
    record.source_path = PathBuf::from("fuzz/unsafe_dep/src/lib.rs"); // Third party path
    validated.records.push(record);

    assert_eq!(
        inventory_completion_status(validated).unwrap(),
        UnsafeIsolationStatus::Complete { boundary_count: 1 }
    );
}

// =============================================================================
// Helper functions
// =============================================================================

fn test_tempdir() -> tempfile::TempDir {
    let temp_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/boundary-inventory-tmp");
    assert_io_ok(fs::create_dir_all(&temp_root), "create test temp root");
    let dir = tempfile::Builder::new()
        .prefix("boundary-inventory-")
        .tempdir_in(&temp_root);
    assert!(dir.is_ok(), "tempdir succeeds: {dir:?}");
    match dir {
        Ok(dir) => dir,
        Err(_) => std::process::abort(),
    }
}

fn create_valid_workspace(path: &std::path::Path) {
    assert_io_ok(fs::create_dir(path.join("crates")), "create crates dir");
    assert_io_ok(fs::create_dir(path.join("fuzz")), "create fuzz dir");
    assert_io_ok(fs::create_dir(path.join("scripts")), "create scripts dir");
    assert_io_ok(fs::write(path.join("Cargo.toml"), ""), "write Cargo.toml");
    // Create evidence file that validation expects
    assert_io_ok(fs::create_dir_all(path.join("fuzz")), "ensure fuzz dir");
    assert_io_ok(
        fs::write(path.join("fuzz/test.rs"), ""),
        "write fuzz evidence",
    );
}

fn make_valid_record(id: &str) -> BoundaryRecord {
    BoundaryRecordDraft::new(BoundaryRecordParts {
        id: id.to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        owner: FieldState::Present(Owner("test-owner".to_string())),
        threat: FieldState::Present(ThreatStatement("test-threat".to_string())),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("fuzz/test.rs"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}

fn make_record_with_class(class: BoundaryClass) -> BoundaryRecord {
    BoundaryRecordDraft::new(BoundaryRecordParts {
        id: "test-id".to_string(),
        class,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        owner: FieldState::Present(Owner("test-owner".to_string())),
        threat: FieldState::Present(ThreatStatement("test-threat".to_string())),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("fuzz/test.rs"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}

#[test]
fn classify_boundary_stability_idempotent() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let result1 = classify_boundary(candidate.clone()).unwrap();
    let result2 = classify_boundary(candidate).unwrap();
    assert_eq!(result1.id, result2.id);
    assert_eq!(result1.class, result2.class);
    assert_eq!(result1.source_path, result2.source_path);
    assert_eq!(result1.exposure.risk, result2.exposure.risk);
}

// =============================================================================
// discover_boundaries tests — additional edge cases
// =============================================================================

#[test]
fn discover_boundaries_decoder_surface_present_continues() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    // boundary-surfaces.txt WITH decoder-byte-ingest-boundary
    assert_io_ok(
        fs::write(
            temp_dir.path().join("boundary-surfaces.txt"),
            "extern-c-boundary\ndecoder-byte-ingest-boundary\n",
        ),
        "write surfaces with decoder entry",
    );
    let marker_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    assert_io_ok(
        fs::create_dir_all(marker_file.parent().unwrap()),
        "create marker parent",
    );
    assert_io_ok(
        fs::write(&marker_file, "// extern-c-boundary"),
        "write marker file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "extern-c-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("crates/vb_core/src/lib.rs"));
}

#[test]
fn discover_boundaries_finds_marker_in_cargo_toml() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    assert_io_ok(
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "# foreign-function-boundary\n",
        ),
        "write Cargo.toml with marker",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "foreign-function-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("Cargo.toml"));
}

#[test]
fn discover_boundaries_finds_markers_in_nested_subdirectories() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    let deep_path = temp_dir
        .path()
        .join("crates/vb_core/src/boundary_inventory/deep/lib.rs");
    assert_io_ok(
        fs::create_dir_all(deep_path.parent().unwrap()),
        "create deep dirs",
    );
    assert_io_ok(
        fs::write(&deep_path, "// ipc-frame-boundary\n"),
        "write deep marker file",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "ipc-frame-boundary");
    assert_eq!(
        candidates[0].source_path,
        PathBuf::from("crates/vb_core/src/boundary_inventory/deep/lib.rs")
    );
}

#[test]
fn discover_boundaries_workspace_with_no_files_still_fails_on_no_markers() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::IncompleteDiscoveryInput
    );
}

// =============================================================================
// classify_boundary tests — additional edge cases
// =============================================================================

#[test]
fn classify_boundary_rejects_empty_marker() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "");
    let result = classify_boundary(candidate);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn classify_boundary_rejects_unrecognized_marker() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "nonexistent-marker-xyz");
    let result = classify_boundary(candidate);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn classify_boundary_external_binary_marker_to_correct_class() {
    let candidate = BoundaryCandidate::new("crates/bin/src/app.rs", "external-binary-boundary");
    let classified = classify_boundary(candidate).unwrap();
    assert_eq!(classified.class, BoundaryClass::ExternalBinary);
    assert_eq!(classified.id, "vb-y1zq-ExternalBinary-crates-bin-src-app-rs");
}

#[test]
fn classify_boundary_id_path_contains_name_of_class() {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let result = classify_boundary(candidate).unwrap();
    assert_eq!(result.id, "vb-y1zq-CAbi-crates-test-src-lib-rs");
}

// =============================================================================
// required_evidence tests — additional edge cases
// =============================================================================

#[test]
fn required_evidence_risky_by_process_limit() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::Ipc,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ProcessLimit),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_risky_by_language_limit() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::ExternalBinary,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::LanguageLimit),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_safe_class_but_risky_exposure() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ExternalBytes),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

#[test]
fn required_evidence_risky_class_but_none_risk() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::GeneratedCode,
        source_path: PathBuf::from("crates/gen/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    assert_eq!(
        required_evidence(classified).unwrap(),
        EvidenceRequirement::FuzzOrIsolationOrManualQa
    );
}

// =============================================================================
// validate_inventory tests — additional edge cases
// =============================================================================

#[test]
fn validate_inventory_second_record_has_missing_owner() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record1 = make_valid_record("test-id-1");
    let mut record2 = make_valid_record("test-id-2");
    record2.owner = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record1, record2], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingOwner);
}

#[test]
fn validate_inventory_empty_owner_string_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.owner = FieldState::Present(Owner(String::new()));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingOwner);
}

#[test]
fn validate_inventory_empty_threat_string_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.threat = FieldState::Present(ThreatStatement(String::new()));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingThreat);
}

#[test]
fn validate_inventory_source_path_wrong_prefix_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.source_path = PathBuf::from("external/vendor/src/lib.rs");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::WorkspaceNotDiscoverable
    );
}

#[test]
fn validate_inventory_nonexistent_evidence_file_error() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Present(EvidenceReference::repo_local(
        PathBuf::from("fuzz/nonexistent_file.rs"),
        EvidenceKind::Fuzz,
    ));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InvalidEvidencePath
    );
}

#[test]
fn validate_inventory_free_text_evidence_rejected() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Present(EvidenceReference::free_text("some free text"));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InvalidEvidencePath
    );
}

#[test]
fn validate_inventory_review_status_other_rejected() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.review_status = FieldState::Present(ReviewStatus::Other("pending".to_string()));
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::ReviewStatusInvalid
    );
}

#[test]
fn validate_inventory_review_status_missing_rejected() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.review_status = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::ReviewStatusInvalid
    );
}

#[test]
fn validate_inventory_stale_evidence_schema_ahead_of_evidence() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.freshness = FreshnessMarker::new(1, 3, 1); // evidence < schema
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::StaleEvidence);
}

#[test]
fn validate_inventory_third_record_missing_threat() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let record1 = make_valid_record("test-id-1");
    let record2 = make_valid_record("test-id-2");
    let mut record3 = make_valid_record("test-id-3");
    record3.threat = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record1, record2, record3], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingThreat);
}

#[test]
fn validate_inventory_approved_with_external_evidence_ok() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(
        "vb-abc123".to_string(),
    ));
    let expected_evidence = record.evidence.clone();
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.records.len(), 1);
    assert_eq!(validated.records[0].evidence, expected_evidence);
}

#[test]
fn validate_inventory_approved_with_sha256_external_evidence_ok() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Present(EvidenceReference::ExternalProvenance(
        "external:vb-abc123#sha256=abcdef1234567890".to_string(),
    ));
    let expected_evidence = record.evidence.clone();
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let validated = validate_inventory(inventory, workspace).unwrap();
    assert_eq!(validated.records.len(), 1);
    assert_eq!(validated.records[0].evidence, expected_evidence);
}

// =============================================================================
// inventory_completion_status tests — additional edge cases
// =============================================================================

#[test]
fn inventory_completion_status_empty_records_zero_discovered() {
    let validated = ValidatedBoundaryInventory::from_records(Vec::new());
    match inventory_completion_status(validated).unwrap() {
        UnsafeIsolationStatus::Complete { boundary_count } => {
            assert_eq!(boundary_count, 0);
        }
    }
}

#[test]
fn inventory_completion_status_single_valid_record() {
    let record = make_valid_record("test-id");
    let validated = ValidatedBoundaryInventory::from_records(vec![record]);
    match inventory_completion_status(validated).unwrap() {
        UnsafeIsolationStatus::Complete { boundary_count } => {
            assert_eq!(boundary_count, 1);
        }
    }
}

#[test]
fn inventory_completion_status_third_party_unsafe_adjacent_in_fuzz_allowed() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    let mut record = make_record_with_class(BoundaryClass::UnsafeAdjacentDependency);
    record.source_path = PathBuf::from("fuzz/vendor_sdk/src/lib.rs");
    validated.records.push(record);
    assert_eq!(
        inventory_completion_status(validated).unwrap(),
        UnsafeIsolationStatus::Complete { boundary_count: 1 }
    );
}

#[test]
fn inventory_completion_status_third_party_unsafe_adjacent_in_scripts_allowed() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    let mut record = make_record_with_class(BoundaryClass::UnsafeAdjacentDependency);
    record.source_path = PathBuf::from("scripts/vendor_tool.sh");
    validated.records.push(record);
    assert_eq!(
        inventory_completion_status(validated).unwrap(),
        UnsafeIsolationStatus::Complete { boundary_count: 1 }
    );
}

// =============================================================================
// discover_boundaries tests — surface file edge cases
// =============================================================================

#[test]
fn discover_boundaries_with_surfaces_file_containing_decoder_entry() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    assert_io_ok(
        fs::write(
            temp_dir.path().join("boundary-surfaces.txt"),
            "extern-c-boundary\ndecoder-byte-ingest-boundary\n",
        ),
        "write surfaces file with decoder marker",
    );
    let marker_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    assert_io_ok(
        fs::create_dir_all(marker_file.parent().unwrap()),
        "create parent",
    );
    assert_io_ok(
        fs::write(&marker_file, "// decoder-byte-ingest-boundary"),
        "write marker",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let candidates = discover_boundaries(workspace).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].marker, "decoder-byte-ingest-boundary");
    assert_eq!(candidates[0].source_path, PathBuf::from("crates/vb_core/src/lib.rs"));
}

#[test]
fn discover_boundaries_with_surfaces_file_missing_decoder_line() {
    let temp_dir = test_tempdir();
    create_valid_workspace(temp_dir.path());
    assert_io_ok(
        fs::write(
            temp_dir.path().join("boundary-surfaces.txt"),
            "extern-c-boundary\nforeign-function-boundary\n",
        ),
        "write surfaces file without decoder",
    );

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::IncompleteDiscoveryInput
    );
}
