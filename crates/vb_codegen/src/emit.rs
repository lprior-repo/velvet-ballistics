//! Code emission logic for generating Rust source.

use std::fmt::Write;

use vb_core::{ActionId, CompiledWorkflow, SlotIdx, StepIdx};

use crate::CodegenError;
use crate::CodegenResult;

mod emit_exprs;
mod emit_meta;
mod emit_steps;
mod emit_step_bodies;

pub use emit_exprs::emit_expr_function;
pub use emit_meta::{
    emit_resource_contract, emit_trybuild_fixture, format_generated_rust,
    compile_check_generated_rust,
};
pub(crate) use emit_meta::emit_constants;
pub use emit_steps::emit_step_function;
pub(crate) use emit_steps::emit_unsupported_expr;

/// Format error conversion for writeln! chains.
pub(crate) fn fmt_err(_: std::fmt::Error) -> CodegenError {
    CodegenError::FormatBufferOverflow
}

/// Generate typed ID helper constants for the workflow.
pub fn emit_ids(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Typed ID constants ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const WORKFLOW_SLOT_COUNT: usize = {};",
        workflow.slot_count()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const WORKFLOW_NODE_COUNT: u16 = {};",
        workflow.node_count()
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Generate the main step loop that drives the compiled workflow.
pub fn emit_drive_function(
    out: &mut String,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    writeln!(out, "// --- Main drive function ---").map_err(fmt_err)?;
    writeln!(
        out,
        "pub fn drive(mut slots: [Option<SlotValue>; {}]) -> Result<SlotValue, DriveError> {{",
        workflow.slot_count()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let mut pc: u16 = {};", workflow.entry().get()).map_err(fmt_err)?;
    writeln!(out, "    loop {{").map_err(fmt_err)?;
    writeln!(out, "        let outcome = match pc {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        writeln!(
            out,
            "            {step_idx} => step_{step_idx}(&mut slots)?,"
        )
        .map_err(fmt_err)?;
    }
    writeln!(
        out,
        "            _ => return Err(DriveError::InvalidProgramCounter),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        }};").map_err(fmt_err)?;
    writeln!(out, "        match outcome {{").map_err(fmt_err)?;
    writeln!(out, "            StepOutcome::Continue(next) => pc = next,").map_err(fmt_err)?;
    writeln!(
        out,
        "            StepOutcome::Finished(value) => return Ok(value),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        }}").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Generate action dispatch boundaries for external action nodes.
pub fn emit_action_boundary(out: &mut String, action: ActionId, input: SlotIdx) -> CodegenResult<()> {
    writeln!(out, "    let _action_input = read_slot(slots, {});", input.get())
        .map_err(fmt_err)?;
    writeln!(
        out,
        "    Err(DriveError::ActionSuspend {{ action_id: {}, input_slot: {} }})",
        action.get(),
        input.get()
    )
    .map_err(fmt_err)?;
    Ok(())
}

/// Generate result extraction code for the workflow.
pub fn emit_finish(out: &mut String, _workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Result extraction ---").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Generate the match-on-ActionId dispatch for all action nodes in the workflow.
pub fn emit_action_match_dispatch(
    out: &mut String,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    writeln!(out, "// --- Action match dispatch ---").map_err(fmt_err)?;
    writeln!(
        out,
        "pub fn dispatch_action(action_id: u16) -> Result<(), DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match action_id {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let vb_core::CompiledNodeKind::Do { action, .. } = &node.kind
        {
            writeln!(out, "        {} => Ok(()),", action.get()).map_err(fmt_err)?;
        }
    }
    writeln!(out, "        _ => Err(DriveError::UnknownAction),").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Emit code to evaluate an accessor by reading the root slot.
pub(crate) fn emit_accessor_eval(
    out: &mut String,
    accessor_idx: vb_core::AccessorIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let Some(accessor) = workflow.accessor(accessor_idx) else {
        writeln!(
            out,
            "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor index out of bounds\" }});"
        )
        .map_err(fmt_err)?;
        return Ok(());
    };

    let root_slot = accessor.root.get();
    if accessor.path.is_empty() {
        writeln!(out, "    stack.push(read_slot(slots, {root_slot})?)?;").map_err(fmt_err)?;
    } else {
        let Some(first_segment) = accessor.path.first() else {
            writeln!(
                out,
                "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor path segment missing\" }});"
            )
            .map_err(fmt_err)?;
            return Ok(());
        };
        let segment_name = match first_segment {
            vb_core::PathSegment::Field(_) => "field",
            vb_core::PathSegment::Index(_) => "index",
        };
        writeln!(
            out,
            "    {{ let _root = read_slot(slots, {root_slot})?; return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor traversal '{segment_name}' on generated type\" }}); }}"
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}
