use super::support::*;

#[test]
fn unsafe_forbidden_boundary_rejects_with_unsafe_forbidden_violation() {
    let record = valid_record(
        BoundaryClass::UnsafeAdjacentDependency,
        "crates/first_party_unsafe/src/lib.rs",
    );
    let inventory = ValidatedBoundaryInventory::from_records(vec![record]);

    let result = inventory_completion_status(inventory);

    assert_eq!(
        result,
        Err(BoundaryInventoryError::UnsafeForbiddenViolation)
    );
}

#[test]
fn inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present() {
    let record = valid_record(BoundaryClass::Unknown, "crates/unknown/src/lib.rs");
    let inventory = ValidatedBoundaryInventory::from_records(vec![record]);

    let result = inventory_completion_status(inventory);

    assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
}

#[test]
fn inventory_completion_status_returns_incomplete_discovery_input_when_inventory_empty_but_boundaries_discovered()
 {
    let inventory = ValidatedBoundaryInventory::empty_with_discovered_boundary_count(1);

    let result = inventory_completion_status(inventory);

    assert_eq!(
        result,
        Err(BoundaryInventoryError::IncompleteDiscoveryInput)
    );
}

#[test]
fn inventory_completion_status_returns_complete_when_all_boundaries_valid_fresh_reviewed_and_traceable()
 {
    let records = vec![
        valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs"),
        valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs"),
    ];
    let inventory = ValidatedBoundaryInventory::from_records(records);

    let result = inventory_completion_status(inventory);

    assert_eq!(
        result,
        Ok(UnsafeIsolationStatus::Complete { boundary_count: 2 })
    );
}

#[test]
fn validated_inventory_from_validated_records_preserves_schema_records_status_and_count() {
    let ipc = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let decoder = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    let expected_records = vec![ipc, decoder];

    let inventory = ValidatedBoundaryInventory::from_validated_records(
        1,
        expected_records.clone(),
        Some(String::from("approved")),
    );

    assert_eq!(
        (
            inventory.schema_version,
            inventory.records,
            inventory.discovered_boundary_count,
            inventory.review_status,
        ),
        (1, expected_records, 2, Some(String::from("approved")))
    );
}

#[test]
fn validate_inventory_preserves_record_ids_source_paths_evidence_and_order() {
    let ipc = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let decoder = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    let expected = vec![
        (
            ipc.id.clone(),
            ipc.source_path.clone(),
            ipc.evidence.clone(),
        ),
        (
            decoder.id.clone(),
            decoder.source_path.clone(),
            decoder.evidence.clone(),
        ),
    ];
    let inventory = BoundaryInventory::new(Some(1), vec![ipc, decoder], None);

    let result = validate_inventory(inventory, workspace("complete_workspace")).map(|validated| {
        validated
            .records
            .iter()
            .map(|record| {
                (
                    record.id.clone(),
                    record.source_path.clone(),
                    record.evidence.clone(),
                )
            })
            .collect::<Vec<_>>()
    });

    assert_eq!(result, Ok(expected));
}

#[test]
fn validated_inventory_from_records_preserves_record_count_and_order_for_completion() {
    let decoder = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    let ipc = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let expected_ids = vec![decoder.id.clone(), ipc.id.clone()];

    let inventory = ValidatedBoundaryInventory::from_records(vec![decoder, ipc]);
    let ids = inventory
        .records
        .iter()
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        (
            inventory.discovered_boundary_count,
            ids,
            inventory_completion_status(inventory),
        ),
        (
            2,
            expected_ids,
            Ok(UnsafeIsolationStatus::Complete { boundary_count: 2 }),
        )
    );
}

#[test]
fn boundary_inventory_new_preserves_discovery_trace_until_validation_boundary() {
    let record = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let trace = evidence("proof-obligations.jsonl");

    let inventory = BoundaryInventory::new(Some(1), vec![record.clone()], Some(trace.clone()));

    assert_eq!(
        (
            inventory.schema_version,
            inventory.records,
            inventory.discovery_trace,
        ),
        (Some(1), vec![record], Some(trace))
    );
}

#[test]
fn validate_then_completion_preserves_records_traceability_and_count() {
    let ipc = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let decoder = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    let expected_ids = [ipc.id.clone(), decoder.id.clone()];
    let inventory = BoundaryInventory::new(Some(1), vec![ipc, decoder], None);

    let result = validate_inventory(inventory, workspace("complete_workspace"));

    match result {
        Ok(validated) => {
            assert_eq!(validated.discovered_boundary_count, expected_ids.len());
            assert_eq!(validated.review_status, Some(String::from("approved")));
            assert_eq!(validated.records.len(), expected_ids.len());
            let first_record = validated.records.first();
            let first_id = expected_ids.first();
            assert_eq!(first_record.map(|record| &record.id), first_id);
            assert!(matches!(
                first_record.map(|record| &record.evidence),
                Some(FieldState::Present(_))
            ));
            assert_eq!(
                inventory_completion_status(validated),
                Ok(UnsafeIsolationStatus::Complete { boundary_count: 2 })
            );
        }
        Err(error) => assert_eq!(error, BoundaryInventoryError::InventoryParseFailure),
    }
}

#[test]
fn validated_inventory_equality_rejects_different_schema_status_and_counts() {
    let left = validated_with_schema_status_and_count(1, Some("approved"), 1);
    let right = validated_with_schema_status_and_count(2, Some("waived"), 2);

    assert_ne!(left, right);
}

#[test]
fn validated_inventory_equality_requires_all_identity_fields_to_match() {
    let baseline = validated_with_schema_status_and_count(1, Some("approved"), 3);
    let schema_mismatch = validated_with_schema_status_and_count(2, Some("approved"), 3);
    let status_mismatch = validated_with_schema_status_and_count(1, Some("waived"), 3);
    let count_mismatch = validated_with_schema_status_and_count(1, Some("approved"), 4);

    assert_ne!(baseline, schema_mismatch);
    assert_ne!(baseline, status_mismatch);
    assert_ne!(baseline, count_mismatch);
}

#[test]
fn validated_inventory_equality_rejects_count_mismatch_when_schema_and_status_match() {
    let left = validated_with_schema_status_and_count(1, Some("approved"), 7);
    let right = validated_with_schema_status_and_count(1, Some("approved"), 8);

    assert_ne!(left, right);
}

#[test]
fn validated_inventory_equality_rejects_different_explicit_review_statuses() {
    let approved = validated_empty_with_status("approved");
    let waived = validated_empty_with_status("waived");

    assert_ne!(approved, waived);
}

#[test]
fn inventory_completion_status_preserves_first_review_status_in_validated_output() {
    let mut approved = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    approved.review_status = FieldState::Present(ReviewStatus::Approved);
    let mut waived = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");
    waived.review_status = FieldState::Present(ReviewStatus::Waived);
    waived.waiver = FieldState::Present(evidence(".beads/vb-y1zq/contract-verification-review.md"));
    let inventory = BoundaryInventory::new(Some(1), vec![approved, waived], None);

    let result = validate_inventory(inventory, workspace("complete_workspace"))
        .map(|validated| validated.review_status);

    assert_eq!(result, Ok(Some(String::from("approved"))));
}

#[test]
fn validated_inventory_equality_rejects_discovered_count_mismatch() {
    let left = validated_empty_with_count(2);
    let right = validated_empty_with_count(3);

    assert_ne!(left, right);
}

#[test]
fn validated_inventory_equality_rejects_empty_vs_nonempty_records() {
    let empty = validated_with_schema_status_and_count(1, None, 1);
    let nonempty = validated_with_records(vec![valid_record(
        BoundaryClass::Ipc,
        "crates/vb_ipc/src/frame.rs",
    )]);

    assert_ne!(empty, nonempty);
}

#[test]
fn validated_inventory_equality_uses_records_when_left_records_nonempty() {
    let left = validated_with_records(vec![valid_record(
        BoundaryClass::Ipc,
        "crates/vb_ipc/src/frame.rs",
    )]);
    let right = validated_empty_with_count(1);

    assert_ne!(left, right);
}

#[test]
fn validated_inventory_equality_uses_records_when_right_records_nonempty() {
    let left = validated_empty_with_count(1);
    let right = validated_with_records(vec![valid_record(
        BoundaryClass::Ipc,
        "crates/vb_ipc/src/frame.rs",
    )]);

    assert_ne!(left, right);
}

#[test]
fn validated_inventory_equality_accepts_identical_records_and_rejects_different_records() {
    let ipc_left = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let ipc_right = valid_record(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs");
    let decoder = valid_record(BoundaryClass::Decoder, "crates/vb_yaml/src/decode.rs");

    assert_eq!(
        validated_with_records(vec![ipc_left.clone()]),
        validated_with_records(vec![ipc_right])
    );
    assert_ne!(
        validated_with_records(vec![ipc_left]),
        validated_with_records(vec![decoder])
    );
}
