//! Property-based tests for vb_boundary_inventory
//!
//! Uses proptest to generate arbitrary inputs for comprehensive coverage

// `assert!(false, ...)` appears inside `match` arms whose Ok branch already
// bound the value. The assertion is the documented "this branch is unreachable"
// marker with a clear diagnostic message. `panic!`/`unreachable!` are forbidden
// by workspace lints, so we suppress `assertions_on_constants` at module scope.
#![allow(clippy::assertions_on_constants)]

use std::path::PathBuf;

use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryRecord, BoundaryRecordDraft,
    BoundaryRecordParts, BoundaryRisk, ClassifiedBoundary, ClassifiedBoundaryInput, EvidenceKind,
    EvidenceReference, FieldState, FreshnessMarker, Owner, ReviewStatus, ThreatStatement,
    ValidatedBoundaryInventory, WorkspaceRoot, classify_boundary,
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
        prop_assume!(!path.is_empty());
        let candidate = BoundaryCandidate::new(path.clone(), "test-marker".to_string());
        prop_assert!(!candidate.source_path.as_os_str().is_empty(), "kani harness assertion");
    }

    fn boundary_candidate_marker_roundtrip(marker: String) {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker.clone());
        prop_assert_eq!(candidate.marker, marker);
    }

    fn classify_boundary_exposure_is_risky_for_multiple(marker in "extern-c-boundary|foreign-function-boundary|ipc-frame-boundary") {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker.clone());
        let result = classify_boundary(candidate);
        match &result {
            Ok(classified) => {
                prop_assert_eq!(classified.exposure.risk, BoundaryRisk::Multiple);
            }
            Err(_) => {
                prop_assert!(false, "kani harness assertion");
            }
        }
    }

    fn classify_boundary_different_paths_produce_different_ids(path1 in "[a-z]{1,10}", path2 in "[a-z]{1,10}") {
        prop_assume!(path1 != path2);
        let candidate1 = BoundaryCandidate::new(format!("crates/{}/src/lib.rs", path1), "extern-c-boundary".to_string());
        let candidate2 = BoundaryCandidate::new(format!("crates/{}/src/lib.rs", path2), "extern-c-boundary".to_string());
        let result1 = classify_boundary(candidate1);
        let result2 = classify_boundary(candidate2);
        let id1 = match &result1 {
            Ok(classified) => classified.id.clone(),
            Err(_) => {
                prop_assert!(false, "kani harness assertion");
                String::new()
            }
        };
        let id2 = match &result2 {
            Ok(classified) => classified.id.clone(),
            Err(_) => {
                prop_assert!(false, "kani harness assertion");
                String::new()
            }
        };
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
        prop_assert!(!classified.id.is_empty(), "kani harness assertion");
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
        prop_assert!(marker.evidence_version >= marker.source_version, "kani harness assertion");
        prop_assert!(marker.evidence_version >= marker.schema_version, "kani harness assertion");
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
            _ => prop_assert!(false), }
    }

    fn field_state_from_option_some(value: String) {
        let state: FieldState<String> = FieldState::from(Some(value.clone()));
        match state {
            FieldState::Present(v) => prop_assert_eq!(v, value),
            FieldState::Missing => prop_assert!(false), }
    }

    fn field_state_as_ref(value: String) {
        let state: FieldState<String> = FieldState::Present(value.clone());
        let ref_state = state.as_ref();
        match ref_state {
            FieldState::Present(v) => prop_assert_eq!(v, &value),
            FieldState::Missing => prop_assert!(false), }
    }

    fn field_state_map(value: String) {
        let state: FieldState<String> = FieldState::Present(value.clone());
        let mapped = state.map(|v| v.len());
        match mapped {
            FieldState::Present(len) => prop_assert_eq!(len, value.len()),
            FieldState::Missing => prop_assert!(false), }
    }

    fn validated_inventory_with_schema_version(schema: u32) {
        let validated = ValidatedBoundaryInventory::with_schema_version(schema);
        prop_assert_eq!(validated.schema_version, schema);
        prop_assert!(validated.records.is_empty(), "kani harness assertion");
        prop_assert_eq!(validated.discovered_boundary_count, 0);
    }

    fn validated_inventory_with_review_status(status: String) {
        let validated = ValidatedBoundaryInventory::with_review_status(status.clone());
        prop_assert_eq!(validated.review_status, Some(status));
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

    fn boundary_candidate_new_empty_path(marker: String) {
        let candidate = BoundaryCandidate::new("", marker.clone());
        prop_assert_eq!(candidate.marker, marker);
        prop_assert!(candidate.source_path.as_os_str().is_empty(), "kani harness assertion");
    }

    fn workspace_root_new_roundtrip(path: String) {
        let workspace = WorkspaceRoot::new(PathBuf::from(path.clone()));
        prop_assert_eq!(workspace.path, PathBuf::from(path));
    }

    fn freshness_marker_new_any_versions(sv: u64, schema: u64, ev: u64) {
        let marker = FreshnessMarker::new(sv, schema, ev);
        prop_assert_eq!(marker.source_version, sv);
        prop_assert_eq!(marker.schema_version, schema);
        prop_assert_eq!(marker.evidence_version, ev);
    }
}

// =============================================================================
// Regular tests for enum variants and deterministic behavior
// =============================================================================

#[test]
fn boundary_exposure_none() {
    let exposure = BoundaryExposure::none();
    assert_eq!(exposure.risk, BoundaryRisk::None);
}

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
        let Ok(classified) = result else {
            assert!(false, "classify_boundary must succeed for valid class");
            return;
        };
        assert_eq!(classified.class, class);
    }
}

#[test]
fn review_status_from_serialized_approved() {
    let status = ReviewStatus::from_serialized("approved");
    assert!(
        matches!(status, ReviewStatus::Approved),
        "approved must parse to Approved"
    );
}

#[test]
fn review_status_from_serialized_waived() {
    let status = ReviewStatus::from_serialized("waived");
    assert!(
        matches!(status, ReviewStatus::Waived),
        "waived must parse to Waived"
    );
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
            _ => assert!(false, "Expected RepoLocal"),
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
    assert!(
        candidate.source_path.as_os_str().is_empty(),
        "empty path must produce empty source_path"
    );
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
    assert!(
        record.review_status().is_none(),
        "Missing review_status must return None"
    );
    // Mutation gate: if review_status() were to return Some(_) for Missing state,
    // this test would catch it.
}

#[test]
fn review_status_from_serialized_unique_values() {
    let approved = ReviewStatus::from_serialized("approved");
    let waived = ReviewStatus::from_serialized("waived");
    let custom = ReviewStatus::from_serialized("custom-status");
    assert!(
        matches!(approved, ReviewStatus::Approved),
        "approved must parse to Approved"
    );
    assert!(
        matches!(waived, ReviewStatus::Waived),
        "waived must parse to Waived"
    );
    assert!(matches!(custom, ReviewStatus::Other(ref s) if s == "custom-status"));
}

// =============================================================================
// Kani verification harnesses — prove no-panic and invariants
// =============================================================================
// Kani harnesses are in src/kani_harnesses.rs (outside #[cfg(test)]
// so they compile with `cargo kani --lib`).
// See that file for the 20 `#[kani::proof]` harnesses.
