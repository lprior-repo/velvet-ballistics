//! Linear step emission functions.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::{CompiledNode, CompiledNodeKind, ConstIdx, SlotIdx, StepIdx, SymbolId};

pub fn emit_linear_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::Nop => emit_nop_step(out, node.next),
        CompiledNodeKind::SetConst { value } => {
            emit_set_const_step(out, node.output, *value, node.next)
        }
        CompiledNodeKind::Copy { source } => emit_copy_step(out, node.output, *source, node.next),
        CompiledNodeKind::EvalExpr { expr } => {
            emit_eval_expr_step(out, node.output, *expr, node.next)
        }
        CompiledNodeKind::Finish { result } => emit_finish_step(out, *result),
        CompiledNodeKind::Jump { target } => emit_continue_step(out, *target),
        CompiledNodeKind::BuildObject { fields } => emit_build_object_step(out, node.output, fields, node.next),
        CompiledNodeKind::BuildList { items } => emit_build_list_step(out, node.output, items, node.next),
        _ => super::emit_unsupported::emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_nop_step(out: &mut String, next: Option<StepIdx>) -> CodegenResult<()> {
    match next {
        Some(next_step) => emit_continue_step(out, next_step),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

pub fn emit_set_const_step(
    out: &mut String,
    output: Option<SlotIdx>,
    value: ConstIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    write_slot(slots, {}, Some(read_const({})?))?;",
            output_slot.get(),
            value.get()
        )
        .map_err(fmt_err)?;
    }
    super::helpers::write_next_or_error(out, next)
}

pub fn emit_copy_step(
    out: &mut String,
    output: Option<SlotIdx>,
    source: SlotIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    let copied = read_slot_optional(slots, {});\n    write_slot(slots, {}, copied)?;",
            source.get(),
            output_slot.get()
        )
        .map_err(fmt_err)?;
    }
    super::helpers::write_next_or_error(out, next)
}

pub fn emit_eval_expr_step(
    out: &mut String,
    output: Option<SlotIdx>,
    expr: vb_core::ExprIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    write_slot(slots, {}, Some(eval_expr_{}(slots)?))?;",
            output_slot.get(),
            expr.get()
        )
        .map_err(fmt_err)?;
    }
    super::helpers::write_next_or_error(out, next)
}

fn emit_finish_step(out: &mut String, result: SlotIdx) -> CodegenResult<()> {
    writeln!(out, "    let value = read_slot(slots, {})?;", result.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Finished(value))").map_err(fmt_err)
}

pub fn emit_build_object_step(
    out: &mut String,
    output: Option<SlotIdx>,
    fields: &[(SymbolId, SlotIdx)],
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    let field_count = fields.len();
    writeln!(out, "    // BuildObject with {} fields", field_count).map_err(fmt_err)?;
    for (key, slot) in fields {
        writeln!(
            out,
            "    let _field_{} = read_slot(slots, {})?;",
            key.get(),
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if output.is_some() {
        writeln!(
            out,
            "    return Err(DriveError::UnsupportedPrimitive {{ primitive: \"BuildObject requires value-store context\" }});"
        )
        .map_err(fmt_err)?;
    }
    super::helpers::write_next_or_error(out, next)
}

pub fn emit_build_list_step(
    out: &mut String,
    output: Option<SlotIdx>,
    items: &[SlotIdx],
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    let item_count = items.len();
    writeln!(out, "    // BuildList with {} items", item_count).map_err(fmt_err)?;
    for (i, slot) in items.iter().enumerate() {
        writeln!(
            out,
            "    let _item_{} = read_slot(slots, {})?;",
            i,
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if output.is_some() {
        writeln!(
            out,
            "    return Err(DriveError::UnsupportedPrimitive {{ primitive: \"BuildList requires value-store context\" }});"
        )
        .map_err(fmt_err)?;
    }
    super::helpers::write_next_or_error(out, next)
}

pub(crate) fn emit_continue_step(out: &mut String, target: StepIdx) -> CodegenResult<()> {
    writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)
}
