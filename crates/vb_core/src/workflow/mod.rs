#![forbid(unsafe_code)]
//! Compiled workflow IR.

pub mod lifecycle;
pub mod types;
pub mod validation;

// Re-export commonly used types at the workflow level for ergonomic API surface.
pub use crate::workflow::lifecycle::{
    LifecycleCommand, LifecycleState, RunState, check_lifecycle_transition,
};
pub use crate::workflow::types::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
    ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound,
};

impl CompiledWorkflow {
    /// Creates a compiled workflow after validating all numeric references.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validation::validate_parts(&parts)?;
        validation::validate_budget(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "test_workflow_validation.rs"]
mod test_workflow_validation;

#[cfg(test)]
#[path = "test_workflow_errors.rs"]
mod test_workflow_errors;

#[cfg(test)]
#[path = "test_workflow_blake3.rs"]
mod test_workflow_blake3;

#[cfg(test)]
#[path = "proptest_workflow.rs"]
mod proptest_workflow;
