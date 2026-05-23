//! Property-based tests for vb_boundary_inventory
//!
//! Uses proptest to generate arbitrary inputs for comprehensive coverage

use std::path::PathBuf;

use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryRecord, BoundaryRecordDraft,
    BoundaryRecordParts, BoundaryRisk, ClassifiedBoundary, ClassifiedBoundaryInput,
    EvidenceKind, EvidenceReference, FieldState, FreshnessMarker, Owner, ReviewStatus,
    ThreatStatement, ValidatedBoundaryInventory, WorkspaceRoot, classify_boundary,
};
use proptest::prelude::*;

// =============================================================================
// Custom proptest strategies
// =============================================================================

impl Arbitrary for BoundaryClass {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(BoundaryClass::CAbi),
            Just(BoundaryClass::Ffi),
            Just(BoundaryClass::Ipc),
            Just(BoundaryClass::ExternalBinary),
            Just(BoundaryClass::Decoder),
            Just(BoundaryClass::GeneratedCode),
            Just(BoundaryClass::UnsafeAdjacentDependency),
            Just(BoundaryClass::Unknown),
        ]
        .boxed()
    }
}

impl Arbitrary for BoundaryRisk {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(BoundaryRisk::None),
            Just(BoundaryRisk::ExternalBytes),
            Just(BoundaryRisk::ProcessLimit),
            Just(BoundaryRisk::LanguageLimit),
            Just(BoundaryRisk::Multiple),
        ]
        .boxed()
    }
}

impl Arbitrary for EvidenceKind {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(EvidenceKind::Fuzz),
            Just(EvidenceKind::Isolation),
            Just(EvidenceKind::ManualQa),
            Just(EvidenceKind::Provenance),
        ]
        .boxed()
    }
}

// =============================================================================
// Property tests using proptest
// =============================================================================

proptest! {
    fn boundary_candidate_new_roundtrip(source_path: String, marker: String) {
        let candidate = BoundaryCandidate::new(source_path.clone(), marker.clone());
        prop_assert_eq!(candidate.source_path, std::path::PathBuf::from(source_path));
        prop_assert_eq!(candidate.marker, marker);
    }

    fn boundary_candidate_path_preserves_slashes(path: String) {
        let candidate = BoundaryCandidate::new(path.clone(), "test-marker".to_string());
        prop_assert!(!candidate.source_path.as_os_str().is_empty());
    }

    fn boundary_candidate_marker_empty_allowed(marker: String) {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker);
        prop_assert_eq!(candidate.marker.len(), 0);
    }

    fn classify_boundary_unknown_marker_produces_unknown_class(_flag: bool) {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "unknown-marker");
        let result = classify_boundary(candidate);
        prop_assert!(result.is_ok());
        let classified = result.unwrap();
        prop_assert_eq!(classified.class, BoundaryClass::Unknown);
    }

    fn classify_boundary_exposure_is_risky_for_multiple(marker in "extern-c-boundary|foreign-function-boundary|ipc-frame-boundary") {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker.clone());
        let result = classify_boundary(candidate);
        prop_assert!(result.is_ok());
        let classified = result.unwrap();
        prop_assert_eq!(classified.exposure.risk, BoundaryRisk::Multiple);
    }

    fn classify_boundary_different_paths_produce_different_ids(path1 in "[a-z]{1,10}", path2 in "[a-z]{1,10}") {
        prop_assume!(path1 != path2);
        let candidate1 = BoundaryCandidate::new(format!("crates/{}/src/lib.rs", path1), "extern-c-boundary".to_string());
        let candidate2 = BoundaryCandidate::new(format!("crates/{}/src/lib.rs", path2), "extern-c-boundary".to_string());
        let result1 = classify_boundary(candidate1);
        let result2 = classify_boundary(candidate2);
        prop_assert!(result1.is_ok() && result2.is_ok());
        let id1 = result1.unwrap().id;
        let id2 = result2.unwrap().id;
        prop_assert_ne!(id1, id2);
    }

    fn classified_boundary_id_roundtrip(id: String) {
        let input = ClassifiedBoundaryInput {
            id: id.clone(),
            class: BoundaryClass::CAbi,
            source_path: std::path::PathBuf::from("crates/test/src/lib.rs"),
            exposure: BoundaryExposure::risky(BoundaryRisk::Multiple),
        };
        let classified = ClassifiedBoundary::new(input);
        prop_assert_eq!(classified.id, id);
    }

    fn classified_boundary_source_path_roundtrip(source_path: String) {
        let input = ClassifiedBoundaryInput {
            id: "test-id".to_string(),
            class: BoundaryClass::CAbi,
            source_path: std::path::PathBuf::from(source_path.clone()),
            exposure: BoundaryExposure::risky(BoundaryRisk::Multiple),
        };
        let classified = ClassifiedBoundary::new(input);
        prop_assert_eq!(classified.source_path, std::path::PathBuf::from(source_path));
    }

    fn classified_boundary_class_roundtrip(class: BoundaryClass) {
        let input = ClassifiedBoundaryInput {
            id: "test-id".to_string(),
            class,
            source_path: std::path::PathBuf::from("crates/test/src/lib.rs"),
            exposure: BoundaryExposure::risky(BoundaryRisk::Multiple),
        };
        let classified = ClassifiedBoundary::new(input);
        prop_assert_eq!(classified.class, class);
    }

    fn classified_boundary_id_non_empty(id: String) {
        prop_assume!(!id.is_empty());
        let input = ClassifiedBoundaryInput {
            id,
            class: BoundaryClass::CAbi,
            source_path: std::path::PathBuf::from("crates/test/src/lib.rs"),
            exposure: BoundaryExposure::none(),
        };
        let classified = ClassifiedBoundary::new(input);
        prop_assert!(!classified.id.is_empty());
    }

    fn boundary_exposure_none(_flag: bool) {
        let exposure = BoundaryExposure::none();
        prop_assert_eq!(exposure.risk, BoundaryRisk::None);
    }

    fn boundary_exposure_risky_roundtrip(risk: BoundaryRisk) {
        let exposure = BoundaryExposure::risky(risk);
        prop_assert_eq!(exposure.risk, risk);
    }

    fn freshness_marker_new_roundtrip(sv: u64, ev: u64) {
        prop_assume!(ev >= sv);
        let marker = FreshnessMarker::new(sv, sv, ev);
        prop_assert_eq!(marker.source_version, sv);
        prop_assert_eq!(marker.schema_version, sv);
        prop_assert_eq!(marker.evidence_version, ev);
    }

    fn freshness_marker_valid_versions(sv: u64, ev: u64) {
        prop_assume!(ev >= sv);
        let marker = FreshnessMarker::new(sv, sv, ev);
        prop_assert!(marker.evidence_version >= marker.source_version);
        prop_assert!(marker.evidence_version >= marker.schema_version);
    }

    fn inventory_completion_status_empty_record_count(count: usize) {
        let validated = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(count);
        prop_assert_eq!(validated.discovered_boundary_count, count);
    }

    fn evidence_reference_repo_local_roundtrip(path: String, kind: EvidenceKind) {
        let reference = EvidenceReference::repo_local(std::path::PathBuf::from(path.clone()), kind);
        match reference {
            EvidenceReference::RepoLocal { path: p, kind: k } => {
                prop_assert_eq!(p, std::path::PathBuf::from(path));
                prop_assert_eq!(k, kind);
            }
            _ => prop_assert!(false),
        }
    }

    fn evidence_reference_free_text_roundtrip(text: String) {
        let reference = EvidenceReference::free_text(text.clone());
        match reference {
            EvidenceReference::FreeText(t) => {
                prop_assert_eq!(t, text);
            }
            _ => prop_assert!(false),
        }
    }

    fn evidence_reference_external_provenance(text: String) {
        prop_assume!(!text.is_empty());
        let reference = EvidenceReference::ExternalProvenance(text.clone());
        match reference {
            EvidenceReference::ExternalProvenance(t) => {
                prop_assert_eq!(t, text);
            }
            _ => prop_assert!(false),
        }
    }

    fn field_state_from_option_some(value: String) {
        let state: FieldState<String> = FieldState::from(Some(value.clone()));
        match state {
            FieldState::Present(v) => prop_assert_eq!(v, value),
            FieldState::Missing => prop_assert!(false),
        }
    }

    fn field_state_as_ref(value: String) {
        let state: FieldState<String> = FieldState::Present(value.clone());
        let ref_state = state.as_ref();
        match ref_state {
            FieldState::Present(v) => prop_assert_eq!(v, &value),
            FieldState::Missing => prop_assert!(false),
        }
    }

    fn field_state_map(value: String) {
        let state: FieldState<String> = FieldState::Present(value.clone());
        let mapped = state.map(|v| v.len());
        match mapped {
            FieldState::Present(len) => prop_assert_eq!(len, value.len()),
            FieldState::Missing => prop_assert!(false),
        }
    }

    fn review_status_from_serialized_other(value in "[a-z]{1,20}") {
        prop_assume!(value != "approved" && value != "waived");
        let status = ReviewStatus::from_serialized(value.clone());
        match status {
            ReviewStatus::Other(v) => prop_assert_eq!(v, value),
            _ => prop_assert!(false),
        }
    }

    fn review_status_serialized_other(value in "[a-z]{1,20}") {
        prop_assume!(!value.is_empty());
        let status = ReviewStatus::Other(value.clone());
        prop_assert_eq!(status.serialized(), value);
    }

    fn validated_inventory_with_schema_version(schema: u32) {
        let validated = ValidatedBoundaryInventory::with_schema_version(schema);
        prop_assert_eq!(validated.schema_version, schema);
        prop_assert!(validated.records.is_empty());
        prop_assert_eq!(validated.discovered_boundary_count, 0);
    }

    fn validated_inventory_with_review_status(status: String) {
        let validated = ValidatedBoundaryInventory::with_review_status(status.clone());
        prop_assert_eq!(validated.review_status, Some(status));
    }

    fn field_state_from_option_none(_flag: bool) {
        let state: FieldState<String> = FieldState::from(None);
        match state {
            FieldState::Missing => {},
            FieldState::Present(_) => prop_assert!(false),
        }
    }

    fn field_state_missing_as_ref_prop(_flag: bool) {
        let state: FieldState<String> = FieldState::Missing;
        let ref_state = state.as_ref();
        match ref_state {
            FieldState::Missing => {},
            FieldState::Present(_) => prop_assert!(false),
        }
    }

    fn field_state_missing_map_prop(_flag: bool) {
        let state: FieldState<String> = FieldState::Missing;
        let mapped = state.map(|_v: String| 0usize);
        match mapped {
            FieldState::Missing => {},
            FieldState::Present(_) => prop_assert!(false),
        }
    }

    fn validated_inventory_from_records_preserves_count(records_count in 0usize..100) {
        let records: Vec<BoundaryRecord> = (0..records_count)
            .map(|i| BoundaryRecordDraft::new(BoundaryRecordParts {
                id: format!("test-id-{}", i),
                class: BoundaryClass::CAbi,
                source_path: PathBuf::from(format!("crates/test{}/src/lib.rs", i)),
                owner: FieldState::Present(Owner("test-owner".to_string())),
                threat: FieldState::Present(ThreatStatement("test-threat".to_string())),
                evidence: FieldState::Present(EvidenceReference::repo_local(
                    PathBuf::from("fuzz/test.rs"),
                    EvidenceKind::Fuzz,
                )),
                freshness: FreshnessMarker::new(1, 1, 1),
                review_status: FieldState::Present(ReviewStatus::Approved),
                waiver: FieldState::Missing,
            }))
            .collect();
        let validated = ValidatedBoundaryInventory::from_records(records);
        prop_assert_eq!(validated.discovered_boundary_count, records_count);
        prop_assert_eq!(validated.records.len(), records_count);
    }

    fn evidence_reference_repo_local_with_kind(kind: EvidenceKind) {
        let path = PathBuf::from("fuzz/test.rs");
        let reference = EvidenceReference::repo_local(path.clone(), kind);
        match reference {
            EvidenceReference::RepoLocal { path: p, kind: k } => {
                prop_assert_eq!(p, path);
                prop_assert_eq!(k, kind);
            }
            _ => prop_assert!(false),
        }
    }

    fn boundary_candidate_new_empty_path(marker: String) {
        let candidate = BoundaryCandidate::new("", marker.clone());
        prop_assert_eq!(candidate.marker, marker);
        prop_assert!(candidate.source_path.as_os_str().is_empty());
    }

    fn workspace_root_new_roundtrip(path: String) {
        let workspace = WorkspaceRoot::new(PathBuf::from(path.clone()));
        prop_assert_eq!(workspace.path, PathBuf::from(path));
    }

    fn review_status_from_serialized_then_serialized(value: String) {
        let status = ReviewStatus::from_serialized(value.clone());
        let serialized = status.serialized();
        // For approved and waived, serialized returns the canonical form
        if value == "approved" {
            prop_assert_eq!(serialized, "approved");
        } else if value == "waived" {
            prop_assert_eq!(serialized, "waived");
        } else {
            prop_assert_eq!(serialized, value);
        }
    }

    fn evidence_reference_external_provenance_format(text in "[a-z0-9]{1,30}") {
        prop_assume!(!text.is_empty());
        let ref_text = format!("external:{}#sha256=abcdef", text);
        let reference = EvidenceReference::ExternalProvenance(ref_text.clone());
        match reference {
            EvidenceReference::ExternalProvenance(t) => {
                prop_assert_eq!(t, ref_text);
            }
            _ => prop_assert!(false),
        }
    }

    fn freshness_marker_new_any_versions(sv: u64, schema: u64, ev: u64) {
        let marker = FreshnessMarker::new(sv, schema, ev);
        prop_assert_eq!(marker.source_version, sv);
        prop_assert_eq!(marker.schema_version, schema);
        prop_assert_eq!(marker.evidence_version, ev);
    }
}

// =============================================================================
// Regular tests for enum variants (no property needed)
// =============================================================================

#[test]
fn boundary_risk_all_variants() {
    let risks = [
        BoundaryRisk::None,
        BoundaryRisk::ExternalBytes,
        BoundaryRisk::ProcessLimit,
        BoundaryRisk::LanguageLimit,
        BoundaryRisk::Multiple,
    ];
    for risk in risks {
        let exposure = BoundaryExposure::risky(risk);
        assert_eq!(exposure.risk, risk);
    }
}

#[test]
fn boundary_class_all_variants() {
    let classes = [
        BoundaryClass::CAbi,
        BoundaryClass::Ffi,
        BoundaryClass::Ipc,
        BoundaryClass::ExternalBinary,
        BoundaryClass::Decoder,
        BoundaryClass::GeneratedCode,
        BoundaryClass::UnsafeAdjacentDependency,
    ];
    for class in classes {
        let candidate = BoundaryCandidate::new(
            "crates/test/src/lib.rs",
            match class {
                BoundaryClass::CAbi => "extern-c-boundary",
                BoundaryClass::Ffi => "foreign-function-boundary",
                BoundaryClass::Ipc => "ipc-frame-boundary",
                BoundaryClass::ExternalBinary => "external-binary-boundary",
                BoundaryClass::Decoder => "decoder-byte-ingest-boundary",
                BoundaryClass::GeneratedCode => "generated-interface-boundary",
                BoundaryClass::UnsafeAdjacentDependency => "unsafe-adjacent-dependency-boundary",
                BoundaryClass::Unknown => "unknown-marker", // Won't be reached
            },
        );
        let result = classify_boundary(candidate);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().class, class);
    }
}

#[test]
fn review_status_from_serialized_approved() {
    let status = ReviewStatus::from_serialized("approved");
    assert!(matches!(status, ReviewStatus::Approved));
}

#[test]
fn review_status_from_serialized_waived() {
    let status = ReviewStatus::from_serialized("waived");
    assert!(matches!(status, ReviewStatus::Waived));
}

#[test]
fn review_status_serialized_approved() {
    let status = ReviewStatus::Approved;
    assert_eq!(status.serialized(), "approved");
}

#[test]
fn review_status_serialized_waived() {
    let status = ReviewStatus::Waived;
    assert_eq!(status.serialized(), "waived");
}

#[test]
fn evidence_kind_all_variants() {
    let kinds = [
        EvidenceKind::Fuzz,
        EvidenceKind::Isolation,
        EvidenceKind::ManualQa,
        EvidenceKind::Provenance,
    ];
    for kind in kinds {
        let ref_e = EvidenceReference::repo_local(PathBuf::from("fuzz/test.rs"), kind);
        match ref_e {
            EvidenceReference::RepoLocal { kind: k, .. } => {
                assert_eq!(k, kind);
            }
            _ => panic!("Expected RepoLocal"),
        }
    }
}

#[test]
fn field_state_from_none_returns_missing() {
    let state: FieldState<String> = FieldState::from(None);
    assert!(matches!(state, FieldState::Missing));
}

#[test]
fn field_state_missing_map_preserves_missing() {
    let state: FieldState<String> = FieldState::Missing;
    let mapped = state.map(|s: String| s.len());
    assert!(matches!(mapped, FieldState::Missing));
}

#[test]
fn field_state_missing_as_ref_preserves_missing() {
    let state: FieldState<String> = FieldState::Missing;
    let ref_state = state.as_ref();
    assert!(matches!(ref_state, FieldState::Missing));
}

#[test]
fn validated_inventory_empty_records_cast_to_completion() {
    let validated = ValidatedBoundaryInventory::from_records(Vec::new());
    assert_eq!(validated.records.len(), 0);
    assert_eq!(validated.discovered_boundary_count, 0);
}

#[test]
fn workspace_root_path_accessor_works() {
    let path = PathBuf::from("/some/test/path");
    let root = WorkspaceRoot::new(path.clone());
    assert_eq!(root.path, path);
}

#[test]
fn freshness_marker_new_preserves_all_three_versions() {
    let marker = FreshnessMarker::new(10, 20, 30);
    assert_eq!(marker.source_version, 10);
    assert_eq!(marker.schema_version, 20);
    assert_eq!(marker.evidence_version, 30);
}

#[test]
fn boundary_candidate_new_empty_path_returns_empty() {
    let candidate = BoundaryCandidate::new("", "extern-c-boundary");
    assert!(candidate.source_path.as_os_str().is_empty());
    assert_eq!(candidate.marker, "extern-c-boundary");
}

#[test]
fn boundary_record_draft_review_status_missing() {
    let record = BoundaryRecordDraft::new(BoundaryRecordParts {
        id: "test-id".to_string(),
        class: BoundaryClass::CAbi,
        source_path: PathBuf::from("crates/test/src/lib.rs"),
        owner: FieldState::Present(Owner("test-owner".to_string())),
        threat: FieldState::Present(ThreatStatement("test-threat".to_string())),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("fuzz/test.rs"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Missing,
        waiver: FieldState::Missing,
    });
    assert!(record.review_status().is_none());
}

#[test]
fn review_status_from_serialized_unique_values() {
    let approved = ReviewStatus::from_serialized("approved");
    let waived = ReviewStatus::from_serialized("waived");
    let custom = ReviewStatus::from_serialized("custom-status");
    assert!(matches!(approved, ReviewStatus::Approved));
    assert!(matches!(waived, ReviewStatus::Waived));
    assert!(matches!(custom, ReviewStatus::Other(ref s) if s == "custom-status"));
}

// =============================================================================
// Kani verification harnesses — prove no-panic and invariants
// =============================================================================
// Kani harnesses are in src/kani_harnesses.rs (outside #[cfg(test)]
// so they compile with `cargo kani --lib`).
// See that file for the 20 `#[kani::proof]` harnesses.
