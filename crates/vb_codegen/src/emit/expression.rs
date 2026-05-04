//! Expression emission functions for code generation.

use std::fmt::Write;
use crate::{CodegenResult, fmt_err};
use crate::helpers::{emit_accessor_eval, emit_unsupported_expr};
use vb_core::{CompiledWorkflow, ExprOp};

pub(crate) fn emit_expr_function(
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
                emit_accessor_eval(out, *accessor_idx, workflow)?;
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
            ExprOp::Contains => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_contains(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::StartsWith => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_starts_with(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::EndsWith => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_ends_with(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Has => {
                writeln!(out, "    {{ let _key = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _obj = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_obj, &_key) {{ (SlotValue::Object(_), SlotValue::Symbol(_)) => true, (SlotValue::List(_), SlotValue::I64(_)) => true, _ => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Exists => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(!matches!(_v, SlotValue::Null)))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Length => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _len = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_len))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Empty => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _is_empty = match _v {{ SlotValue::List(n) => n == 0, SlotValue::Object(n) => n == 0, SlotValue::Null => true, _ => false }}; stack.push(SlotValue::Bool(_is_empty))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Append => emit_unsupported_expr(out, "append")?,
            ExprOp::AppendIf => emit_unsupported_expr(out, "append_if")?,
            ExprOp::Merge => emit_unsupported_expr(out, "merge")?,
            ExprOp::Sum => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Count => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Unique => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
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

// Generate action dispatch boundaries for external action nodes.
