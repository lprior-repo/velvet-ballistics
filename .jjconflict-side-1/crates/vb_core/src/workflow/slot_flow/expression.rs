use super::facts::invalid;
use super::{Context, Fact, FlowResult, SlotFlowFailure, capacity};
use crate::ids::ExprIdx;
use crate::workflow::{ExprOp as Op, PathSegment};

pub(super) fn infer(ctx: &Context<'_>, expr: ExprIdx) -> FlowResult<Fact> {
    let program = ctx
        .parts
        .expressions
        .get(expr.as_usize())
        .ok_or_else(|| invalid(ctx.node.id))?;
    crate::workflow::check_expr_stack_bound(&program.ops, program.max_stack)
        .map_err(|_| invalid(ctx.node.id))?;
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(usize::from(program.max_stack))
        .map_err(|_| capacity())?;
    for op in &program.ops {
        let result = match *op {
            Op::LoadSlot(slot) => ctx.read(slot)?,
            Op::LoadConst(value) => Fact::constant(
                ctx.parts
                    .constants
                    .get(value.as_usize())
                    .ok_or_else(|| invalid(ctx.node.id))?,
            ),
            Op::LoadAccessor(accessor) => {
                let program = ctx
                    .parts
                    .accessors
                    .get(accessor.as_usize())
                    .ok_or_else(|| invalid(ctx.node.id))?;
                if program.path.len() > crate::limits::MAX_PATH_DEPTH {
                    return Err(invalid(ctx.node.id));
                }
                let mut value = ctx.read(program.root)?;
                for segment in &program.path {
                    ctx.require(
                        value,
                        match segment {
                            PathSegment::Field(_) => Fact::OBJECT,
                            PathSegment::Index(_) => Fact::LIST,
                        },
                    )?;
                    value = Fact::assigned(Fact::DYNAMIC);
                }
                value
            }
            other => operator(ctx, &mut stack, other)?,
        };
        stack.push(result);
    }
    stack.pop().ok_or_else(|| invalid(ctx.node.id))
}

fn pop(ctx: &Context<'_>, stack: &mut Vec<Fact>) -> FlowResult<Fact> {
    stack.pop().ok_or_else(|| invalid(ctx.node.id))
}

fn operator(ctx: &Context<'_>, stack: &mut Vec<Fact>, op: Op) -> FlowResult<Fact> {
    let right = pop(ctx, stack)?;
    let output = match op {
        Op::Not => {
            ctx.require(right, Fact::BOOL)?;
            Fact::BOOL
        }
        Op::Exists => Fact::BOOL,
        Op::Length | Op::Empty => {
            ctx.require(right, Fact::NULL | Fact::SYMBOL | Fact::LIST | Fact::OBJECT)?;
            if op == Op::Length {
                Fact::I64
            } else {
                Fact::BOOL
            }
        }
        Op::Count => {
            ctx.require(right, Fact::NULL | Fact::LIST)?;
            Fact::I64
        }
        Op::Sum | Op::Unique => {
            ctx.require(right, Fact::LIST)?;
            if op == Op::Sum { Fact::I64 } else { Fact::LIST }
        }
        Op::AppendIf => {
            ctx.require(right, Fact::BOOL)?;
            pop(ctx, stack)?;
            ctx.require(pop(ctx, stack)?, Fact::LIST)?;
            Fact::LIST
        }
        other => binary(ctx, pop(ctx, stack)?, right, other)?,
    };
    Ok(Fact::assigned(output))
}

fn binary(ctx: &Context<'_>, left: Fact, right: Fact, op: Op) -> FlowResult<u16> {
    let (allowed_left, allowed_right, result) = match op {
        Op::Eq | Op::NotEq => return Ok(Fact::BOOL),
        Op::And | Op::Or => (Fact::BOOL, Fact::BOOL, Fact::BOOL),
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let number = Fact::I64 | Fact::F64;
            ctx.require(left, number)?;
            ctx.require(right, number)?;
            let integer = left.kinds & (Fact::I64 | Fact::DYNAMIC) != 0
                && right.kinds & (Fact::I64 | Fact::DYNAMIC) != 0;
            let float = (left.kinds | right.kinds) & (Fact::F64 | Fact::DYNAMIC) != 0;
            return Ok((if integer { Fact::I64 } else { 0 }) | (if float { Fact::F64 } else { 0 }));
        }
        Op::Gt | Op::Gte | Op::Lt | Op::Lte => {
            (Fact::I64 | Fact::F64, Fact::I64 | Fact::F64, Fact::BOOL)
        }
        Op::Contains => {
            ctx.require(left, Fact::SYMBOL | Fact::LIST)?;
            if left.kinds & Fact::SYMBOL != 0 {
                ctx.require(right, Fact::SYMBOL)?;
            }
            return Ok(Fact::BOOL);
        }
        Op::StartsWith | Op::EndsWith => (Fact::SYMBOL, Fact::SYMBOL, Fact::BOOL),
        Op::Has => (Fact::OBJECT, Fact::SYMBOL, Fact::BOOL),
        Op::Append => {
            ctx.require(left, Fact::LIST)?;
            return Ok(Fact::LIST);
        }
        Op::Merge => (Fact::OBJECT, Fact::OBJECT, Fact::OBJECT),
        _ => return Err(ctx.error(None, SlotFlowFailure::UnsupportedSemantics)),
    };
    ctx.require(left, allowed_left)?;
    ctx.require(right, allowed_right)?;
    Ok(result)
}
