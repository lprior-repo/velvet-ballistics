//! Branch step emission functions.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::{CompiledNodeKind, ExprBranch, SlotBranch, SlotIdx, StepIdx};

pub fn emit_branch_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => emit_choose_step(out, branches, *otherwise),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => emit_choose_slot_step(out, branches, *otherwise),
        _ => super::emit_unsupported::emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_choose_step(
    out: &mut String,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> CodegenResult<()> {
    for branch in branches {
        writeln!(
            out,
            "    if eval_expr_{}(slots)?.is_true() {{ return Ok(StepOutcome::Continue({})); }}",
            branch.condition.get(),
            branch.target.get()
        )
        .map_err(fmt_err)?;
    }
    emit_choice_fallback(out, otherwise)
}

fn emit_choose_slot_step(
    out: &mut String,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> CodegenResult<()> {
    for branch in branches {
        writeln!(
            out,
            "    if read_slot(slots, {})?.is_true() {{ return Ok(StepOutcome::Continue({})); }}",
            branch.condition.get(),
            branch.target.get()
        )
        .map_err(fmt_err)?;
    }
    emit_choice_fallback(out, otherwise)
}

pub fn emit_choice_fallback(out: &mut String, otherwise: Option<StepIdx>) -> CodegenResult<()> {
    match otherwise {
        Some(fallback) => super::emit_linear::emit_continue_step(out, fallback),
        None => writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err),
    }
}
