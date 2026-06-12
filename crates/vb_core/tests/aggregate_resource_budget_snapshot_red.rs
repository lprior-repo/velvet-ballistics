const BUDGET_RS: &str = concat!(
    include_str!("../src/budget/mod.rs"),
    include_str!("../src/budget/aggregate_budget.rs"),
    include_str!("../src/budget/aggregate_usage.rs"),
    include_str!("../src/budget/aggregate_usage_checks.rs"),
    include_str!("../src/budget/budget_error.rs"),
    include_str!("../src/budget/policy.rs"),
    include_str!("../src/budget/small_linear.rs"),
    include_str!("../src/budget/traversal.rs"),
    include_str!("../src/budget/traversal_fanout.rs"),
    include_str!("../src/budget/traversal_loop.rs"),
    include_str!("../src/budget/traversal_path.rs"),
    include_str!("../src/budget/traversal_step_count.rs"),
    include_str!("../src/budget/traversal_successors.rs"),
    include_str!("../src/budget/traversal_tracking.rs"),
    include_str!("../src/budget/types.rs"),
    include_str!("../src/budget/validation.rs"),
);
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
