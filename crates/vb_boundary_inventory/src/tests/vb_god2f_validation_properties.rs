#![forbid(unsafe_code)]

//! HVR-PO-BI-002: generated behavior properties for boundary-inventory validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use proptest::prelude::*;
use proptest::strategy::Strategy;

use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryInventory, BoundaryInventoryError,
    BoundaryRecordDraft, BoundaryRecordParts, EvidenceKind, EvidenceReference, FieldState,
    FreshnessMarker, Owner, ReviewStatus, ThreatStatement, UnsafeIsolationStatus,
    ValidatedBoundaryInventory, WorkspaceRoot, classify_boundary, inventory_completion_status,
    validate_evidence_reference_bytes, validate_inventory,
};

fn base36_suffix() -> impl Strategy<Value = String> {
    proptest::collection::vec(0u8..36, 1..16).prop_map(|symbols| {
        symbols
            .into_iter()
            .map(|symbol| {
                if symbol < 26 {
                    char::from(b'a'.saturating_add(symbol))
                } else {
                    char::from(b'0'.saturating_add(symbol.saturating_sub(26)))
                }
            })
            .collect()
    })
}

fn marker_strategy() -> impl Strategy<Value = (&'static str, Option<BoundaryClass>)> {
    prop_oneof![
        Just(("extern-c-boundary", Some(BoundaryClass::CAbi))),
        Just(("foreign-function-boundary", Some(BoundaryClass::Ffi))),
        Just(("ipc-frame-boundary", Some(BoundaryClass::Ipc))),
        Just((
            "external-binary-boundary",
            Some(BoundaryClass::ExternalBinary)
        )),
        Just(("decoder-byte-ingest-boundary", Some(BoundaryClass::Decoder))),
        Just((
            "generated-interface-boundary",
            Some(BoundaryClass::GeneratedCode)
        )),
        Just((
            "unsafe-adjacent-dependency-boundary",
            Some(BoundaryClass::UnsafeAdjacentDependency)
        )),
        Just(("unknown-boundary-marker", None)),
    ]
}

fn valid_record(
    id: String,
    class: BoundaryClass,
    review_status: ReviewStatus,
    freshness: FreshnessMarker,
) -> BoundaryRecordDraft {
    BoundaryRecordDraft::new(BoundaryRecordParts {
        id,
        class,
        source_path: PathBuf::from("crates/vb_boundary_inventory/src/lib.rs"),
        owner: FieldState::Present(Owner(String::from("verification"))),
        threat: FieldState::Present(ThreatStatement(String::from("untrusted evidence path"))),
        evidence: FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
            "vb-god2f",
        ))),
        freshness,
        review_status: FieldState::Present(review_status),
        waiver: FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
            "vb-god2f",
        ))),
    })
}

fn duplicate_exists(records: &[BoundaryRecordDraft]) -> bool {
    let mut seen = BTreeSet::new();
    for record in records {
        if !seen.insert(record.id.as_str()) {
            return true;
        }
    }
    false
}

proptest! {
    #[test]
    fn vb_god2f_boundary_inventory_validation_evidence_properties(suffix in base36_suffix()) {
        let bead_id = format!("vb-{suffix}");
        prop_assert!(matches!(
            validate_evidence_reference_bytes(bead_id.as_bytes()),
            Ok(EvidenceReference::ExternalProvenance(_))
        ));

        let external = format!("external:source-{suffix}#sha256=abc123");
        prop_assert!(matches!(
            validate_evidence_reference_bytes(external.as_bytes()),
            Ok(EvidenceReference::ExternalProvenance(_))
        ));

        let missing_digest = format!("external:source-{suffix}");
        prop_assert!(matches!(
            validate_evidence_reference_bytes(missing_digest.as_bytes()),
            Err(BoundaryInventoryError::InvalidEvidencePath)
        ));

        let parent_path = format!("../{suffix}");
        prop_assert!(matches!(
            validate_evidence_reference_bytes(parent_path.as_bytes()),
            Err(BoundaryInventoryError::InvalidEvidencePath)
        ));
    }

    #[test]
    fn vb_god2f_boundary_inventory_validation_classification_properties(
        (marker, expected) in marker_strategy(),
        suffix in base36_suffix(),
    ) {
        let source_path = PathBuf::from(format!("crates/{suffix}/src/lib.rs"));
        let first = classify_boundary(BoundaryCandidate::new(source_path.clone(), marker));
        let second = classify_boundary(BoundaryCandidate::new(source_path, marker));
        match expected {
            Some(class) => {
                let first = first.map_err(|error| TestCaseError::fail(format!("classification failed: {error:?}")))?;
                let second = second.map_err(|error| TestCaseError::fail(format!("classification failed: {error:?}")))?;
                prop_assert_eq!(first.class, class);
                prop_assert_eq!(first.id, second.id);
            }
            None => {
                prop_assert!(matches!(first, Err(BoundaryInventoryError::UnknownBoundaryClass)));
                prop_assert!(matches!(second, Err(BoundaryInventoryError::UnknownBoundaryClass)));
            }
        }
    }

    #[test]
    fn vb_god2f_boundary_inventory_validation_inventory_properties(
        ids in proptest::collection::vec(base36_suffix(), 0..32),
        stale in any::<bool>(),
        other_status in any::<bool>(),
    ) {
        let freshness = if stale {
            FreshnessMarker::new(2, 2, 1)
        } else {
            FreshnessMarker::new(1, 1, 1)
        };
        let review = if other_status {
            ReviewStatus::Other(String::from("manual-review"))
        } else {
            ReviewStatus::Approved
        };
        let records: Vec<BoundaryRecordDraft> = ids
            .into_iter()
            .map(|id| valid_record(format!("boundary-{id}"), BoundaryClass::Decoder, review.clone(), freshness))
            .collect();
        let has_duplicate = duplicate_exists(&records);
        let count = records.len();
        let inventory = BoundaryInventory::new(Some(1), records, None);
        let result = validate_inventory(inventory, WorkspaceRoot::new(PathBuf::new()));

        if has_duplicate {
            prop_assert!(matches!(result, Err(BoundaryInventoryError::DuplicateBoundaryId)));
        } else if stale && count > 0 {
            prop_assert!(matches!(result, Err(BoundaryInventoryError::StaleEvidence)));
        } else if other_status && count > 0 {
            prop_assert!(matches!(result, Err(BoundaryInventoryError::ReviewStatusInvalid)));
        } else {
            prop_assert!(result.is_ok(), "valid generated inventory should pass, got {result:?}");
        }
    }

    #[test]
    fn vb_god2f_boundary_inventory_validation_completion_properties(count in 0usize..32) {
        let inventory = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(count);
        let status = inventory_completion_status(inventory);
        if count == 0 {
            prop_assert_eq!(
                status,
                Ok(UnsafeIsolationStatus::Complete { boundary_count: 0 })
            );
        } else {
            prop_assert!(matches!(status, Err(BoundaryInventoryError::IncompleteDiscoveryInput)));
        }
    }
}

#[test]
fn vb_god2f_boundary_inventory_validation_duplicate_golden_case() {
    let records = vec![
        valid_record(
            String::from("duplicate"),
            BoundaryClass::Decoder,
            ReviewStatus::Approved,
            FreshnessMarker::new(1, 1, 1),
        ),
        valid_record(
            String::from("duplicate"),
            BoundaryClass::Decoder,
            ReviewStatus::Approved,
            FreshnessMarker::new(1, 1, 1),
        ),
    ];
    let inventory = BoundaryInventory::new(Some(1), records, None);
    assert!(matches!(
        validate_inventory(inventory, WorkspaceRoot::new(PathBuf::new())),
        Err(BoundaryInventoryError::DuplicateBoundaryId)
    ));
}

#[test]
fn vb_god2f_boundary_inventory_validation_repo_local_evidence_uses_tempfile_workspace()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    fs::create_dir_all(workspace.path().join("fuzz"))?;
    fs::write(workspace.path().join("fuzz/hvr-po-bi-002.txt"), b"seed")?;

    let record = BoundaryRecordDraft::new(BoundaryRecordParts {
        id: String::from("repo-local-evidence"),
        class: BoundaryClass::Decoder,
        source_path: PathBuf::from("crates/vb_boundary_inventory/src/lib.rs"),
        owner: FieldState::Present(Owner(String::from("verification"))),
        threat: FieldState::Present(ThreatStatement(String::from("repo-local path validation"))),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("fuzz/hvr-po-bi-002.txt"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    });
    let inventory = BoundaryInventory::new(Some(1), vec![record], None);
    let result = validate_inventory(
        inventory,
        WorkspaceRoot::new(workspace.path().to_path_buf()),
    );
    assert!(
        result.is_ok(),
        "repo-local tempfile evidence should validate: {result:?}"
    );
    Ok(())
}
