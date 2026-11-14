use vb_core::workflow::CompiledNodeKind;

use crate::engine::execute::handlers_compound::{
    handle_for_each_join, handle_for_each_next, handle_for_each_start, handle_together_branch,
    handle_together_join, handle_together_start,
};
use crate::engine::types::{RuntimeEngineResult, RuntimeSignal};
use crate::primitives::body_entry::BodyEntryPolicy;

use super::{NodeDispatchContext, family_invariant};

pub(super) fn dispatch_for_each<P: BodyEntryPolicy>(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachStart { .. } => dispatch_for_each_start(ctx),
        CompiledNodeKind::ForEachNext { .. } => dispatch_for_each_next::<P>(ctx),
        CompiledNodeKind::ForEachJoin { .. } => dispatch_for_each_join(ctx),
        _ => Err(family_invariant()),
    }
}

fn dispatch_for_each_start(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => handle_for_each_start(
            ctx.run,
            ctx.store,
            *input,
            *item_slot,
            limit.get(),
            *body,
            *done,
            ctx.node.output.slot(),
        ),
        _ => Err(family_invariant()),
    }
}

fn dispatch_for_each_next<P: BodyEntryPolicy>(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => handle_for_each_next::<P>(
            ctx.run,
            ctx.store,
            *iterator_slot,
            *body,
            *done,
            ctx.node.output.slot(),
        ),
        _ => Err(family_invariant()),
    }
}

fn dispatch_for_each_join(ctx: &mut NodeDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachJoin { output } => handle_for_each_join(
            ctx.run,
            *output,
            ctx.node.output.slot(),
            ctx.node.routing.next.step(),
            ctx.node.id,
        ),
        _ => Err(family_invariant()),
    }
}

pub(super) fn dispatch_together(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::TogetherStart { .. } => dispatch_together_start(ctx),
        CompiledNodeKind::TogetherBranch { .. } => dispatch_together_branch(ctx),
        CompiledNodeKind::TogetherJoin { .. } => dispatch_together_join(ctx),
        _ => Err(family_invariant()),
    }
}

fn dispatch_together_start(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            handle_together_start(ctx.run, ctx.store, branches, *join, ctx.node.output.slot())
        }
        _ => Err(family_invariant()),
    }
}

fn dispatch_together_branch(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => handle_together_branch(
            ctx.run,
            ctx.store,
            branch.get(),
            *entry,
            *join,
            *accumulator,
            ctx.node.output.slot(),
        ),
        _ => Err(family_invariant()),
    }
}

fn dispatch_together_join(ctx: &mut NodeDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => handle_together_join(
            ctx.run,
            ctx.store,
            branch_count.get(),
            *accumulator,
            ctx.node.output.slot(),
            ctx.node.routing.next.step(),
            ctx.node.id,
        ),
        _ => Err(family_invariant()),
    }
}
