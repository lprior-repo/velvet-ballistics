#![forbid(unsafe_code)]
//! Compiled workflow IR.

pub mod accessor;
pub mod admission_kernel;
pub mod branch;
pub mod compiled_query;
pub mod compiled_slug;
pub mod error;
pub mod expr;
pub mod lifecycle;
pub mod node;
pub mod resource_contract;
pub mod validation;
pub mod workflow;

#[cfg(kani)]
mod compiled_empty_path_kani;
#[cfg(kani)]
mod compiled_query_kani;
#[cfg(kani)]
mod compiled_slug_kani;
#[cfg(kani)]
mod compiled_total_cost_kani;
#[cfg(all(kani, feature = "kani-vb-ajc40"))]
mod kani_vb_dzibx_ajc40_admission; // RPO-AJC40-004 verifier-only harness module.

// Re-export commonly used types at the workflow level for ergonomic API surface.
pub use crate::workflow::accessor::{AccessorProgram, PathSegment};
pub use crate::workflow::branch::{ExprBranch, SlotBranch};
pub use crate::workflow::error::WorkflowError;
pub use crate::workflow::expr::{check_expr_stack_bound, ExprOp, ExprProgram};
pub use crate::workflow::lifecycle::{
    check_lifecycle_transition, LifecycleCommand, LifecycleState, RunState,
};
pub use crate::workflow::node::{CompiledNode, CompiledNodeKind};
pub use crate::workflow::resource_contract::ResourceContract;
pub use crate::workflow::workflow::{CompiledWorkflow, WorkflowParts};

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

#[cfg(test)]
#[path = "vb_dzibx_ajc40_admission_props.rs"]
mod vb_dzibx_ajc40_admission_props; // RPO-AJC40-002 verifier-only proptest module.
