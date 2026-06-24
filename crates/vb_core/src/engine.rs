#![forbid(unsafe_code)]
//! Synchronous in-memory state-machine loop.

pub(crate) mod choose;
pub(crate) mod error_routing;
pub(crate) mod expr_eval;
pub(crate) mod node_helpers;
pub(crate) mod object_list;
pub(crate) mod run_loop;
pub(crate) mod signals;
pub(crate) mod step;
pub(crate) mod validate;

pub use crate::errors::EngineError;
pub use crate::frame::RunFrame;
pub use crate::value_store::ValueStore;
pub use crate::workflow::CompiledWorkflow;
pub use error_routing::{ErrorHandlerOutcome, ErrorSlotData, route_error_handler};
pub use expr_eval::eval_accessor;
pub use expr_eval::eval_accessor_with_store;
pub use expr_eval::eval_expr;
pub use expr_eval::eval_expr_with_store;
pub use object_list::build_list;
pub use object_list::build_list as build_list_impl;
pub use object_list::build_object;
pub use object_list::build_object as build_object_impl;
pub use run_loop::{drive_deterministic, run_until_blocked};
pub use signals::{EngineSignal, StepBudget};
pub use step::{
    journal_action_suspended, resume_action_completion, resume_action_failure, step_once,
};
pub use validate::{
    validate_compiled_workflow, validate_node_bounds, validate_no_nested_together,
    validate_resource_contract, validate_transition_target,
};

use crate::ids::RunId;

/// Creates a run frame for a compiled workflow.
pub fn new_run_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, EngineError> {
    RunFrame::new(
        run_id,
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
}

#[cfg(test)]
mod tests;
