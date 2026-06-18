#![forbid(unsafe_code)]
//! Plan verifier gates for compiled workflow IR (Section 63 of the master doc).
//!
//! Gates 7 through 15 validate structural properties of `WorkflowParts`
//! that the core `validate_parts` function does not cover or that need
//! additional cold-path checks for the accepted-artifact pipeline.

pub mod gate_07;
pub mod gate_08;
pub mod gate_09;
pub mod gate_10;
pub mod gate_11;
pub mod gate_12;
pub mod gate_13;
pub mod gate_14;
pub mod gate_15;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports (for callers that depend on the monolithic `gates` API)
// ---------------------------------------------------------------------------

// Core types used by the gate APIs.
pub use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
pub use vb_core::workflow::{CompiledNode, 
    AccessorProgram, CompiledNodeKind, ExprOp, ExprProgram, PathSegment, WorkflowParts,
};

// Gate 7: Expression stack depth bounded.
pub use gate_07::{
    compute_stack_depth, stack_effect, validate_gate_07_expression_stack_depth, MAX_CAPABILITY_NAME_BYTES,
};

// Gate 8: Accessor path segments are valid symbols.
pub use gate_08::validate_gate_08_accessor_path_segments;

// Gate 9: All referenced slots exist within declared slot_count.
pub use gate_09::validate_gate_09_slot_references;

// Gate 10: Node-kind-specific constraints.
pub use gate_10::validate_gate_10_node_kind_specific;

// Gate 11: ForEach/Together body graph is well-formed.
pub use gate_11::validate_gate_11_loop_body_graph;

// Gate 12: Action contract completeness.
pub use gate_12::validate_gate_12_action_contract_completeness;

// Gate 13: No circular references in slot dependency graph.
pub use gate_13::validate_gate_13_no_slot_cycles;

// Gate 14: Slot type consistency.
pub use gate_14::validate_gate_14_slot_type_consistency;

// Gate 15: Determinism proof.
pub use gate_15::validate_gate_15_determinism_proof;

// Validation types used by gate tests.
pub use crate::{ValidationError, ValidationResult};
