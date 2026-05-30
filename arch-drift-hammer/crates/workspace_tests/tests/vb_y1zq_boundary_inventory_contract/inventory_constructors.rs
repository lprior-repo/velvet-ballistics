use super::support::*;

#[test]
fn validated_inventory_with_review_status_preserves_status_and_defaults_other_fields() {
    let inventory = ValidatedBoundaryInventory::with_review_status("approved");

    assert_eq!(
        (
            inventory.schema_version,
            inventory.records,
            inventory.discovered_boundary_count,
            inventory.review_status,
        ),
        (1, Vec::new(), 0, Some(String::from("approved")))
    );
}

#[test]
fn validated_inventory_with_review_status_preserves_waived_status_without_records() {
    let inventory = ValidatedBoundaryInventory::with_review_status("waived");

    assert_eq!(
        (
            inventory.schema_version,
            inventory.records.len(),
            inventory.discovered_boundary_count,
            inventory.review_status,
        ),
        (1, 0, 0, Some(String::from("waived")))
    );
}

#[test]
fn validated_inventory_with_review_status_is_equal_to_same_explicit_empty_status() {
    let constructor = ValidatedBoundaryInventory::with_review_status("approved");
    let explicit = validated_empty_with_status("approved");

    assert_eq!(constructor, explicit);
}
