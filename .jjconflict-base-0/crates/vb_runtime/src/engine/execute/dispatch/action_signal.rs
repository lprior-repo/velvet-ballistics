use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::budget::handle_retry_check;
use crate::engine::execute::handlers_suspend::{
    handle_ask, handle_ask_resume, handle_do, handle_error_handler, handle_wait_event,
    handle_wait_until,
};
use crate::engine::execute::signals::handle_core_step_once;
use crate::engine::types::{RetryPolicy, RuntimeEngineResult, RuntimeSignal};

use super::family_invariant;

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_action_and_signal(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    granted: &CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. } => dispatch_signal(run, node),
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::ErrorHandler { .. } => {
            dispatch_action(run, node, contracts, retry_policy, granted)
        }
        _ => handle_core_step_once(plan, run, store),
    }
}

fn dispatch_signal(run: &mut RunFrame, node: &CompiledNode) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } => handle_wait_until(run, *deadline_slot),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => handle_wait_event(run, *event, *timeout_slot),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => handle_ask(run, *prompt, *timeout_slot),
        CompiledNodeKind::AskResume { answer } => {
            handle_ask_resume(run, *answer, node.output, node.next, node.id)
        }
        _ => Err(family_invariant()),
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action(
    run: &mut RunFrame,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    granted: &CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::Do { action, input } => handle_do(
            run,
            *action,
            *input,
            contracts,
            granted,
            retry_policy,
            node.id,
        ),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => handle_retry_check(run, *policy_slot, *body, *exhausted, retry_policy),
        CompiledNodeKind::ErrorHandler {
            body: handler_body, ..
        } => handle_error_handler(run, *handler_body),
        _ => Err(family_invariant()),
    }
}
