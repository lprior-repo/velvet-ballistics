const BUDGET_RS: &str = include_str!("../src/budget.rs");
const ADMISSION_RS: &str = concat!(
    include_str!("../../vb_runtime/src/admission.rs"),
    include_str!("../../vb_runtime/src/admission/errors.rs")
);

#[test]
fn aggregate_budget_error_shape_snapshot_matches_contract() {
    let variants = [
        "WorkflowBudget",
        "PolicyExceeded",
        "CapacityExceeded",
        "Overflow",
        "Underflow",
        "InvalidCapacity",
        "ReservationNotFound",
    ];
    let expected = variants.join("|");
    let observed: String = [
        BUDGET_RS.contains("WorkflowBudget"),
        BUDGET_RS.contains("PolicyExceeded"),
        BUDGET_RS.contains("CapacityExceeded"),
        BUDGET_RS.contains("Overflow"),
        BUDGET_RS.contains("Underflow"),
        BUDGET_RS.contains("InvalidCapacity"),
        BUDGET_RS.contains("ReservationNotFound"),
    ]
    .into_iter()
    .zip(variants)
    .filter(|(present, _)| *present)
    .map(|(_, name)| name)
    .collect::<Vec<_>>()
    .join("|");

    assert_eq!(observed, expected);
}

#[test]
fn runtime_admission_error_shape_snapshot_matches_contract() {
    let variants = [
        "ArtifactNotFound",
        "CapabilityDenied",
        "ResourceCapacityExceeded",
    ];
    let expected = variants.join("|");
    let observed: String = [
        ADMISSION_RS.contains("ArtifactNotFound"),
        ADMISSION_RS.contains("CapabilityDenied"),
        ADMISSION_RS.contains("ResourceCapacityExceeded"),
    ]
    .into_iter()
    .zip(variants)
    .filter(|(present, _)| *present)
    .map(|(_, name)| name)
    .collect::<Vec<_>>()
    .join("|");

    assert_eq!(observed, expected);
}
