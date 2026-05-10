#![forbid(unsafe_code)]

fn main() {
    let source = include_str!("../../../crates/vb_core/src/budget.rs");
    assert_eq!(source.contains("AggregateResourceBudget"), true);
    assert_eq!(source.contains("from_workflow"), true);
}
