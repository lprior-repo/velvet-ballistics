use super::facts::invalid;
use super::graph::Route;
use super::{Context, Fact, FlowResult, SlotFlowFailure};
use crate::workflow::CompiledNodeKind as K;

pub(super) fn apply(ctx: &mut Context<'_>, kind: &K, route: Route) -> FlowResult<()> {
    match *kind {
        K::ForEachStart {
            input, item_slot, ..
        } => {
            ctx.typed_read(input, Fact::LIST)?;
            if route == Route::Body {
                ctx.write(item_slot, Fact::assigned(Fact::DYNAMIC), true)?;
            }
            ctx.output(Fact::assigned(Fact::LIST))
        }
        K::ForEachNext { iterator_slot, .. } | K::ReduceNext { iterator_slot, .. } => {
            ctx.typed_read(iterator_slot, Fact::LIST)?;
            if route == Route::Body {
                ctx.output(Fact::assigned(Fact::DYNAMIC))?;
                ctx.write(iterator_slot, Fact::assigned(Fact::LIST), true)?;
            }
            Ok(())
        }
        K::ForEachJoin { output } => {
            let fact = ctx.typed_read(output, Fact::LIST)?;
            ctx.output(fact)
        }
        K::CollectStart { source, .. } => {
            ctx.typed_read(source, Fact::LIST)?;
            let collector = ctx
                .node
                .output
                .slot()
                .map_or(source, core::convert::identity);
            ctx.write(collector, Fact::assigned(Fact::LIST), true)
        }
        K::CollectPage { collector_slot, .. } => {
            ctx.typed_read(collector_slot, Fact::LIST)?;
            Ok(())
        }
        K::CollectNext { collector_slot, .. } => {
            ctx.typed_read(collector_slot, Fact::LIST)?;
            ctx.write(collector_slot, Fact::assigned(Fact::LIST), true)
        }
        K::CollectFinish { collector_slot } => {
            let fact = ctx.read(collector_slot)?;
            ctx.output(fact)
        }
        K::ReduceStart {
            input,
            accumulator,
            initial,
            ..
        } => {
            ctx.typed_read(input, Fact::LIST)?;
            let init = ctx
                .parts
                .constants
                .get(initial.as_usize())
                .ok_or_else(|| invalid(ctx.node.id))?;
            ctx.write(accumulator, Fact::constant(init), true)?;
            if route == Route::Body {
                ctx.write(input, Fact::assigned(Fact::DYNAMIC), true)?;
                ctx.output(Fact::assigned(Fact::LIST))?;
            }
            Ok(())
        }
        K::ReduceFinish { accumulator } => {
            let fact = ctx.read(accumulator)?;
            ctx.output(fact)
        }
        K::RepeatStart { .. } => ctx.output(Fact::assigned(Fact::I64)),
        K::RepeatAttempt { attempt_slot, .. } => {
            ctx.typed_read(attempt_slot, Fact::I64)?;
            Ok(())
        }
        K::RepeatCheck { attempt_slot, .. } => {
            ctx.typed_read(attempt_slot, Fact::I64)?;
            ctx.write(attempt_slot, Fact::assigned(Fact::I64), true)
        }
        K::RepeatFinish { result } => {
            let fact = ctx.read(result)?;
            ctx.output(fact)
        }
        K::TogetherBranch {
            branch,
            accumulator,
            ..
        } => {
            if branch != 0 {
                let out = ctx.node.output.slot().ok_or_else(|| invalid(ctx.node.id))?;
                ctx.read(out)?;
                ctx.typed_read(accumulator, Fact::LIST)?;
                ctx.write(accumulator, Fact::assigned(Fact::LIST), true)?;
            }
            Ok(())
        }
        K::TogetherStart { ref branches, .. } => {
            if branches.is_empty() {
                return Err(invalid(ctx.node.id));
            }
            ctx.output(Fact::assigned(Fact::LIST))
        }
        K::TogetherJoin { accumulator, .. } => {
            let fact = ctx.read(accumulator)?;
            // Runtime joins the output's prior taint even for scalar fallback.
            let out = ctx.node.output.slot().ok_or_else(|| invalid(ctx.node.id))?;
            ctx.read(out)?;
            if fact.kinds & (Fact::LIST | Fact::DYNAMIC) != 0 {
                ctx.write(accumulator, fact, true)?;
            }
            ctx.output(fact)
        }
        _ => Err(ctx.error(None, SlotFlowFailure::UnsupportedSemantics)),
    }
}
