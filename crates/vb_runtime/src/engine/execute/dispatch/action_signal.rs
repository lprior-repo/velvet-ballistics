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

/// Context for dispatching action and signal nodes.
/// Groups the 7 environment parameters into a single typed value.
pub(crate) struct ActionDispatchContext<'a> {
    pub plan: &'a CompiledWorkflow,
    pub run: &'a mut RunFrame,
    pub store: &'a mut ValueStore,
    pub node: &'a CompiledNode,
    pub contracts: &'a [ActionContract],
    pub retry_policy: RetryPolicy,
    pub granted: &'a CapabilitySet,
    pub state_run_attempt: u16,
}

pub(super) fn dispatch_action_and_signal(
    ctx: &mut ActionDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. } => dispatch_signal(ctx),
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::ErrorHandler { .. } => dispatch_action(ctx),
        _ => handle_core_step_once(ctx.plan, ctx.run, ctx.store),
    }
}

fn dispatch_signal(ctx: &mut ActionDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } => handle_wait_until(ctx.run, *deadline_slot),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => handle_wait_event(ctx.run, *event, *timeout_slot),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => handle_ask(ctx.run, *prompt, *timeout_slot),
        CompiledNodeKind::AskResume { answer } => handle_ask_resume(
            ctx.run,
            *answer,
            ctx.node.output,
            ctx.node.next,
            ctx.node.id,
        ),
        _ => Err(family_invariant()),
    }
}

fn dispatch_action(ctx: &mut ActionDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::Do { action, input } => handle_do(ctx, *action, *input, ctx.node.id),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => handle_retry_check(ctx.run, *policy_slot, *body, *exhausted, ctx.retry_policy),
        CompiledNodeKind::ErrorHandler {
            body: handler_body, ..
        } => handle_error_handler(ctx.run, *handler_body),
        _ => Err(family_invariant()),
    }
}