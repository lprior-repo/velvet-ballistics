#![forbid(unsafe_code)]
//! Plan verifier gates for compiled workflow IR (Section 63 of the master doc).
//!
//! Gates 7, 8, 9, 11, and 13 validate structural properties of `WorkflowParts`
//! that the core `validate_parts` function does not cover or that need additional
//! cold-path checks for the accepted-artifact pipeline.

// Re-export the core types we need so callers only depend on vb_validate.
pub use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
pub use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

/// Maximum byte length for a compiled action capability requirement name.
pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

mod gate_07;
mod gate_08;
mod gate_09;
mod gate_10;
mod gate_10_refs;
mod gate_11;
mod gate_11_spans;
mod gate_12;
mod gate_13;
mod gate_14;
mod gate_15;

pub use gate_07::{compute_stack_depth, validate_gate_07_expression_stack_depth};
pub use gate_08::validate_gate_08_accessor_path_segments;
pub use gate_09::validate_gate_09_slot_references;
pub use gate_10::validate_gate_10_node_kind_specific;
pub use gate_11::validate_gate_11_loop_body_graph;
pub use gate_12::validate_gate_12_action_contract_completeness;
pub use gate_13::validate_gate_13_no_slot_cycles;
pub use gate_14::validate_gate_14_slot_type_consistency;
pub use gate_15::validate_gate_15_determinism_proof;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

#[cfg(any(kani, test))]
use gate_08::validate_accessor_paths;

#[cfg(kani)]
#[path = "../verification/kani_gate_08_projection.rs"]
mod gate_08_projection;

#[cfg(test)]
#[path = "../gate_08_projection_tests.rs"]
mod gate_08_projection_tests;
