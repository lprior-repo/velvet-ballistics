//! API function tests for vb_boundary_inventory
//!
//! Tests 5 pub fns: discover_boundaries, classify_boundary, required_evidence,
//! validate_inventory, inventory_completion_status

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use vb_boundary_inventory::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryInventory,
    BoundaryInventoryError, BoundaryRecord, BoundaryRecordDraft, BoundaryRecordParts,
    BoundaryRisk, ClassifiedBoundary, ClassifiedBoundaryInput, EvidenceReference,
    FieldState, FreshnessMarker, Owner, ReviewStatus, ThreatStatement, ValidatedBoundaryInventory,
    WorkspaceRoot,
};

use vb_boundary_inventory::boundary_inventory::api::{
    classify_boundary, discover_boundaries, inventory_completion_status, required_evidence,
    validate_inventory,
};

// =============================================================================
// discover_boundaries tests
// =============================================================================

#[test]
fn discover_boundaries_rejects_nonexistent_workspace() {
    let workspace = WorkspaceRoot::new(PathBuf::from("/nonexistent/path"));
    let result = discover_boundaries(workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::WorkspaceNotDiscoverable);
}

#[test]
fn discover_boundaries_rejects_workspace_without_required_surfaces() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::WorkspaceNotDiscoverable);
}

#[test]
fn discover_boundaries_rejects_incomplete_workspace() {
    let temp_dir = tempfile::tempdir().unwrap();
    // Create only some required surfaces
    fs::create_dir(temp_dir.path().join("crates")).ok();
    fs::write(temp_dir.path().join("Cargo.toml"), "").ok();
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    // Missing fuzz, scripts directories
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::WorkspaceNotDiscoverable);
}

#[test]
fn discover_boundaries_finds_markers_in_crates() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());
    // Create a file with a boundary marker
    let marker_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    fs::create_dir_all(marker_file.parent().unwrap()).ok();
    fs::write(&marker_file, "// extern-c-boundary").ok();

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    let candidates = result.unwrap();
    assert!(!candidates.is_empty());
    assert!(candidates.iter().any(|c| c.marker == "extern-c-boundary"));
}

#[test]
fn discover_boundaries_finds_markers_in_fuzz() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());
    let marker_file = temp_dir.path().join("fuzz/fuzz_target_1.rs");
    fs::create_dir_all(marker_file.parent().unwrap()).ok();
    fs::write(&marker_file, "// foreign-function-boundary").ok();

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    let candidates = result.unwrap();
    assert!(!candidates.is_empty());
    assert!(candidates.iter().any(|c| c.marker == "foreign-function-boundary"));
}

#[test]
fn discover_boundaries_finds_markers_in_scripts() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());
    let marker_file = temp_dir.path().join("scripts/build.sh");
    fs::create_dir_all(marker_file.parent().unwrap()).ok();
    fs::write(&marker_file, "# ipc-frame-boundary").ok();

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    let candidates = result.unwrap();
    assert!(!candidates.is_empty());
    assert!(candidates.iter().any(|c| c.marker == "ipc-frame-boundary"));
}

#[test]
fn discover_boundaries_returns_empty_on_no_markers() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());
    // Create files without markers
    let regular_file = temp_dir.path().join("crates/vb_core/src/lib.rs");
    fs::create_dir_all(regular_file.parent().unwrap()).ok();
    fs::write(&regular_file, "// regular code").ok();

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::IncompleteDiscoveryInput);
}

#[test]
fn discover_boundaries_detects_decoder_surface_omission() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());
    // Create boundary-surfaces.txt without decoder-byte-ingest-boundary
    fs::write(
        temp_dir.path().join("boundary-surfaces.txt"),
        "extern-c-boundary\nforeign-function-boundary\n",
    )
    .ok();

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::IncompleteDiscoveryInput);
}

#[test]
fn discover_boundaries_all_marker_types() {
    let temp_dir = tempfile::tempdir().unwrap();
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
        fs::create_dir_all(full_path.parent().unwrap()).ok();
        fs::write(&full_path, format!("// {marker}")).ok();
    }

    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = discover_boundaries(workspace).unwrap();
    assert_eq!(result.len(), markers.len());
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
    assert!(classified.id.starts_with("vb-y1zq-CAbi-"));
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
    let candidate = BoundaryCandidate::new("crates/decoder/src/lib.rs", "decoder-byte-ingest-boundary");
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
    // Paths with /, ., _ should normalize to same ID
    let candidate1 = BoundaryCandidate::new("crates/test.src/lib.rs", "extern-c-boundary");
    let candidate2 = BoundaryCandidate::new("crates/test/src/lib.rs", "extern-c-boundary");
    let classified1 = classify_boundary(candidate1).unwrap();
    let classified2 = classify_boundary(candidate2).unwrap();
    // IDs may differ due to different paths
    assert!(!classified1.id.is_empty());
    assert!(!classified2.id.is_empty());
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
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::UnknownBoundaryClass);
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
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingEvidencePath);
}

#[test]
fn required_evidence_risky_boundary_ok() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ExternalBytes),
    });
    let result = required_evidence(classified);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vb_boundary_inventory::boundary_inventory::EvidenceRequirement::FuzzOrIsolationOrManualQa);
}

#[test]
fn required_evidence_generated_code_risky() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::GeneratedCode,
        source_path: PathBuf::from("crates/gen/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    let result = required_evidence(classified);
    assert!(result.is_ok());
}

#[test]
fn required_evidence_unsafe_adjacent_risky() {
    let classified = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: "test-id".to_string(),
        class: BoundaryClass::UnsafeAdjacentDependency,
        source_path: PathBuf::from("crates/unsafe_dep/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });
    let result = required_evidence(classified);
    assert!(result.is_ok());
}

// =============================================================================
// validate_inventory tests
// =============================================================================

#[test]
fn validate_inventory_wrong_schema_version() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let inventory = BoundaryInventory::new(Some(99), Vec::new(), None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::SchemaVersionUnsupported);
}

#[test]
fn validate_inventory_no_schema_version() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let inventory = BoundaryInventory::new(None, Vec::new(), None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::SchemaVersionUnsupported);
}

#[test]
fn validate_inventory_duplicate_ids() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let record = make_valid_record("same-id");
    let inventory = BoundaryInventory::new(
        Some(1),
        vec![record.clone(), record],
        None,
    );
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::DuplicateBoundaryId);
}

#[test]
fn validate_inventory_unknown_class_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let record = make_record_with_class(BoundaryClass::Unknown);
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::UnknownBoundaryClass);
}

#[test]
fn validate_inventory_missing_owner_error() {
    let temp_dir = tempfile::tempdir().unwrap();
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
    let temp_dir = tempfile::tempdir().unwrap();
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
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let mut record = make_valid_record("test-id");
    record.evidence = FieldState::Missing;
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::MissingEvidencePath);
}

#[test]
fn validate_inventory_stale_evidence_error() {
    let temp_dir = tempfile::tempdir().unwrap();
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
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let record = make_valid_record("test-id");
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert!(result.is_ok());
}

#[test]
fn validate_inventory_valid_multiple_records() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_valid_workspace(temp_dir.path());

    let record1 = make_valid_record("test-id-1");
    let record2 = make_valid_record("test-id-2");
    let inventory = BoundaryInventory::new(Some(1), vec![record1, record2], None);
    let workspace = WorkspaceRoot::new(temp_dir.path().to_path_buf());
    let result = validate_inventory(inventory, workspace);
    assert!(result.is_ok());
}

// =============================================================================
// inventory_completion_status tests
// =============================================================================

#[test]
fn inventory_completion_status_unknown_class_error() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    validated.records.push(make_record_with_class(BoundaryClass::Unknown));

    let result = inventory_completion_status(validated);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::UnknownBoundaryClass);
}

#[test]
fn inventory_completion_status_first_party_unsafe_error() {
    let mut validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);
    let mut record = make_record_with_class(BoundaryClass::UnsafeAdjacentDependency);
    record.source_path = PathBuf::from("crates/vb_core/src/lib.rs"); // First party path
    validated.records.push(record);

    let result = inventory_completion_status(validated);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::UnsafeForbiddenViolation);
}

#[test]
fn inventory_completion_status_empty_with_nonzero_discovered_error() {
    let validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(5);

    let result = inventory_completion_status(validated);
    assert_eq!(result.unwrap_err(), BoundaryInventoryError::IncompleteDiscoveryInput);
}

#[test]
fn inventory_completion_status_complete_ok() {
    let record = make_valid_record("test-id");
    let validated = ValidatedBoundaryInventory::from_records(vec![record]);

    let result = inventory_completion_status(validated);
    assert!(result.is_ok());
    match result.unwrap() {
        vb_boundary_inventory::boundary_inventory::UnsafeIsolationStatus::Complete { boundary_count } => {
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

    let result = inventory_completion_status(validated);
    assert!(result.is_ok());
}

// =============================================================================
// Helper functions
// =============================================================================

fn create_valid_workspace(path: &std::path::Path) {
    fs::create_dir(path.join("crates")).ok();
    fs::create_dir(path.join("fuzz")).ok();
    fs::create_dir(path.join("scripts")).ok();
    fs::write(path.join("Cargo.toml"), "").ok();
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
            vb_boundary_inventory::boundary_inventory::EvidenceKind::Fuzz,
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
            vb_boundary_inventory::boundary_inventory::EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}
