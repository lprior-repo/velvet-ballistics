//! Step emission functions for code generation.

use std::fmt::Write;
use crate::{CodegenResult, fmt_err};
use crate::emit::action::emit_action_boundary;
use crate::helpers::{emit_unsupported_step, write_next_or_error};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprBranch, ExprOp, SlotBranch, SlotIdx, StepIdx};

pub(crate) fn emit_step_function(
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
        CompiledNodeKind::BuildObject { .. } | CompiledNodeKind::BuildList { .. } => {
            emit_construct_step_body(out, node)
        }
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. } => emit_boundary_step_body(out, node),
        CompiledNodeKind::RetryCheck { .. } => emit_retry_check_step_body(out, &node.kind),
        CompiledNodeKind::CollectStart { .. } => emit_collect_start_step_body(out, &node.kind),
        CompiledNodeKind::CollectPage { .. } => emit_collect_page_step_body(out, &node.kind),
        CompiledNodeKind::CollectNext { .. } => emit_collect_next_step_body(out, &node.kind),
        CompiledNodeKind::CollectFinish { .. } => emit_collect_finish_step_body(out, &node.kind),
        unsupported => emit_unsupported_node_step(out, unsupported),
    }
}

fn emit_linear_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
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
        _ => emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_branch_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => emit_choose_step(out, branches, *otherwise),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => emit_choose_slot_step(out, branches, *otherwise),
        _ => emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_boundary_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::Do { action, input } => emit_action_boundary(out, *action, *input),
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            emit_wait_until_step(out, *deadline_slot, node.next)
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => emit_wait_event_step(out, *event, *timeout_slot, node.next),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => emit_ask_step(out, *prompt, *timeout_slot, node.next),
        CompiledNodeKind::AskResume { answer } => emit_ask_resume_step(out, *answer, node.next),
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            emit_error_handler_step(out, *body, *handler)
        }
        _ => emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_nop_step(out: &mut String, next: Option<StepIdx>) -> CodegenResult<()> {
    match next {
        Some(next_step) => emit_continue_step(out, next_step),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

fn emit_set_const_step(
    out: &mut String,
    output: Option<SlotIdx>,
    value: ConstIdx,
    next: Option<StepIdx>,
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
    write_next_or_error(out, next)
}

fn emit_copy_step(
    out: &mut String,
    output: Option<SlotIdx>,
    source: SlotIdx,
    next: Option<StepIdx>,
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
    write_next_or_error(out, next)
}

fn emit_eval_expr_step(
    out: &mut String,
    output: Option<SlotIdx>,
    expr: vb_core::ExprIdx,
    next: Option<StepIdx>,
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
    write_next_or_error(out, next)
}

fn emit_finish_step(out: &mut String, result: SlotIdx) -> CodegenResult<()> {
    writeln!(out, "    let value = read_slot(slots, {})?;", result.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Finished(value))").map_err(fmt_err)
}

fn emit_continue_step(out: &mut String, target: StepIdx) -> CodegenResult<()> {
    writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)
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

fn emit_choice_fallback(out: &mut String, otherwise: Option<StepIdx>) -> CodegenResult<()> {
    match otherwise {
        Some(fallback) => emit_continue_step(out, fallback),
        None => writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err),
    }
}

fn emit_wait_until_step(
    out: &mut String,
    deadline_slot: SlotIdx,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _deadline = read_slot(slots, {})?;",
        deadline_slot.get()
    )
    .map_err(fmt_err)?;
    match next {
        Some(target) => emit_continue_step(out, target),
        None => writeln!(
            out,
            "    Err(DriveError::WaitSuspend {{ deadline_slot: {} }})",
            deadline_slot.get()
        )
        .map_err(fmt_err),
    }
}

fn emit_wait_event_step(
    out: &mut String,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(out, "    let _event = read_slot(slots, {})?;", event.get()).map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    match next {
        Some(target) => emit_continue_step(out, target),
        None => writeln!(
            out,
            "    Err(DriveError::WaitSuspend {{ deadline_slot: {} }})",
            event.get()
        )
        .map_err(fmt_err),
    }
}

fn emit_ask_step(
    out: &mut String,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _prompt = read_slot(slots, {})?;",
        prompt.get()
    )
    .map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    match next {
        Some(target) => emit_continue_step(out, target),
        None => writeln!(
            out,
            "    Err(DriveError::AskSuspend {{ prompt_slot: {} }})",
            prompt.get()
        )
        .map_err(fmt_err),
    }
}

fn emit_optional_timeout_read(
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
) -> CodegenResult<()> {
    writeln!(out, "    let _answer_slot: u16 = {};", answer.get()).map_err(fmt_err)?;
    write_next_or_error(out, next)
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

fn emit_construct_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::BuildObject { fields } => {
            emit_build_object_step(out, fields, node.output, node.next)
        }
        CompiledNodeKind::BuildList { items } => {
            emit_build_list_step(out, items, node.output, node.next)
        }
        _ => emit_unsupported_step(out, "UnsupportedStep"),
    }
}

fn emit_build_object_step(
    out: &mut String,
    fields: &[(vb_core::SymbolId, SlotIdx)],
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(out, "    // BuildObject: {} field(s)", fields.len()).map_err(fmt_err)?;
    for (i, (sym, slot)) in fields.iter().enumerate() {
        writeln!(
            out,
            "    let _f{} = (_sym_{}, read_slot(slots, {})?)",
            i,
            sym.get(),
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if let Some(output_slot) = output {
        let handle = u32::try_from(fields.len().saturating_add(1)).unwrap_or(u32::MAX);
        writeln!(
            out,
            "    write_slot(slots, {}, Some(SlotValue::Object({})))?;",
            output_slot.get(),
            handle
        )
        .map_err(fmt_err)?;
    }
    write_next_or_error(out, next)
}

fn emit_build_list_step(
    out: &mut String,
    items: &[SlotIdx],
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(out, "    // BuildList: {} item(s)", items.len()).map_err(fmt_err)?;
    for (i, slot) in items.iter().enumerate() {
        writeln!(
            out,
            "    let _item{} = read_slot(slots, {})?;",
            i,
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if let Some(output_slot) = output {
        let handle = u32::try_from(items.len().saturating_add(1)).unwrap_or(u32::MAX);
        writeln!(
            out,
            "    write_slot(slots, {}, Some(SlotValue::List({})))?;",
            output_slot.get(),
            handle
        )
        .map_err(fmt_err)?;
    }
    write_next_or_error(out, next)
}

fn emit_retry_check_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let CompiledNodeKind::RetryCheck {
        policy_slot,
        body,
        exhausted,
    } = kind
    else {
        return emit_unsupported_step(out, "RetryCheck");
    };
    emit_retry_check_step(out, *policy_slot, *body, *exhausted)
}

fn emit_retry_check_step(
    out: &mut String,
    policy_slot: SlotIdx,
    body: StepIdx,
    exhausted: StepIdx,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _policy = read_slot(slots, {})?;",
        policy_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _retry_count = match _policy {{ SlotValue::I64(n) => n, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }};"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _limit = i64::from(CONTRACT_MAX_RETRY_ATTEMPTS);"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    if _retry_count < _limit {{ Ok(StepOutcome::Continue({})) }} else {{ Ok(StepOutcome::Continue({})) }}",
        body.get(),
        exhausted.get()
    )
    .map_err(fmt_err)
}

fn emit_collect_start_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let CompiledNodeKind::CollectStart { source, limit, page_size, body, done } = kind else {
        return emit_unsupported_step(out, "CollectStart");
    };
    emit_collect_start_step(out, *source, *limit, *page_size, *body, *done)
}

fn emit_collect_start_step(out: &mut String, source: SlotIdx, limit: u32, page_size: u32, body: StepIdx, done: StepIdx) -> CodegenResult<()> {
    writeln!(out, "    // CollectStart: source={}, limit={}, page_size={}, body={}, done={}", source.get(), limit, page_size, body.get(), done.get()).map_err(fmt_err)?;
    writeln!(out, "    let _source = read_slot(slots, {})?;", source.get()).map_err(fmt_err)?;
    writeln!(out, "    let _limit = {limit}u32;").map_err(fmt_err)?;
    writeln!(out, "    let _page_size = {page_size}u32;").map_err(fmt_err)?;
    writeln!(out, "    let _collected = 0u32;").map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Continue({}))", body.get()).map_err(fmt_err)
}

fn emit_collect_page_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let CompiledNodeKind::CollectPage { collector_slot, body, done } = kind else {
        return emit_unsupported_step(out, "CollectPage");
    };
    emit_collect_page_step(out, *collector_slot, *body, *done)
}

fn emit_collect_page_step(out: &mut String, collector_slot: SlotIdx, body: StepIdx, done: StepIdx) -> CodegenResult<()> {
    writeln!(out, "    // CollectPage: collector_slot={}, body={}, done={}", collector_slot.get(), body.get(), done.get()).map_err(fmt_err)?;
    writeln!(out, "    let _collector = read_slot(slots, {})?;", collector_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Continue({}))", body.get()).map_err(fmt_err)
}

fn emit_collect_next_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let CompiledNodeKind::CollectNext { collector_slot, body, done } = kind else {
        return emit_unsupported_step(out, "CollectNext");
    };
    emit_collect_next_step(out, *collector_slot, *body, *done)
}

fn emit_collect_next_step(out: &mut String, collector_slot: SlotIdx, body: StepIdx, done: StepIdx) -> CodegenResult<()> {
    writeln!(out, "    // CollectNext: collector_slot={}, body={}, done={}", collector_slot.get(), body.get(), done.get()).map_err(fmt_err)?;
    writeln!(out, "    let _collector = read_slot(slots, {})?;", collector_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Continue({}))", body.get()).map_err(fmt_err)
}

fn emit_collect_finish_step_body(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let CompiledNodeKind::CollectFinish { collector_slot } = kind else {
        return emit_unsupported_step(out, "CollectFinish");
    };
    emit_collect_finish_step(out, *collector_slot)
}

fn emit_collect_finish_step(out: &mut String, collector_slot: SlotIdx) -> CodegenResult<()> {
    writeln!(out, "    // CollectFinish: collector_slot={}", collector_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    let _collector = read_slot(slots, {})?;", collector_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Continue({}))", collector_slot.get()).map_err(fmt_err)
}

fn emit_unsupported_node_step(out: &mut String, kind: &CompiledNodeKind) -> CodegenResult<()> {
    let name = match kind {
        CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        CompiledNodeKind::CollectStart { .. } => "CollectStart",
        CompiledNodeKind::CollectPage { .. } => "CollectPage",
        CompiledNodeKind::CollectNext { .. } => "CollectNext",
        CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        _ => "UnsupportedStep",
    };
    emit_unsupported_step(out, name)
}

// Generate an expression evaluator function.

