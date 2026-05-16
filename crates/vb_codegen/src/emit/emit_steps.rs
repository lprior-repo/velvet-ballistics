//! Step function emission orchestration for generated Rust.

use std::fmt::Write;

use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::CodegenResult;
use crate::emit::fmt_err;
use crate::emit::emit_step_bodies::{
    emit_linear_step_body, emit_branch_step_body, emit_boundary_step_body,
    emit_unsupported_node_step,
};

/// Generate a per-step function for one compiled node.
#[allow(unreachable_pub)]
pub fn emit_step_function(
    out: &mut String,
    node: &CompiledNode,
    _workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let step_id = node.id.get();
    writeln!(
        out,
        "fn step_{step_id}(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<StepOutcome, DriveError> {{"
    )
    .map_err(fmt_err)?;

    emit_step_body(out, node)?;

    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::Jump { .. } => emit_linear_step_body(out, node),
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {
            emit_branch_step_body(out, &node.kind)
        }
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. } => emit_boundary_step_body(out, node),
        unsupported => emit_unsupported_node_step(out, unsupported),
    }
}

#[allow(unreachable_pub)]
pub fn emit_unsupported_expr(out: &mut String, op: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    return Err(DriveError::UnsupportedExpressionOp {{ op: \"{op}\" }});"
    )
    .map_err(fmt_err)
}
