use super::graph::Route;
use super::{Context, FlowResult};
use crate::workflow::CompiledNodeKind as K;

// The executable drive observes outputs even for control nodes that do not
// write them. Loop evidence has branch-specific exceptions to that rule.
pub(super) fn check(ctx: &Context<'_>, route: Route) -> FlowResult<()> {
    match ctx.node.kind {
        K::ReduceStart { accumulator, .. } => {
            ctx.read(accumulator)?;
            if route == Route::Body {
                output(ctx)?;
            }
            Ok(())
        }
        K::ForEachNext { .. } | K::ReduceNext { .. } if route == Route::Done => Ok(()),
        K::RepeatCheck { attempt_slot, .. } => {
            ctx.read(attempt_slot)?;
            Ok(())
        }
        K::CollectNext { collector_slot, .. } => {
            ctx.read(collector_slot)?;
            Ok(())
        }
        _ => output(ctx),
    }
}

fn output(ctx: &Context<'_>) -> FlowResult<()> {
    if let Some(slot) = ctx.node.output.slot() {
        ctx.read(slot)?;
    }
    Ok(())
}
