#[cfg(kani)]
mod kani_verification {
    use std::path::PathBuf;

    use crate::boundary_inventory::{
        BoundaryCandidate, BoundaryExposure, BoundaryRecordDraft, BoundaryRecordParts,
        ClassifiedBoundary, ClassifiedBoundaryInput, EvidenceKind, EvidenceReference, FieldState,
        FreshnessMarker, Owner, ReviewStatus, ThreatStatement, ValidatedBoundaryInventory,
        WorkspaceRoot, classify_boundary,
    };

    #[kani::proof]
    fn field_state_present_map_never_panics() {
        let val: u64 = kani::any();
        let present: FieldState<u64> = FieldState::Present(val);
        let mapped = present.map(|v| v.wrapping_add(1));
        match mapped {
            FieldState::Present(v) => assert!(v == val.wrapping_add(1)),
            FieldState::Missing => {}
        }
    }

    #[kani::proof]
    fn field_state_missing_map_never_panics() {
        let missing: FieldState<u64> = FieldState::Missing;
        let mapped = missing.map(|v: u64| v.wrapping_add(1));
        match mapped {
            FieldState::Missing => {}
            FieldState::Present(_) => {}
        }
    }

    #[kani::proof]
    fn field_state_from_option_never_panics() {
        let val: u64 = kani::any();
        let some: FieldState<u64> = FieldState::from(Some(val));
        match some {
            FieldState::Present(v) => #[cfg(kani)]
mod kani_verification {
    use std::path::PathBuf;

    use crate::boundary_inventory::{
        BoundaryCandidate, BoundaryExposure, BoundaryRecordDraft, BoundaryRecordParts,
        ClassifiedBoundary, ClassifiedBoundaryInput, EvidenceKind, EvidenceReference, FieldState,
        FreshnessMarker, Owner, ReviewStatus, ThreatStatement, ValidatedBoundaryInventory,
        WorkspaceRoot, classify_boundary,
    };

    #[kani::proof]
    fn field_state_present_map_never_panics() {
        let val: u64 = kani::any();
        let present: FieldState<u64> = FieldState::Present(val);
        let mapped = present.map(|v| v.wrapping_add(1));
        match mapped {
            FieldState::Present(v) => assert!(v == val.wrapping_add(1)),
            FieldState::Missing => {}
        }
    }

    #[kani::proof]
    fn field_state_missing_map_never_panics() {
        let missing: FieldState<u64> = FieldState::Missing;
        let mapped = missing.map(|v: u64| v.wrapping_add(1));
        match mapped {
            FieldState::Missing => {}
            FieldState::Present(_) => {}
        }
    }

    #[kani::proof]
    fn field_state_from_option_never_panics() {
        let val: u64 = kani::any();
        let some: FieldState<u64> = FieldState::from(Some(val));
        match some {
            FieldState::Present(v) => kani::assert(v == val),
            FieldState::Missing => {}
        }
        let none: FieldState<u64> = FieldState::from(None);
        match none {
            FieldState::Present(_) => {}
            FieldState::Missing => {}
        }
    }

    #[kani::proof]
    fn freshness_marker_new_never_panics() {
        let sv: u64 = kani::any();
        let schema: u64 = kani::any();
        let ev: u64 = kani::any();
        let marker = FreshnessMarker::new(sv, schema, ev);
        kani::assert(marker.source_version == sv, "kani harness assertion");
        kani::assert(marker.schema_version == schema, "kani harness assertion");
        kani::assert(marker.evidence_version == ev, "kani harness assertion");
    }

    #[kani::proof]
    fn boundary_exposure_none_risk_is_none() {
        let none = BoundaryExposure::none();
        kani::assert(
            none.risk == crate::boundary_inventory::BoundaryRisk::None,
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn boundary_exposure_risky_preserves_risk() {
        let none = BoundaryExposure::risky(crate::boundary_inventory::BoundaryRisk::None);
        kani::assert(
            none.risk == crate::boundary_inventory::BoundaryRisk::None,
            "kani harness assertion",
        );
        let ext = BoundaryExposure::risky(crate::boundary_inventory::BoundaryRisk::ExternalBytes);
        kani::assert(
            ext.risk == crate::boundary_inventory::BoundaryRisk::ExternalBytes,
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn workspace_root_new_never_panics() {
        let path = PathBuf::new();
        let root = WorkspaceRoot::new(path);
        let _ = root.path;
    }

    #[kani::proof]
    fn classified_boundary_new_never_panics() {
        let input = ClassifiedBoundaryInput {
            id: String::new(),
            class: crate::boundary_inventory::BoundaryClass::CAbi,
            source_path: PathBuf::new(),
            exposure: BoundaryExposure::none(),
        };
        let classified = ClassifiedBoundary::new(input);
        let _ = classified.id;
    }

    #[kani::proof]
    fn evidence_reference_repo_local_never_panics() {
        let kind = EvidenceKind::Fuzz;
        let reference = EvidenceReference::repo_local(PathBuf::new(), kind);
        match reference {
            EvidenceReference::RepoLocal { .. } => {}
            _ => {}
        }
    }

    #[kani::proof]
    fn evidence_reference_free_text_never_panics() {
        let reference = EvidenceReference::free_text(String::new());
        match reference {
            EvidenceReference::FreeText(_) => {}
            _ => {}
        }
    }

    #[kani::proof]
    fn review_status_approved_never_panics() {
        let status = ReviewStatus::from_serialized("approved");
        kani::assert(status.serialized() == "approved", "kani harness assertion");
    }

    #[kani::proof]
    fn review_status_waived_never_panics() {
        let status = ReviewStatus::from_serialized("waived");
        kani::assert(status.serialized() == "waived", "kani harness assertion");
    }

    #[kani::proof]
    fn review_status_other_preserves_value() {
        let status = ReviewStatus::Other(String::from("custom-status"));
        kani::assert(status.serialized() == "custom-status",
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn validated_inventory_with_schema_version_preserves_value() {
        let schema: u32 = kani::any();
        let v = ValidatedBoundaryInventory::with_schema_version(schema);
         == "custom-status",
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn validated_inventory_with_schema_version_preserves_value() {
        let schema: u32 = kani::any();
        let v = ValidatedBoundaryInventory::with_schema_version(schema);
        kani::assert(v.schema_version == schema, "kani harness assertion");
        kani::assert(v.records.is_empty(), "kani harness assertion");
        kani::assert(v.discovered_boundary_count == 0, "kani harness assertion");
    }

    #[kani::proof]
    fn validated_inventory_empty_with_count_preserves_value() {
        let count: usize = kani::any();
        kani::assume(count <= 8);
        let v = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(count);
        kani::assert(
            v.discovered_boundary_count == count,
            "kani harness assertion",
        );
    }

    #[kani::proof]
    fn boundary_record_draft_new_never_panics() {
        let parts = BoundaryRecordParts {
            id: String::from("test"),
            class: crate::boundary_inventory::BoundaryClass::CAbi,
            source_path: PathBuf::from("crates/test/src/lib.rs"),
            owner: FieldState::Present(Owner(String::from("owner"))),
            threat: FieldState::Present(ThreatStatement(String::from("threat"))),
            evidence: FieldState::Present(EvidenceReference::repo_local(
                PathBuf::from("fuzz/test.rs"),
                EvidenceKind::Fuzz,
            )),
            freshness: FreshnessMarker::new(1, 1, 1),
            review_status: FieldState::Present(ReviewStatus::Approved),
            waiver: FieldState::Missing,
        };
        let record = BoundaryRecordDraft::new(parts);
        match record.review_status() {
            Some(s) => assert!(!s.is_empty()),
            None => {}
        }
    }

    #[kani::proof]
    fn classify_boundary_all_known_markers_never_panic() {
        let markers = [
            "extern-c-boundary",
            "foreign-function-boundary",
            "ipc-frame-boundary",
            "external-binary-boundary",
            "decoder-byte-ingest-boundary",
            "generated-interface-boundary",
            "unsafe-adjacent-dependency-boundary",
        ];
        for marker in markers {
            let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker);
            let result = classify_boundary(candidate);
            match result {
                Ok(classified) => {
                    kani::assert(!classified.id.is_empty(), "kani harness assertion");
                }
                Err(_) => {}
            }
        }
    }

    #[kani::proof]
    fn classify_boundary_unknown_marker_handled() {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "nonexistent-marker-xyz");
        let result = classify_boundary(candidate);
        match result {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    #[kani::proof]
    fn field_state_missing_as_ref_never_panics() {
        let missing: FieldState<u64> = FieldState::Missing;
        let ref_state = missing.as_ref();
        match ref_state {
            FieldState::Missing => {}
            FieldState::Present(_) => {}
        }
    }

    #[kani::proof]
    fn classify_boundary_unknown_marker_handled() {
        let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "nonexistent-marker-xyz");
        let result = classify_boundary(candidate);
        match result {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    #[kani::proof]
    fn field_state_missing_as_ref_never_panics() {
        let missing: FieldState<u64> = FieldState::Missing;
        let ref_state = missing.as_ref();
        match ref_state {
            FieldState::Missing => {}
            FieldState::Present(_) => {}
        }
    }

    #[kani::proof]
    fn freshness_marker_edge_versions() {
        let marker_min = FreshnessMarker::new(0, 0, 0);
        kani::assert(marker_min.source_version == 0, "kani harness assertion");
        let marker_max = FreshnessMarker::new(u64::MAX, u64::MAX, u64::MAX);
        kani::assert(
            marker_max.evidence_version == u64::MAX,
            "kani harness assertion",
        );
    }
}
