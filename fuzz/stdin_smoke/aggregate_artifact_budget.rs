#![forbid(unsafe_code)]

fn main() {
    let budget_source = include_str!("../../crates/vb_core/src/budget.rs");
    let admission_source = include_str!("../../crates/vb_runtime/src/admission.rs");
    assert!(budget_source.contains("from_whole_workflow_budget"));
    assert!(admission_source.contains("admit_run_with_budget"));
}
