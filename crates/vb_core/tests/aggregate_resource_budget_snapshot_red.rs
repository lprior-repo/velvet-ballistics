const BUDGET_RS: &str = include_str!("../src/budget.rs");
const ADMISSION_SHELL_RS: &str = include_str!("../../vb_runtime/src/admission.rs");
const ADMISSION_ERROR_RS: &str =
    include_str!("../../vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs");

fn admission_source() -> String {
    [ADMISSION_SHELL_RS, ADMISSION_ERROR_RS].join("\n")
}

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
    let admission = admission_source();
    let variants = [
        "ArtifactNotFound",
        "CapabilityDenied",
        "ResourceCapacityExceeded",
    ];
    let expected = variants.join("|");
    let observed: String = [
        admission.contains("ArtifactNotFound"),
        admission.contains("CapabilityDenied"),
        admission.contains("ResourceCapacityExceeded"),
    ]
    .into_iter()
    .zip(variants)
    .filter(|(present, _)| *present)
    .map(|(_, name)| name)
    .collect::<Vec<_>>()
    .join("|");

    assert_eq!(observed, expected);
}
