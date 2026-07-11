const BUDGET_RS: &str = include_str!("../src/budget.rs");
// `vb_runtime/src/admission.rs` is a 56-line shell that `include!`s
// focused chunks from `admission/parts/`. `include_str!` of the shell
// alone would miss contract tokens declared in the chunks, so we
// concatenate the shell + all chunks into `ADMISSION_RS` here at
// compile time. Mirrors the fix in `aggregate_resource_budget_red.rs`.
const ADMISSION_RS: &str = concat!(
    include_str!("../../vb_runtime/src/admission.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_002_records.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_003_stores.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_004_validation.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_005_admit_core.rs"),
    "\n",
    include_str!("../../vb_runtime/src/admission/parts/chunk_006_admit_budget.rs"),
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
