use super::facts::invalid;
use super::graph::Route;
use super::{Context, Fact, FlowResult};
use crate::workflow::CompiledNodeKind as K;

pub(super) fn apply(ctx: &mut Context<'_>, route: Route) -> FlowResult<()> {
    match &ctx.node.kind {
        K::SetConst { value } => {
            let constant = ctx
                .parts
                .constants
                .get(value.as_usize())
                .ok_or_else(|| invalid(ctx.node.id))?;
            ctx.output(Fact::constant(constant))
        }
        K::Copy { source } | K::AskResume { answer: source } => {
            let fact = ctx.read(*source)?;
            if let Some(out) = ctx.node.output.slot() {
                ctx.write(out, fact, true)?;
            }
            Ok(())
        }
        K::EvalExpr { expr } => ctx.output(super::expression::infer(ctx, *expr)?),
        K::BuildObject { fields } => {
            for (_, slot) in fields {
                ctx.read(*slot)?;
            }
            ctx.output(Fact::assigned(Fact::OBJECT))
        }
        K::BuildList { items } => {
            for slot in items {
                ctx.read(*slot)?;
            }
            ctx.output(Fact::assigned(Fact::LIST))
        }
        K::ChooseSlot { branches, .. } => {
            for branch in branches {
                ctx.typed_read(branch.condition, Fact::BOOL)?;
            }
            Ok(())
        }
        K::Choose { branches, .. } => {
            for branch in branches {
                ctx.require(super::expression::infer(ctx, branch.condition)?, Fact::BOOL)?;
            }
            Ok(())
        }
        K::Do { input, .. } => {
            ctx.read(*input)?;
            // Failure completion does not write a success result.
            if let Some(out) = ctx.node.output.slot() {
                ctx.write(out, Fact::assigned(Fact::DYNAMIC), false)?;
            }
            Ok(())
        }
        K::WaitUntil { deadline_slot } => {
            ctx.typed_read(*deadline_slot, Fact::I64 | Fact::F64)?;
            Ok(())
        }
        K::WaitEvent {
            event,
            timeout_slot,
        } => {
            ctx.typed_read(*event, Fact::I64 | Fact::F64)?;
            timeout(ctx, *timeout_slot)
        }
        K::Ask {
            prompt,
            timeout_slot,
        } => {
            // Current wait_ask::validate_prompt accepts every non-Bool shape.
            ctx.typed_read(
                *prompt,
                Fact::NULL
                    | Fact::I64
                    | Fact::F64
                    | Fact::SYMBOL
                    | Fact::LIST
                    | Fact::OBJECT
                    | Fact::BLOB,
            )?;
            timeout(ctx, *timeout_slot)?;
            // Host authority comes from the immediate AskResume, even without Ask.output.
            // Failure edges retain the pre-answer state.
            if let Some(K::AskResume { answer }) = ctx
                .node
                .routing
                .next
                .step()
                .and_then(|next| ctx.parts.nodes.get(next.as_usize()))
                .map(|resume| &resume.kind)
            {
                ctx.write(*answer, Fact::assigned(Fact::DYNAMIC), false)?;
            }
            Ok(())
        }
        K::RetryCheck { policy_slot, .. } => {
            // Runtime's first visit interprets absence as attempt zero.
            ctx.require(ctx.peek(*policy_slot)?, Fact::I64)?;
            ctx.write(*policy_slot, Fact::assigned(Fact::I64), true)
        }
        K::Finish { result } => {
            ctx.read(*result)?;
            Ok(())
        }
        K::Nop | K::Jump { .. } | K::ErrorHandler { .. } => Ok(()),
        other => super::compound::apply(ctx, other, route),
    }
}

fn timeout(ctx: &Context<'_>, slot: Option<crate::ids::SlotIdx>) -> FlowResult<()> {
    if let Some(slot) = slot {
        ctx.typed_read(slot, Fact::I64 | Fact::F64)?;
    }
    Ok(())
}
