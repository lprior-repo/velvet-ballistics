#![forbid(unsafe_code)]
//! Compiled workflow IR.

#[cfg(test)]
pub(crate) use crate::errors::CoreError;

#[path = "concurrent/error.rs"]
mod concurrent_analysis_error;
#[path = "concurrent/analyze.rs"]
mod concurrent_analyze;
#[path = "concurrent/backedges.rs"]
mod concurrent_backedges;
pub(crate) mod concurrent_bootstrap;
#[path = "concurrent/cursor.rs"]
mod concurrent_cursor;
#[path = "concurrent/lane_end.rs"]
mod concurrent_lane_end;
#[path = "concurrent/lane_walk.rs"]
mod concurrent_lane_walk;
#[path = "concurrent/ownership.rs"]
mod concurrent_ownership;
#[path = "concurrent/pair.rs"]
mod concurrent_pair;
#[path = "concurrent/regions.rs"]
mod concurrent_regions;
#[path = "concurrent/start_entries.rs"]
mod concurrent_start_entries;
#[path = "concurrent/targets.rs"]
mod concurrent_targets;
#[path = "concurrent/together_policy.rs"]
mod concurrent_together_policy;
#[path = "concurrent/kind_refs.rs"]
mod kind_refs;
pub use concurrent_analysis_error::ConcurrentAnalysisError;
mod compiled;
#[path = "concurrent/group_frame_bound.rs"]
mod concurrent_group_frame_bound;
#[path = "concurrent/peaks.rs"]
mod concurrent_peaks;
mod ir_profile;
pub(crate) use compiled::resolved_error_slot;
pub use compiled::{
    CompiledWorkflow, ConcurrentRequirements, VersionedWorkflowParts, WorkflowIrV2,
};
pub use ir_profile::WorkflowIrProfile;

mod cursor_ownership;
mod input_constraints;
mod retry_policy;
pub use retry_policy::CompiledRetryPolicy;
mod slot_flow;
pub use slot_flow::{SlotFlowError, SlotFlowFailure, validate_slot_flow};
pub mod source_metadata;
pub use crate::input_schema::{CompiledInputField, InputDefaultValue};

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

pub(crate) mod loop_regions;
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
use validate::validate_input_constraint;
mod validate_node_kind;
mod validate_refs;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod proptest_collect_budget;

#[cfg(test)]
mod proptest_collect_traversal;
