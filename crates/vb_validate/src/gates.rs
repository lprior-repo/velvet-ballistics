//! Plan verifier gates for compiled workflow IR (Section 63 of the master doc).
//!
//! Gates 7, 8, 9, 11, and 13 validate structural properties of `WorkflowParts`
//! that the core `validate_parts` function does not cover or that need additional
//! cold-path checks for the accepted-artifact pipeline.

// Re-exports for backwards compatibility - gates are declared in lib.rs
pub use crate::gate_action::validate_gate_12_action_contract_completeness;
pub use crate::gate_accessor::validate_gate_08_accessor_path_segments;
pub use crate::gate_cycle::{node_reads, validate_gate_13_no_slot_cycles};
pub use crate::gate_expr::{
    compute_stack_depth, pop_count, push_count, stack_effect,
    validate_gate_07_expression_stack_depth,
};
pub use crate::gate_loop::validate_gate_11_loop_body_graph;
pub use crate::gate_node::validate_gate_10_node_kind_specific;
pub use crate::gate_slot::validate_gate_09_slot_references;
pub use crate::gate_type::{
    validate_gate_14_slot_type_consistency, validate_gate_15_determinism_proof,
};
