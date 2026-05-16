//! Plan verifier gates for compiled workflow IR.
//!
//! Gates 7, 8, 9, 10, 11, 12, 13, and 14 validate structural properties
//! of `WorkflowParts` that the core `validate_parts` function does not cover.

pub use crate::{ValidationError, ValidationResult};

pub mod gate_07;
pub mod gate_08;
pub mod gate_09;
pub mod gate_10;
pub mod gate_11;
pub mod gate_12;
pub mod gate_13;
pub mod gate_14;

pub use gate_07::{
    validate_gate_07_expression_stack_depth, compute_stack_depth, MAX_CAPABILITY_NAME_BYTES,
};
pub use gate_08::validate_gate_08_accessor_path_segments;
pub use gate_09::validate_gate_09_slot_references;
pub use gate_11::validate_gate_11_loop_body_graph;
pub use gate_10::validate_gate_13_no_slot_cycles;
pub use gate_12::{validate_gate_12_action_contract_completeness, MAX_CAPABILITY_NAME_BYTES as GATE12_MAX_CAPABILITY_NAME_BYTES};
pub use gate_13::validate_gate_14_slot_type_consistency;
