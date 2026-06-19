#![cfg(all(kani, feature = "kani-vb-god2f-boundary-inventory"))]
#![forbid(unsafe_code)]

//! HVR-PO-BI-001: feature-isolated boundary-inventory Kani harnesses.

use std::path::PathBuf;
use std::vec::Vec;

use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryInventory, BoundaryInventoryError,
    BoundaryRecordDraft, BoundaryRecordParts, EvidenceReference, FieldState, FreshnessMarker,
    Owner, ReviewStatus, ThreatStatement, ValidatedBoundaryInventory, WorkspaceRoot,
    classify_boundary, inventory_completion_status, validate_evidence_reference_bytes,
    validate_inventory,
};

fn marker_from_selector(selector: u8) -> &'static str {
    match selector {
        0 => "extern-c-boundary",
        1 => "foreign-function-boundary",
        2 => "ipc-frame-boundary",
        3 => "external-binary-boundary",
        4 => "decoder-byte-ingest-boundary",
        5 => "generated-interface-boundary",
        6 => "unsafe-adjacent-dependency-boundary",
        _ => "unknown-boundary-marker",
    }
}

fn expected_class(selector: u8) -> Option<BoundaryClass> {
    match selector {
        0 => Some(BoundaryClass::CAbi),
        1 => Some(BoundaryClass::Ffi),
        2 => Some(BoundaryClass::Ipc),
        3 => Some(BoundaryClass::ExternalBinary),
        4 => Some(BoundaryClass::Decoder),
        5 => Some(BoundaryClass::GeneratedCode),
        6 => Some(BoundaryClass::UnsafeAdjacentDependency),
        _ => None,
    }
}

fn bounded_evidence_bytes(selector: u8, raw: [u8; 32]) -> Vec<u8> {
    match selector {
        0 => b"external:manual#sha256=abc".to_vec(),
        1 => b"external:missing-digest".to_vec(),
        2 => b"vb-god2f".to_vec(),
        3 => b"../outside".to_vec(),
        _ => raw.to_vec(),
    }
}

fn record_with_id(
    id: String,
    review: ReviewStatus,
    freshness: FreshnessMarker,
) -> BoundaryRecordDraft {
    BoundaryRecordDraft::new(BoundaryRecordParts {
        id,
        class: BoundaryClass::Decoder,
        source_path: PathBuf::from("crates/vb_boundary_inventory/src/lib.rs"),
        owner: FieldState::Present(Owner(String::from("verification"))),
        threat: FieldState::Present(ThreatStatement(String::from("untrusted evidence path"))),
        evidence: FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
            "vb-god2f",
        ))),
        freshness,
        review_status: FieldState::Present(review),
        waiver: FieldState::Present(EvidenceReference::ExternalProvenance(String::from(
            "vb-god2f",
        ))),
    })
}

fn generated_inventory(selector: u8, duplicate: bool, stale: bool) -> BoundaryInventory {
    let count = usize::from(selector & 3);
    let freshness = if stale {
        FreshnessMarker::new(2, 2, 1)
    } else {
        FreshnessMarker::new(1, 1, 1)
    };
    let review = if selector & 4 == 0 {
        ReviewStatus::Approved
    } else {
        ReviewStatus::Waived
    };
    let mut records = Vec::new();
    let mut index: usize = 0;
    while index < count {
        let id = if duplicate && index > 0 {
            String::from("boundary-0")
        } else {
            format!("boundary-{index}")
        };
        records.push(record_with_id(id, review.clone(), freshness));
        index = match index.checked_add(1) {
            Some(value) => value,
            None => count,
        };
    }
    BoundaryInventory::new(Some(1), records, None)
}

#[kani::proof]
#[kani::unwind(16)]
fn vb_god2f_boundary_inventory_validation_bounded() {
    let selector: u8 = kani::any();
    kani::assume(selector <= 7);
    let raw: [u8; 32] = kani::any();
    let evidence = bounded_evidence_bytes(selector, raw);
    let evidence_result = validate_evidence_reference_bytes(&evidence);

    kani::cover!(selector == 0, "external sha256 evidence branch");
    kani::cover!(selector == 2, "bead id evidence branch");
    kani::cover!(selector == 3, "parent path rejection branch");

    if selector == 0 || selector == 2 {
        kani::assert(
            evidence_result.is_ok(),
            "known external evidence shapes validate",
        );
    }
    if selector == 1 || selector == 3 {
        kani::assert(
            matches!(
                evidence_result,
                Err(BoundaryInventoryError::InvalidEvidencePath)
            ),
            "invalid external/path shapes reject with typed error",
        );
    }

    let marker_result = classify_boundary(BoundaryCandidate::new(
        PathBuf::from("crates/vb_boundary_inventory/src/lib.rs"),
        marker_from_selector(selector),
    ));
    match expected_class(selector) {
        Some(class) => match marker_result {
            Ok(boundary) => {
                kani::assert(
                    boundary.class == class,
                    "marker maps to expected boundary class",
                );
                kani::assert(
                    !boundary.id.is_empty(),
                    "classified boundary has stable id text",
                );
            }
            Err(_) => kani::assert(false, "known marker must classify"),
        },
        None => kani::assert(
            matches!(
                marker_result,
                Err(BoundaryInventoryError::UnknownBoundaryClass)
            ),
            "unknown marker returns typed class error",
        ),
    }

    let duplicate: bool = kani::any();
    let stale: bool = kani::any();
    let inventory = generated_inventory(selector, duplicate, stale);
    let validation = validate_inventory(inventory, WorkspaceRoot::new(PathBuf::new()));
    if duplicate && usize::from(selector & 3) > 1 {
        kani::assert(
            matches!(validation, Err(BoundaryInventoryError::DuplicateBoundaryId)),
            "duplicate generated IDs reject",
        );
    } else if stale && usize::from(selector & 3) > 0 {
        kani::assert(
            matches!(validation, Err(BoundaryInventoryError::StaleEvidence)),
            "stale generated evidence rejects",
        );
    } else {
        kani::cover!(validation.is_ok(), "generated inventory validates");
    }

    let empty_count: usize = kani::any();
    kani::assume(empty_count <= 4);
    let completion = inventory_completion_status(
        ValidatedBoundaryInventory::empty_with_discovered_boundary_count(empty_count),
    );
    if empty_count == 0 {
        kani::assert(completion.is_ok(), "empty discovered count completes");
    } else {
        kani::assert(
            matches!(
                completion,
                Err(BoundaryInventoryError::IncompleteDiscoveryInput)
            ),
            "nonzero discovered count without records is incomplete",
        );
    }
}
