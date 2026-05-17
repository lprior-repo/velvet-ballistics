#![forbid(unsafe_code)]

fn main() {
    let source = include_str!("../../../crates/vb_core/src/budget.rs");
    assert!(source.contains("AggregateResourceBudget"));
    assert!(source.contains("from_workflow"));
}
