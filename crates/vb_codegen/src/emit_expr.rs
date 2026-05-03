//! Expression emission functions for generating expression evaluator functions.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::{CompiledWorkflow, ExprOp};

/// Generate an expression evaluator function.
pub fn emit_expr_function(
    out: &mut String,
    expr_idx: vb_core::ExprIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let Some(program) = workflow.expression(expr_idx) else {
        return Ok(());
    };

    writeln!(
        out,
        "fn eval_expr_{}(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<SlotValue, DriveError> {{",
        expr_idx.get()
    )
    .map_err(fmt_err)?;

    writeln!(
        out,
        "    let mut stack = ExprStack::new({})?;",
        program.max_stack
    )
    .map_err(fmt_err)?;

    for op in program.ops.as_ref() {
        match op {
            ExprOp::LoadSlot(slot) => {
                writeln!(out, "    stack.push(read_slot(slots, {})?)?;", slot.get())
                    .map_err(fmt_err)?;
            }
            ExprOp::LoadConst(const_idx) => {
                writeln!(out, "    stack.push(read_const({})?)?;", const_idx.get())
                    .map_err(fmt_err)?;
            }
            ExprOp::LoadAccessor(accessor_idx) => {
                crate::helpers::emit_accessor_eval(out, *accessor_idx, workflow)?;
            }
            ExprOp::Eq => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(_l == _r))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::NotEq => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(_l != _r))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gt => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li > _ri))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gte => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li >= _ri))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lt => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li < _ri))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lte => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li <= _ri))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::And => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_lb && _rb))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Or => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_lb || _rb))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Not => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; match _v {{ SlotValue::Bool(b) => stack.push(SlotValue::Bool(!b))?, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }} }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Add => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_add(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Sub => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_sub(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Mul => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_mul(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Div => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_div(_ri).ok_or(DriveError::DivisionByZero)?; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Contains => crate::emit_unsupported::emit_unsupported_expr(out, "contains")?,
            ExprOp::StartsWith => crate::emit_unsupported::emit_unsupported_expr(out, "starts_with")?,
            ExprOp::EndsWith => crate::emit_unsupported::emit_unsupported_expr(out, "ends_with")?,
            ExprOp::Has => crate::emit_unsupported::emit_unsupported_expr(out, "has")?,
            ExprOp::Exists => crate::emit_unsupported::emit_unsupported_expr(out, "exists")?,
            ExprOp::Length => crate::emit_unsupported::emit_unsupported_expr(out, "length")?,
            ExprOp::Empty => crate::emit_unsupported::emit_unsupported_expr(out, "empty")?,
            ExprOp::Append => crate::emit_unsupported::emit_unsupported_expr(out, "append")?,
            ExprOp::AppendIf => crate::emit_unsupported::emit_unsupported_expr(out, "append_if")?,
            ExprOp::Merge => crate::emit_unsupported::emit_unsupported_expr(out, "merge")?,
            ExprOp::Sum => crate::emit_unsupported::emit_unsupported_expr(out, "sum")?,
            ExprOp::Count => crate::emit_unsupported::emit_unsupported_expr(out, "count")?,
            ExprOp::Unique => crate::emit_unsupported::emit_unsupported_expr(out, "unique")?,
        }
    }

    writeln!(
        out,
        "    stack.pop().ok_or(DriveError::ExpressionStackUnderflow)"
    )
    .map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}
