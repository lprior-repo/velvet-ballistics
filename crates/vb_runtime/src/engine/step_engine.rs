#![forbid(unsafe_code)]

//! Step-level execution engine for compiled workflow nodes.

mod action_engine;
mod iteration_engine;

pub use action_engine::{
    execute_do, execute_do_node, execute_do_without_contract, resume_action_outcome,
};
pub use iteration_engine::execute_iteration_node;

use vb_core::action::ActionContract;
use vb_core::engine::step_once;
use vb_core::engine::{EngineSignal, StepBudget};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::signals::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::engine::transition::execute_retry_check;
use crate::engine::RetryPolicy;
use crate::primitives;
use action_engine::resolve_action_contract as resolve_contract;
use iteration_engine::runtime_from_core;

/// Executes one compiled node with full primitive dispatch.
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &vb_core::workflow::CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        // Iteration/compound nodes
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. } => {
            iteration_engine::execute_iteration_node(plan, run, store, node)
        }

        // Repeat nodes
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => primitives::repeat::repeat_start(run, *max_attempts, *body, *done, node.output)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => primitives::repeat::repeat_attempt(run, *attempt_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            primitives::repeat::repeat_check(run, *attempt_slot, *done, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::RepeatFinish { result } => {
            primitives::repeat::repeat_finish(run, *result, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        // Wait/Ask nodes
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            primitives::wait_ask::wait_until(run, *deadline_slot)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => primitives::wait_ask::wait_event(run, *event, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => primitives::wait_ask::ask(run, *prompt, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::AskResume { answer } => {
            primitives::wait_ask::ask_resume(run, *answer, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        // Action node
        CompiledNodeKind::Do { action, input } => {
            action_engine::execute_do_node(run, node, *action, *input, contracts)
        }

        // Retry check
        CompiledNodeKind::RetryCheck {
            policy_slot: _,
            body,
            exhausted,
        } => {
            let target = execute_retry_check(1, retry_policy, *body, *exhausted);
            run.set_pc(target).map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        // Error handler
        CompiledNodeKind::ErrorHandler {
            body: handler_body,
            handler: _,
            ..
        } => {
            run.set_pc(*handler_body)
                .map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        // Fallback to core step_once for primitives not handled above
        _ => {
            let core_signal = step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
            Ok(runtime_from_core(core_signal))
        }
    }
}
