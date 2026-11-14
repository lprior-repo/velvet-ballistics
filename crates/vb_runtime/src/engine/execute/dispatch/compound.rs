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
    CollectStartRequest, ReduceStartRequest, handle_collect_finish, handle_collect_next,
    handle_collect_page, handle_collect_start, handle_reduce_finish, handle_reduce_next,
    handle_reduce_start, handle_repeat_attempt, handle_repeat_check, handle_repeat_finish,
    handle_repeat_start,
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
        CompiledNodeKind::CollectStart { .. } => {
            dispatch_collect_start(run, store, collect_states, node)
        }
        CompiledNodeKind::CollectPage { .. } => {
            dispatch_collect_page(run, store, collect_states, node)
        }
        CompiledNodeKind::CollectNext { .. } => {
            dispatch_collect_next::<P>(run, store, collect_states, node)
        }
        CompiledNodeKind::CollectFinish { .. } => {
            dispatch_collect_finish(run, collect_states, node)
        }
        _ => Err(family_invariant()),
    }
}

fn dispatch_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    handle_collect_start(run, store, states, collect_start_request(node)?)
}

fn collect_start_request(
    node: &CompiledNode,
) -> Result<CollectStartRequest, crate::engine::types::RuntimeEngineError> {
    let CompiledNodeKind::CollectStart {
        source,
        limit,
        page_size,
        body,
        done,
    } = &node.kind
    else {
        return Err(family_invariant());
    };
    Ok(CollectStartRequest {
        source: *source,
        limit: limit.get(),
        page_size: *page_size,
        body: *body,
        done: *done,
        output: node.output.slot(),
    })
}

fn dispatch_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    let CompiledNodeKind::CollectPage {
        collector_slot,
        body,
        done,
    } = &node.kind
    else {
        return Err(family_invariant());
    };
    handle_collect_page(run, store, states, *collector_slot, *body, *done)
}

fn dispatch_collect_next<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    let CompiledNodeKind::CollectNext {
        collector_slot,
        body,
        done,
    } = &node.kind
    else {
        return Err(family_invariant());
    };
    handle_collect_next::<P>(run, store, states, *collector_slot, *body, *done)
}

fn dispatch_collect_finish(
    run: &mut RunFrame,
    states: &mut CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    let CompiledNodeKind::CollectFinish { collector_slot } = &node.kind else {
        return Err(family_invariant());
    };
    handle_collect_finish(
        run,
        states,
        *collector_slot,
        node.output.slot(),
        node.routing.next.step(),
        node.id,
    )
}

pub(super) fn dispatch_reduce<P: BodyEntryPolicy>(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ReduceStart { .. } => dispatch_reduce_start(plan, run, store, node),
        CompiledNodeKind::ReduceNext { .. } => dispatch_reduce_next::<P>(run, store, node),
        CompiledNodeKind::ReduceFinish { .. } => dispatch_reduce_finish(run, node),
        _ => Err(family_invariant()),
    }
}

fn dispatch_reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    handle_reduce_start(plan, run, store, reduce_start_request(node)?)
}

fn reduce_start_request(
    node: &CompiledNode,
) -> Result<ReduceStartRequest, crate::engine::types::RuntimeEngineError> {
    let CompiledNodeKind::ReduceStart {
        input,
        accumulator,
        initial,
        limit,
        body,
        done,
    } = &node.kind
    else {
        return Err(family_invariant());
    };
    Ok(ReduceStartRequest {
        input: *input,
        accumulator: *accumulator,
        initial: *initial,
        limit: *limit,
        body: *body,
        done: *done,
        output: node.output.slot(),
    })
}

fn dispatch_reduce_next<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    let CompiledNodeKind::ReduceNext {
        iterator_slot,
        accumulator,
        body,
        done,
    } = &node.kind
    else {
        return Err(family_invariant());
    };
    handle_reduce_next::<P>(
        run,
        store,
        *iterator_slot,
        *accumulator,
        *body,
        *done,
        node.output.slot(),
    )
}

fn dispatch_reduce_finish(
    run: &mut RunFrame,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    let CompiledNodeKind::ReduceFinish { accumulator } = &node.kind else {
        return Err(family_invariant());
    };
    handle_reduce_finish(
        run,
        *accumulator,
        node.output.slot(),
        node.routing.next.step(),
        node.id,
    )
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
        } => handle_repeat_start(run, max_attempts.get(), *body, *done, node.output.slot()),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => handle_repeat_attempt(run, *attempt_slot, *body, *done),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            handle_repeat_check::<P>(run, *attempt_slot, *done, node.routing.next.step(), node.id)
        }
        CompiledNodeKind::RepeatFinish { result } => handle_repeat_finish(
            run,
            *result,
            node.output.slot(),
            node.routing.next.step(),
            node.id,
        ),
        _ => Err(family_invariant()),
    }
}
