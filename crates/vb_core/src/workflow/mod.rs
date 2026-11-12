#![forbid(unsafe_code)]
//! Compiled workflow IR.

#[cfg(test)]
pub(crate) use crate::errors::CoreError;

mod compiled;
pub use compiled::CompiledWorkflow;

mod constraints;
pub use constraints::{Bound, ExtraFields, Secretness};

mod error;
pub use error::WorkflowError;

mod expr_program;
pub use expr_program::{ExprOp, ExprProgram, check_expr_stack_bound};

mod forward_edges;
mod input_constraint;
pub use input_constraint::{CompiledInputConstraint, InputConstraintIssue};

mod input_slot;
pub use input_slot::{
    CompiledInputSlot, ExprBranch, InputSlotKind, Nullability, Presence, SlotBranch,
};

mod lifecycle;
pub use lifecycle::{LifecycleCommand, LifecycleState, RunState, check_lifecycle_transition};

mod loop_regions;
pub use loop_regions::{CompiledLoopRegion, LoopRegionSlot};

mod node;
pub use node::{CompiledNode, ErrorRouting, NodeContinuation, NodeOutput, NodeRouting};

mod node_assembly;
pub use node_assembly::{node, node_with_handler};

mod node_kind;
pub use node_kind::CompiledNodeKind;

mod parts;
pub use parts::{AccessorProgram, PathSegment, WorkflowParts};

mod repeat_check_validate;
mod repeat_counter_ownership;

mod resource_contract;
pub use resource_contract::{ContractResource, ResourceContract, SecretResultPolicy};

mod resource_limits;
pub(crate) use resource_limits::check_resource_contract_limits;

mod route_enums;
pub use route_enums::{BranchContinuation, Deadline, ErrorSink};

mod traversal;
mod validate;
#[cfg(test)]
pub(crate) use validate::validate_budget_result;
mod validate_node_kind;
mod validate_refs;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod proptest_collect_budget;

#[cfg(test)]
mod proptest_collect_traversal;
