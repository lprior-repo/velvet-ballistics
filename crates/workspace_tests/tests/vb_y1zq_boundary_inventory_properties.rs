//! RED PHASE property tests for bead vb-y1zq boundary inventory invariants.

use std::path::PathBuf;

use proptest::prelude::*;
use vb_workspace_tests::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryInventory, BoundaryInventoryError, BoundaryRecord,
    BoundaryRecordParts, EvidenceKind, EvidenceReference, EvidenceRequirement, FieldState,
    FreshnessMarker, Owner, ReviewStatus, ThreatStatement, WorkspaceRoot, classify_boundary,
    required_evidence, validate_inventory,
};

fn workspace() -> WorkspaceRoot {
    WorkspaceRoot::new(PathBuf::from("tests/fixtures/vb_y1zq/complete_workspace"))
}

fn arb_known_class() -> impl Strategy<Value = BoundaryClass> {
    prop_oneof![
        Just(BoundaryClass::CAbi),
        Just(BoundaryClass::Ffi),
        Just(BoundaryClass::Ipc),
        Just(BoundaryClass::ExternalBinary),
        Just(BoundaryClass::Decoder),
        Just(BoundaryClass::GeneratedCode),
        Just(BoundaryClass::UnsafeAdjacentDependency),
    ]
}

fn source_for(class: &BoundaryClass) -> &'static str {
    match class {
        BoundaryClass::CAbi => "crates/ffi/src/c_abi.rs",
        BoundaryClass::Ffi => "crates/ffi/src/lib.rs",
        BoundaryClass::Ipc => "crates/vb_ipc/src/frame.rs",
        BoundaryClass::ExternalBinary => "scripts/run-verifier.sh",
        BoundaryClass::Decoder => "crates/vb_yaml/src/decode.rs",
        BoundaryClass::GeneratedCode => "crates/vb_runtime/src/generated/interface.rs",
        BoundaryClass::UnsafeAdjacentDependency => "Cargo.toml",
        BoundaryClass::Unknown => "crates/unknown/src/lib.rs",
    }
}

fn marker_for(class: &BoundaryClass) -> &'static str {
    match class {
        BoundaryClass::CAbi => "extern-c-boundary",
        BoundaryClass::Ffi => "foreign-function-boundary",
        BoundaryClass::Ipc => "ipc-frame-boundary",
        BoundaryClass::ExternalBinary => "external-binary-boundary",
        BoundaryClass::Decoder => "decoder-byte-ingest-boundary",
        BoundaryClass::GeneratedCode => "generated-interface-boundary",
        BoundaryClass::UnsafeAdjacentDependency => "unsafe-adjacent-dependency-boundary",
        BoundaryClass::Unknown => "plain-rust-module",
    }
}

fn record_with(
    class: BoundaryClass,
    owner: Option<String>,
    threat: Option<String>,
) -> BoundaryRecord {
    BoundaryRecord::new(BoundaryRecordParts {
        id: format!("vb-y1zq-{class:?}"),
        class,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        owner: owner.map(Owner).into(),
        threat: threat.map(ThreatStatement).into(),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("formal-verification-report.md"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}

proptest! {
    #[test]
    fn classify_boundary_returns_matching_primary_class_for_every_known_marker(class in arb_known_class()) {
        let candidate = BoundaryCandidate::new(source_for(&class), marker_for(&class));

        let result = classify_boundary(candidate);

        prop_assert_eq!(result.map(|boundary| boundary.class), Ok(class));
    }

    #[test]
    fn classify_boundary_returns_unknown_boundary_class_for_unknown_markers(marker in "[a-z]{1,32}") {
        let candidate = BoundaryCandidate::new("crates/vb_core/src/lib.rs", marker);

        let result = classify_boundary(candidate);
        prop_assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
    }

    #[test]
    fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_every_risky_class(class in arb_known_class()) {
        let boundary = classify_boundary(BoundaryCandidate::new(source_for(&class), marker_for(&class)));

        let result = boundary.and_then(required_evidence);
        prop_assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
    }

    #[test]
    fn validate_inventory_returns_missing_owner_when_owner_is_absent(class in arb_known_class()) {
        let record = record_with(class, None, Some(String::from("threat exists")));
        let inventory = BoundaryInventory::new(Some(1), vec![record], None);

        let result = validate_inventory(inventory, workspace());
        prop_assert_eq!(result, Err(BoundaryInventoryError::MissingOwner));
    }

    #[test]
    fn validate_inventory_returns_missing_threat_when_threat_is_absent(class in arb_known_class()) {
        let record = record_with(class, Some(String::from("owner exists")), None);
        let inventory = BoundaryInventory::new(Some(1), vec![record], None);

        let result = validate_inventory(inventory, workspace());
        prop_assert_eq!(result, Err(BoundaryInventoryError::MissingThreat));
    }

    #[test]
    fn validate_inventory_returns_schema_version_unsupported_for_any_version_except_one(version in prop_oneof![0_u16..1, 2_u16..1000]) {
        let record = record_with(BoundaryClass::Ipc, Some(String::from("owner exists")), Some(String::from("threat exists")));
        let inventory = BoundaryInventory::new(Some(u32::from(version)), vec![record], None);

        let result = validate_inventory(inventory, workspace());
        prop_assert_eq!(result, Err(BoundaryInventoryError::SchemaVersionUnsupported));
    }
}
