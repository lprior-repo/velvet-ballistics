//! Main step emission dispatcher.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

/// Generate a per-step function for one compiled node.
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
        | CompiledNodeKind::Jump { .. } => crate::emit_linear::emit_linear_step_body(out, node),
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {
            crate::emit_branch::emit_branch_step_body(out, &node.kind)
        }
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. } => {
            crate::emit_boundary::emit_boundary_step_body(out, node)
        }
        unsupported => crate::emit_unsupported::emit_unsupported_node_step(out, unsupported),
    }
}
