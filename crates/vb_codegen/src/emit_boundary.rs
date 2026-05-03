//! Boundary step emission functions (action, wait, ask, error handler).

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::{ActionId, CompiledNode, SlotIdx, StepIdx};

pub fn emit_boundary_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        vb_core::CompiledNodeKind::Do { action, input } => {
            super::emit_workflow::emit_action_boundary(out, *action, *input)
        }
        vb_core::CompiledNodeKind::WaitUntil { deadline_slot } => {
            emit_wait_until_step(out, *deadline_slot, node.next)
        }
        vb_core::CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => emit_wait_event_step(out, *event, *timeout_slot, node.next),
        vb_core::CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => emit_ask_step(out, *prompt, *timeout_slot, node.next),
        vb_core::CompiledNodeKind::AskResume { answer } => {
            emit_ask_resume_step(out, *answer, node.next)
        }
        vb_core::CompiledNodeKind::ErrorHandler { body, handler } => {
            emit_error_handler_step(out, *body, *handler)
        }
        _ => super::emit_unsupported::emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_wait_until_step(
    out: &mut String,
    deadline_slot: SlotIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _deadline = read_slot(slots, {})?;",
        deadline_slot.get()
    )
    .map_err(fmt_err)?;
    super::helpers::write_next_or_error(out, next)
}

fn emit_wait_event_step(
    out: &mut String,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    writeln!(out, "    let _event = read_slot(slots, {})?;", event.get()).map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    super::helpers::write_next_or_error(out, next)
}

fn emit_ask_step(
    out: &mut String,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _prompt = read_slot(slots, {})?;",
        prompt.get()
    )
    .map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    super::helpers::write_next_or_error(out, next)
}

pub fn emit_optional_timeout_read(
    out: &mut String,
    timeout_slot: Option<SlotIdx>,
) -> CodegenResult<()> {
    if let Some(timeout) = timeout_slot {
        writeln!(
            out,
            "    let _timeout = read_slot(slots, {})?;",
            timeout.get()
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}

fn emit_ask_resume_step(
    out: &mut String,
    answer: SlotIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CodegenResult<()> {
    writeln!(out, "    let _answer_slot: u16 = {};", answer.get()).map_err(fmt_err)?;
    super::helpers::write_next_or_error(out, next)
}

fn emit_error_handler_step(out: &mut String, body: StepIdx, handler: StepIdx) -> CodegenResult<()> {
    writeln!(
        out,
        "    // ErrorHandler: body={}, handler={}",
        body.get(),
        handler.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match step_{}(slots) {{", body.get()).map_err(fmt_err)?;
    writeln!(out, "        Ok(outcome) => Ok(outcome),").map_err(fmt_err)?;
    writeln!(
        out,
        "        Err(_) => Ok(StepOutcome::Continue({})),",
        handler.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)
}
