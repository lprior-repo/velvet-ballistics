//! Compound-family dispatchers (`Collect` / `Reduce` / `Repeat`).
//!
//! Split out of `dispatch.rs` to keep the orchestrator and the primitive
//! families under the source-length limit. These helpers own the arms for
//! the collect, reduce, and repeat `CompiledNodeKind` families and share
//! the parent's defensive [`family_invariant`] typed error for their
//! unreachable `_` arms.

use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::handlers_compound::{
    handle_collect_finish, handle_collect_next, handle_collect_page, handle_collect_start,
    handle_reduce_finish, handle_reduce_next, handle_reduce_start, handle_repeat_attempt,
    handle_repeat_check, handle_repeat_finish, handle_repeat_start,
};
use crate::engine::types::{RuntimeEngineResult, RuntimeSignal};
use crate::primitives::body_entry::BodyEntryPolicy;
use crate::primitives::collect::CollectStates;

use super::family_invariant;

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_collect<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collect_states: &mut CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => handle_collect_start(
            run,
            store,
            collect_states,
            *source,
            *limit,
            *page_size,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => handle_collect_page(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => handle_collect_next::<P>(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => handle_collect_finish(
            run,
            collect_states,
            *collector_slot,
            node.output,
            node.next,
            node.id,
        ),
        _ => Err(family_invariant()),
    }
}

pub(super) fn dispatch_reduce<P: BodyEntryPolicy>(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => handle_reduce_start(
            plan,
            run,
            store,
            *input,
            *accumulator,
            *initial,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => handle_reduce_next::<P>(
            run,
            store,
            *iterator_slot,
            *accumulator,
            *body,
            *done,
            node.output,
        ),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            handle_reduce_finish(run, *accumulator, node.output, node.next, node.id)
        }
        _ => Err(family_invariant()),
    }
}

pub(super) fn dispatch_repeat<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => handle_repeat_start(run, *max_attempts, *body, *done, node.output),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => handle_repeat_attempt(run, *attempt_slot, *body, *done),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            handle_repeat_check::<P>(run, *attempt_slot, *done, node.next, node.id)
        }
        CompiledNodeKind::RepeatFinish { result } => {
            handle_repeat_finish(run, *result, node.output, node.next, node.id)
        }
        _ => Err(family_invariant()),
    }
}
