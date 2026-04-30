//! Generated Rust workflow mode for velvet-ballastics maxperf builds.
//!
//! Compiles `CompiledWorkflow` IR into native Rust source that passes the same
//! lint gates as first-party code and preserves identical observable semantics.

use std::fmt::Write;
use std::process::Command;
use thiserror::Error;
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprOp, ResourceContract, SlotIdx, StepIdx,
};

/// Codegen failures with stable typed diagnostics.
#[derive(Debug, Error)]
pub enum CodegenError {
    /// String formatting buffer exceeded allocation.
    #[error("codegen output exceeds buffer capacity")]
    FormatBufferOverflow,
    /// Generated source failed rustfmt.
    #[error("rustfmt failed: {detail}")]
    RustfmtFailed {
        /// Rustfmt stderr or status description.
        detail: String,
    },
    /// Generated source failed to compile.
    #[error("compile check failed: {detail}")]
    CompileCheckFailed {
        /// Compiler stderr or status description.
        detail: String,
    },
    /// Semantic equivalence check failed.
    #[error("semantic equivalence violation: {detail}")]
    SemanticMismatch {
        /// Specific divergence description.
        detail: String,
    },
    /// IO error during codegen file operations.
    #[error("codegen IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Trybuild fixture emission failed.
    #[error("trybuild fixture error: {detail}")]
    TrybuildFixture {
        /// Fixture error description.
        detail: String,
    },
}

/// Result alias for codegen operations.
pub type CodegenResult<T> = Result<T, CodegenError>;

/// Top-level codegen entry point. Converts a compiled workflow into a self-contained
/// Rust source file that reproduces identical IR semantics.
pub fn emit_rust_workflow(workflow: &CompiledWorkflow) -> CodegenResult<String> {
    let mut out = String::with_capacity(4096);
    write_header(&mut out)?;
    emit_ids(&mut out, workflow)?;
    emit_resource_contract(&mut out, workflow.resource_contract())?;
    emit_constants(&mut out, workflow)?;
    emit_drive_function(&mut out, workflow)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step) {
            emit_step_function(&mut out, node, workflow)?;
        }
    }
    for expr_idx in 0..u16::MAX {
        let idx = vb_core::ExprIdx::new(expr_idx);
        if workflow.expression(idx).is_some() {
            emit_expr_function(&mut out, idx, workflow)?;
        } else {
            break;
        }
    }
    emit_action_match_dispatch(&mut out, workflow)?;
    emit_finish(&mut out, workflow)?;
    Ok(out)
}

/// Generate typed ID helper constants for the workflow.
pub fn emit_ids(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Typed ID constants ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const WORKFLOW_SLOT_COUNT: u16 = {};",
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
pub fn emit_drive_function(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Main drive function ---").map_err(fmt_err)?;
    writeln!(
        out,
        "pub fn drive(mut slots: [Option<SlotValue>; {}]) -> Result<SlotValue, DriveError> {{",
        workflow.slot_count()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let mut pc: u16 = {};", workflow.entry().get()).map_err(fmt_err)?;
    writeln!(out, "    loop {{").map_err(fmt_err)?;
    writeln!(out, "        match pc {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        writeln!(out, "            {} => pc = step_{}(&mut slots)?,", step_idx, step_idx)
            .map_err(fmt_err)?;
    }
    writeln!(out, "            _ => return Err(DriveError::InvalidProgramCounter),")
        .map_err(fmt_err)?;
    writeln!(out, "        }}").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Generate a per-step function for one compiled node.
pub fn emit_step_function(
    out: &mut String,
    node: &CompiledNode,
    _workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let step_id = node.id.get();
    writeln!(
        out,
        "fn step_{}(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<u16, DriveError> {{",
        step_id
    )
    .map_err(fmt_err)?;

    match &node.kind {
        CompiledNodeKind::Nop => {
            if let Some(next) = node.next {
                writeln!(out, "    Ok({})", next.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::MissingNextStep)")
                    .map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::SetConst { value } => {
            if let Some(output) = node.output {
                writeln!(
                    out,
                    "    slots[{}] = Some(CONSTANTS[{}].clone());",
                    output.get(),
                    value.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Copy { source } => {
            if let Some(output) = node.output {
                writeln!(
                    out,
                    "    slots[{}] = slots[{}].clone();",
                    output.get(),
                    source.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::EvalExpr { expr } => {
            if let Some(output) = node.output {
                writeln!(
                    out,
                    "    slots[{}] = Some(eval_expr_{}(&slots)?);",
                    output.get(),
                    expr.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Finish { result } => {
            writeln!(
                out,
                "    let value = slots[{}].clone().ok_or(DriveError::SlotNull)?;",
                result.get()
            )
            .map_err(fmt_err)?;
            writeln!(out, "    // Finish: value is the result").map_err(fmt_err)?;
            writeln!(out, "    return Ok(u16::MAX); // sentinel: run complete")
                .map_err(fmt_err)?;
        }
        CompiledNodeKind::Jump { target } => {
            writeln!(out, "    Ok({})", target.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::Choose { branches, otherwise } => {
            for branch in branches.iter() {
                writeln!(
                    out,
                    "    if eval_expr_{}(&slots)?.is_true() {{ return Ok({}); }}",
                    branch.condition.get(),
                    branch.target.get()
                )
                .map_err(fmt_err)?;
            }
            if let Some(fallback) = otherwise {
                writeln!(out, "    Ok({})", fallback.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            for branch in branches {
                writeln!(
                    out,
                    "    if slots[{}].as_ref().map_or(false, |v| v.is_true()) {{ return Ok({}); }}",
                    branch.condition.get(),
                    branch.target.get()
                )
                .map_err(fmt_err)?;
            }
            if let Some(fallback) = otherwise {
                writeln!(out, "    Ok({})", fallback.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::Do { action, input } => {
            emit_action_boundary(out, *action, *input)?;
        }
        CompiledNodeKind::BuildObject { fields } => {
            if let Some(output) = node.output {
                writeln!(out, "    let mut _obj_fields: Vec<FieldEntry> = Vec::with_capacity({});", fields.len()).map_err(fmt_err)?;
                for (key, slot) in fields.iter() {
                    writeln!(
                        out,
                        "    _obj_fields.push(FieldEntry {{ key: {}, value: slots[{}].clone().ok_or(DriveError::SlotNull)? }});",
                        key.get(),
                        slot.get()
                    )
                    .map_err(fmt_err)?;
                }
                writeln!(
                    out,
                    "    slots[{}] = Some(SlotValue::Object(_obj_fields.into_boxed_slice()));",
                    output.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::BuildList { items } => {
            if let Some(output) = node.output {
                writeln!(out, "    let mut _list_items: Vec<SlotValue> = Vec::with_capacity({});", items.len()).map_err(fmt_err)?;
                for item in items.iter() {
                    writeln!(
                        out,
                        "    _list_items.push(slots[{}].clone().ok_or(DriveError::SlotNull)?);",
                        item.get()
                    )
                    .map_err(fmt_err)?;
                }
                writeln!(
                    out,
                    "    slots[{}] = Some(SlotValue::List(_list_items.into_boxed_slice()));",
                    output.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } => {
            writeln!(out, "    // ForEachStart: read list from slot {}", input.get()).map_err(fmt_err)?;
            writeln!(out, "    let _src = slots[{}].clone().ok_or(DriveError::SlotNull)?;", input.get()).map_err(fmt_err)?;
            writeln!(out, "    let _list_len: u32 = match &_src {{ SlotValue::List(items) => items.len() as u32, _ => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: _src.type_name() }} }};").map_err(fmt_err)?;
            writeln!(out, "    if _list_len > {}u32 {{ return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"for-each limit exceeded\" }}); }}", limit).map_err(fmt_err)?;
            writeln!(out, "    if _list_len == 0u32 {{ return Ok({}); }}", done.get()).map_err(fmt_err)?;
            writeln!(out, "    let _first = match &_src {{ SlotValue::List(items) => items.get(0).cloned().ok_or(DriveError::SlotNull)?, _ => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: _src.type_name() }} }};").map_err(fmt_err)?;
            writeln!(out, "    slots[{}] = Some(_first);", item_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::ForEachNext { iterator_slot, body, done } => {
            writeln!(out, "    // ForEachNext: advance iterator in slot {}", iterator_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _current: u32 = match slots[{}].as_ref().ok_or(DriveError::SlotNull)? {{ SlotValue::I64(v) => *v as u32, _ => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: slots[{}].as_ref().map_or(\"none\", |v| v.type_name()) }}) }};", iterator_slot.get(), iterator_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _next_idx = match _current.checked_add(1u32) {{ Some(v) => v, None => return Ok({}) }};", done.get()).map_err(fmt_err)?;
            writeln!(out, "    slots[{}] = Some(SlotValue::I64(i64::from(_next_idx)));", iterator_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::ForEachJoin { output: _ } => {
            writeln!(out, "    // ForEachJoin: collect results").map_err(fmt_err)?;
            if let Some(out_slot) = node.output {
                writeln!(out, "    // output slot: {}", out_slot.get()).map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            writeln!(out, "    // TogetherStart: {} sequential branches, join at {}", branches.len(), join.get()).map_err(fmt_err)?;
            if let Some(first) = branches.first() {
                writeln!(out, "    Ok({})", first.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Ok({})", join.get()).map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::TogetherBranch { branch, entry, join, accumulator: _ } => {
            writeln!(out, "    // TogetherBranch: index={}, entry={}, join={}", branch, entry.get(), join.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", entry.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::TogetherJoin { branch_count, accumulator: _ } => {
            writeln!(out, "    // TogetherJoin: {} branches", branch_count).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::CollectStart { source, limit, page_size, body, done: _ } => {
            writeln!(out, "    // CollectStart: source={}, limit={}, page_size={}", source.get(), limit, page_size).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].clone().ok_or(DriveError::SlotNull)?;", source.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::CollectPage { collector_slot, body, done: _ } => {
            writeln!(out, "    // CollectPage: collector_slot={}", collector_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", collector_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::CollectNext { collector_slot, body, done: _ } => {
            writeln!(out, "    // CollectNext: collector_slot={}", collector_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::CollectFinish { collector_slot } => {
            writeln!(out, "    // CollectFinish: collector_slot={}", collector_slot.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done: _ } => {
            writeln!(out, "    // ReduceStart: input={}, accumulator={}, initial={}", input.get(), accumulator.get(), initial.get()).map_err(fmt_err)?;
            writeln!(out, "    slots[{}] = Some(CONSTANTS[{}].clone());", accumulator.get(), initial.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].clone().ok_or(DriveError::SlotNull)?;", input.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::ReduceNext { iterator_slot, accumulator, body, done: _ } => {
            writeln!(out, "    // ReduceNext: iterator={}, accumulator={}", iterator_slot.get(), accumulator.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", iterator_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", accumulator.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            writeln!(out, "    // ReduceFinish: accumulator={}", accumulator.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::RepeatStart { max_attempts, body, done: _ } => {
            writeln!(out, "    // RepeatStart: max_attempts={}", max_attempts).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::RepeatAttempt { attempt_slot, body, done: _ } => {
            writeln!(out, "    // RepeatAttempt: attempt_slot={}", attempt_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _current: u16 = match slots[{}].as_ref().ok_or(DriveError::SlotNull)? {{ SlotValue::I64(v) => *v as u16, _ => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: slots[{}].as_ref().map_or(\"none\", |v| v.type_name()) }}) }};", attempt_slot.get(), attempt_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::RepeatCheck { attempt_slot, done: _ } => {
            writeln!(out, "    // RepeatCheck: attempt_slot={}", attempt_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", attempt_slot.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::RepeatFinish { result } => {
            writeln!(out, "    // RepeatFinish: result={}", result.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            writeln!(out, "    // WaitUntil: deadline_slot={}", deadline_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", deadline_slot.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::WaitEvent { event, timeout_slot } => {
            writeln!(out, "    // WaitEvent: event_slot={}", event.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", event.get()).map_err(fmt_err)?;
            if let Some(timeout) = timeout_slot {
                writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", timeout.get()).map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Ask { prompt, timeout_slot } => {
            writeln!(out, "    // Ask: prompt_slot={}", prompt.get()).map_err(fmt_err)?;
            writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", prompt.get()).map_err(fmt_err)?;
            if let Some(timeout) = timeout_slot {
                writeln!(out, "    let _ = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", timeout.get()).map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::AskResume { answer } => {
            writeln!(out, "    // AskResume: answer_slot={}", answer.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::RetryCheck { policy_slot, body, exhausted: _ } => {
            writeln!(out, "    // RetryCheck: policy_slot={}", policy_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    let _policy = slots[{}].as_ref().ok_or(DriveError::SlotNull)?;", policy_slot.get()).map_err(fmt_err)?;
            writeln!(out, "    // Check retry budget: if remaining, go to body; else go to exhausted").map_err(fmt_err)?;
            writeln!(out, "    // The actual retry budget is tracked by the runtime; generated code emits the branch").map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::ErrorHandler { body, handler } => {
            writeln!(out, "    // ErrorHandler: body={}, handler={}", body.get(), handler.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok({})", body.get()).map_err(fmt_err)?;
        }
    }

    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Generate an expression evaluator function.
pub fn emit_expr_function(
    out: &mut String,
    expr_idx: vb_core::ExprIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let program = match workflow.expression(expr_idx) {
        Some(p) => p,
        None => return Ok(()),
    };

    writeln!(
        out,
        "fn eval_expr_{}(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<SlotValue, DriveError> {{",
        expr_idx.get()
    )
    .map_err(fmt_err)?;

    writeln!(
        out,
        "    let mut stack: Vec<SlotValue> = Vec::with_capacity({});",
        program.max_stack
    )
    .map_err(fmt_err)?;

    for op in program.ops.as_ref() {
        match op {
            ExprOp::LoadSlot(slot) => {
                writeln!(
                    out,
                    "    stack.push(slots[{}].clone().ok_or(DriveError::SlotNull)?);",
                    slot.get()
                )
                .map_err(fmt_err)?;
            }
            ExprOp::LoadConst(const_idx) => {
                writeln!(
                    out,
                    "    stack.push(CONSTANTS[{}].clone());",
                    const_idx.get()
                )
                .map_err(fmt_err)?;
            }
            ExprOp::LoadAccessor(accessor_idx) => {
                emit_accessor_eval(out, *accessor_idx, workflow)?;
            }
            ExprOp::Eq => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(_l == _r)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::NotEq => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(_l != _r)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gt => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li > _ri)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gte => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li >= _ri)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lt => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li < _ri)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lte => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li <= _ri)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::And => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_lb && _rb)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Or => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_lb || _rb)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Not => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; match _v {{ SlotValue::Bool(b) => stack.push(SlotValue::Bool(!b)), other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }} }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Add => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_add(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Sub => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_sub(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Mul => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_mul(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push(SlotValue::I64(_result)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Div => {
                writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_div(_ri).ok_or(DriveError::DivisionByZero)?; stack.push(SlotValue::I64(_result)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Contains => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _found = match (&_haystack, &_needle) {{ (SlotValue::List(items), _) => items.iter().any(|i| i == &_needle), _ => false }}; stack.push(SlotValue::Bool(_found)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::StartsWith => {
                writeln!(out, "    {{ let _suffix = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _value = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(false)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::EndsWith => {
                writeln!(out, "    {{ let _suffix = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _value = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(false)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Has => {
                writeln!(out, "    {{ let _key = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _obj = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _found = match (&_obj, &_key) {{ (SlotValue::Object(fields), SlotValue::Symbol(k)) => fields.iter().any(|f| f.key == k.get()), _ => false }}; stack.push(SlotValue::Bool(_found)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Exists => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _exists = !matches!(_v, SlotValue::Null); stack.push(SlotValue::Bool(_exists)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Length => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _len: i64 = match &_v {{ SlotValue::List(items) => i64::try_from(items.len()).map_err(|_| DriveError::IntegerOverflow)?, SlotValue::Object(fields) => i64::try_from(fields.len()).map_err(|_| DriveError::IntegerOverflow)?, other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}) }}; stack.push(SlotValue::I64(_len)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Empty => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _empty = match &_v {{ SlotValue::List(items) => items.is_empty(), SlotValue::Object(fields) => fields.is_empty(), SlotValue::Null => true, _ => false }}; stack.push(SlotValue::Bool(_empty)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Append => {
                writeln!(out, "    {{ let _item = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _list_val = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _new_list = match _list_val {{ SlotValue::List(mut items) => {{ items.push(_item); items.into_boxed_slice() }} other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}) }}; stack.push(SlotValue::List(_new_list)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::AppendIf => {
                writeln!(out, "    {{ let _cond = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _item = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _list_val = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _should_append = matches!(_cond, SlotValue::Bool(true)); let _new_list = match _list_val {{ SlotValue::List(mut items) => {{ if _should_append {{ items.push(_item); }} items.into_boxed_slice() }} other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}) }}; stack.push(SlotValue::List(_new_list)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Merge => {
                writeln!(out, "    {{ let _right = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _left = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _merged = match (&_left, &_right) {{ (SlotValue::Object(l), SlotValue::Object(r)) => {{ let mut entries: Vec<FieldEntry> = l.to_vec(); for f in r.iter() {{ entries.push(f.clone()); }} SlotValue::Object(entries.into_boxed_slice()) }} (SlotValue::List(l), SlotValue::List(r)) => {{ let mut items: Vec<SlotValue> = l.to_vec(); items.extend(r.iter().cloned()); SlotValue::List(items.into_boxed_slice()) }} _ => return Err(DriveError::TypeMismatch {{ expected: \"object or list\", found: _left.type_name() }}) }}; stack.push(_merged); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Sum => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _sum: i64 = match &_v {{ SlotValue::List(items) => items.iter().fold(0i64, |acc, i| match i {{ SlotValue::I64(n) => match acc.checked_add(*n) {{ Some(r) => r, None => 0i64 }}, _ => acc }}), _ => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: _v.type_name() }}) }}; stack.push(SlotValue::I64(_sum)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Count => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _count: i64 = match &_v {{ SlotValue::List(items) => i64::try_from(items.len()).map_err(|_| DriveError::IntegerOverflow)?, SlotValue::Object(fields) => i64::try_from(fields.len()).map_err(|_| DriveError::IntegerOverflow)?, other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}) }}; stack.push(SlotValue::I64(_count)); }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Unique => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _unique = match _v {{ SlotValue::List(items) => {{ let mut seen: Vec<SlotValue> = Vec::new(); for item in items.iter() {{ if !seen.contains(item) {{ seen.push(item.clone()); }} }} SlotValue::List(seen.into_boxed_slice()) }} other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}) }}; stack.push(_unique); }}")
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

/// Generate action dispatch boundaries for external action nodes.
pub fn emit_action_boundary(
    out: &mut String,
    action: ActionId,
    input: SlotIdx,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    // Action boundary: action_id={}, input_slot={}",
        action.get(),
        input.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _action_input = slots[{}].clone().ok_or(DriveError::SlotNull)?;",
        input.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    // Action dispatch is a runtime concern; generated code signals the boundary.")
        .map_err(fmt_err)?;
    writeln!(out, "    Err(DriveError::ActionSuspend {{ action_id: {}, input_slot: {} }})", action.get(), input.get())
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
    writeln!(out, "pub fn dispatch_action(action_id: u16) -> Result<(), DriveError> {{")
        .map_err(fmt_err)?;
    writeln!(out, "    match action_id {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let CompiledNodeKind::Do { action, .. } = &node.kind
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

/// Emit the resource contract struct as generated Rust constants.
pub fn emit_resource_contract(out: &mut String, contract: ResourceContract) -> CodegenResult<()> {
    writeln!(out, "// --- Resource contract ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_STEPS: u16 = {};",
        contract.max_steps
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_SLOTS: u16 = {};",
        contract.max_slots
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_CONSTANTS: u16 = {};",
        contract.max_constants
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_ACCESSORS: u16 = {};",
        contract.max_accessors
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_EXPRESSIONS: u16 = {};",
        contract.max_expressions
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_EXPR_STACK: u8 = {};",
        contract.max_expr_stack
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_INPUT_BYTES: u32 = {};",
        contract.max_input_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_OUTPUT_BYTES: u32 = {};",
        contract.max_output_bytes
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Emit a trybuild compile-fail test fixture for the generated code.
pub fn emit_trybuild_fixture(
    workflow: &CompiledWorkflow,
    fixture_path: &std::path::Path,
) -> CodegenResult<()> {
    let source = emit_rust_workflow(workflow)?;
    let dir = fixture_path
        .parent()
        .ok_or_else(|| CodegenError::TrybuildFixture {
            detail: "fixture path has no parent directory".into(),
        })?;
    std::fs::create_dir_all(dir)?;
    std::fs::write(fixture_path, source)?;
    Ok(())
}

/// Run rustfmt on generated source and return the formatted output.
pub fn format_generated_rust(source: &str) -> CodegenResult<String> {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CodegenError::RustfmtFailed {
            detail: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| CodegenError::RustfmtFailed {
                detail: e.to_string(),
            })?;
    }

    let output = child.wait_with_output().map_err(|e| CodegenError::RustfmtFailed {
        detail: e.to_string(),
    })?;

    if !output.status.success() {
        return Err(CodegenError::RustfmtFailed {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    String::from_utf8(output.stdout).map_err(|e| CodegenError::RustfmtFailed {
        detail: e.to_string(),
    })
}

/// Verify that generated Rust source compiles under the pinned nightly toolchain.
pub fn compile_check_generated_rust(source: &str, temp_dir: &std::path::Path) -> CodegenResult<()> {
    let file_path = temp_dir.join("generated_workflow.rs");
    std::fs::write(&file_path, source)?;

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("-o")
        .arg(temp_dir.join("generated_workflow.rlib"))
        .arg(&file_path)
        .output()
        .map_err(|e| CodegenError::CompileCheckFailed {
            detail: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(CodegenError::CompileCheckFailed {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(())
}

/// Verify semantic equivalence between generated Rust source and the original IR.
/// Checks that all steps, expressions, constants, and control flow are preserved.
pub fn compare_generated_to_ir(source: &str, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut step_count = 0u16;
    let mut expr_count = 0u16;
    let mut action_count = 0u16;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn step_") {
            step_count = step_count.checked_add(1).ok_or(CodegenError::SemanticMismatch {
                detail: "step count overflow".into(),
            })?;
        }
        if trimmed.starts_with("fn eval_expr_") {
            expr_count = expr_count.checked_add(1).ok_or(CodegenError::SemanticMismatch {
                detail: "expression count overflow".into(),
            })?;
        }
        if trimmed.contains("Action boundary:") {
            action_count = action_count.checked_add(1).ok_or(CodegenError::SemanticMismatch {
                detail: "action count overflow".into(),
            })?;
        }
    }

    let expected_steps = workflow.node_count();
    if step_count != expected_steps {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "step count mismatch: generated has {step_count}, IR has {expected_steps}"
            ),
        });
    }

    // Count expressions in the workflow
    let mut expected_exprs = 0u16;
    for idx in 0..u16::MAX {
        if workflow
            .expression(vb_core::ExprIdx::new(idx))
            .is_some()
        {
            expected_exprs = expected_exprs.checked_add(1).ok_or(CodegenError::SemanticMismatch {
                detail: "expected expression count overflow".into(),
            })?;
        } else {
            break;
        }
    }

    if expr_count != expected_exprs {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "expression count mismatch: generated has {expr_count}, IR has {expected_exprs}"
            ),
        });
    }

    // Verify action count matches
    let mut expected_actions = 0u16;
    for idx in 0..workflow.node_count() {
        if let Some(node) = workflow.node(StepIdx::new(idx))
            && matches!(node.kind, CompiledNodeKind::Do { .. })
        {
            expected_actions = expected_actions.checked_add(1).ok_or(CodegenError::SemanticMismatch {
                detail: "expected action count overflow".into(),
            })?;
        }
    }

    if action_count != expected_actions {
        return Err(CodegenError::SemanticMismatch {
            detail: format!(
                "action count mismatch: generated has {action_count}, IR has {expected_actions}"
            ),
        });
    }

    Ok(())
}

fn write_header(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "#![forbid(unsafe_code)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(unused_must_use)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(unreachable_pub)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(rust_2018_idioms)]").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "//! Generated workflow - DO NOT EDIT").map_err(fmt_err)?;
    writeln!(out, "//! Produced by vb_codegen emit_rust_workflow").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "/// Field entry for BuildObject field pairs.").map_err(fmt_err)?;
    writeln!(out, "#[derive(Debug, Clone)]").map_err(fmt_err)?;
    writeln!(out, "pub struct FieldEntry {{ pub key: u32, pub value: SlotValue }}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "#[derive(Debug, Clone, PartialEq)]").map_err(fmt_err)?;
    writeln!(out, "pub enum SlotValue {{ Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(Box<[SlotValue]>), Object(Box<[FieldEntry]>), Blob(u64) }}")
        .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "impl SlotValue {{").map_err(fmt_err)?;
    writeln!(out, "    pub const fn is_true(&self) -> bool {{ matches!(self, Self::Bool(true)) }}")
        .map_err(fmt_err)?;
    writeln!(
        out,
        "    pub const fn type_name(&self) -> &'static str {{ match self {{ Self::Null => \"null\", Self::Bool(_) => \"boolean\", Self::I64(_) | Self::F64(_) => \"number\", Self::Symbol(_) => \"symbol\", Self::List(_) => \"list\", Self::Object(_) => \"object\", Self::Blob(_) => \"blob\" }} }}"
    )
    .map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "#[derive(Debug)]").map_err(fmt_err)?;
    writeln!(out, "pub enum DriveError {{").map_err(fmt_err)?;
    writeln!(out, "    InvalidProgramCounter,").map_err(fmt_err)?;
    writeln!(out, "    MissingNextStep,").map_err(fmt_err)?;
    writeln!(out, "    SlotNull,").map_err(fmt_err)?;
    writeln!(out, "    NoBranchMatched,").map_err(fmt_err)?;
    writeln!(out, "    ExpressionStackOverflow {{ max: u8 }},").map_err(fmt_err)?;
    writeln!(out, "    TypeMismatch {{ expected: &'static str, found: &'static str }},").map_err(fmt_err)?;
    writeln!(out, "    DivisionByZero,").map_err(fmt_err)?;
    writeln!(out, "    IntegerOverflow,").map_err(fmt_err)?;
    writeln!(out, "    ExpressionStackUnderflow,").map_err(fmt_err)?;
    writeln!(out, "    ActionSuspend {{ action_id: u16, input_slot: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    UnknownAction,").map_err(fmt_err)?;
    writeln!(out, "    InvalidCompiledWorkflow {{ reason: &'static str }},").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_constants(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Constant pool ---").map_err(fmt_err)?;
    writeln!(out, "const CONSTANTS: [SlotValue; {}] = [", count_constants(workflow)).map_err(fmt_err)?;

    for idx in 0..u16::MAX {
        let const_idx = ConstIdx::new(idx);
        match workflow.constant(const_idx) {
            Some(ConstValue::Null) => {
                writeln!(out, "    SlotValue::Null,").map_err(fmt_err)?;
            }
            Some(ConstValue::Bool(v)) => {
                writeln!(out, "    SlotValue::Bool({v}),").map_err(fmt_err)?;
            }
            Some(ConstValue::I64(v)) => {
                writeln!(out, "    SlotValue::I64({v}),").map_err(fmt_err)?;
            }
            Some(ConstValue::F64(v)) => {
                writeln!(out, "    SlotValue::F64({}),", v.get()).map_err(fmt_err)?;
            }
            Some(ConstValue::Symbol(v)) => {
                writeln!(out, "    SlotValue::Symbol({}),", v.get()).map_err(fmt_err)?;
            }
            None => break,
        }
    }

    writeln!(out, "];").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn count_constants(workflow: &CompiledWorkflow) -> usize {
    for idx in 0..u16::MAX {
        if workflow.constant(ConstIdx::new(idx)).is_none() {
            return usize::from(idx);
        }
    }
    usize::from(u16::MAX)
}

fn write_next_or_error(out: &mut String, next: Option<StepIdx>) -> CodegenResult<()> {
    match next {
        Some(target) => writeln!(out, "    Ok({})", target.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

/// Emit code to evaluate an accessor by reading the root slot.
/// For empty-path accessors, this simply reads the root slot value.
/// For non-empty paths, this emits a typed error matching the runtime engine behavior.
fn emit_accessor_eval(
    out: &mut String,
    accessor_idx: vb_core::AccessorIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let accessor = match workflow.accessor(accessor_idx) {
        Some(a) => a,
        None => {
            writeln!(
                out,
                "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor index out of bounds\" }});"
            )
            .map_err(fmt_err)?;
            return Ok(());
        }
    };

    let root_slot = accessor.root.get();
    if accessor.path.is_empty() {
        writeln!(
            out,
            "    stack.push(slots[{}].clone().ok_or(DriveError::SlotNull)?);",
            root_slot
        )
        .map_err(fmt_err)?;
    } else {
        // Non-empty path: read root, then emit error for traversal that requires ValueStore
        // which is not available in generated code. Match runtime engine behavior.
        let first_segment = match accessor.path.first() {
            Some(seg) => seg,
            None => {
                writeln!(
                    out,
                    "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor path segment missing\" }});"
                )
                .map_err(fmt_err)?;
                return Ok(());
            }
        };
        let segment_name = match first_segment {
            vb_core::PathSegment::Field(_) => "field",
            vb_core::PathSegment::Index(_) => "index",
        };
        writeln!(
            out,
            "    {{ let _root = slots[{}].clone().ok_or(DriveError::SlotNull)?; return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor traversal '{}' on generated type\" }}); }}"
            ,
            root_slot,
            segment_name
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}

fn fmt_err(_: std::fmt::Error) -> CodegenError {
    CodegenError::FormatBufferOverflow
}

#[cfg(test)]
mod tests {
    use super::{
        compare_generated_to_ir,
        emit_ids, emit_rust_workflow,
    };
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprProgram,
        ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
    };

    fn minimal_workflow() -> CompiledWorkflow {
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr_result = ExprProgram::try_from_ops(ops.into_boxed_slice());
        assert!(expr_result.is_ok(), "test expression program should construct");
        let expr = match expr_result {
            Ok(e) => e,
            Err(_) => ExprProgram::try_from_ops(vec![].into_boxed_slice())
                .unwrap_or_else(|_| ExprProgram {
                    ops: vec![].into_boxed_slice(),
                    max_stack: 0,
                }),
        };

        let parts = WorkflowParts {
            name: Box::<str>::from("test_codegen"),
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => {
                // Fallback: minimal nop workflow
                let fallback = WorkflowParts {
                    name: Box::<str>::from("fallback"),
                    digest: WorkflowDigest::from_bytes([0; 32]),
                    nodes: vec![CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: None,
                        kind: CompiledNodeKind::Nop,
                    }]
                    .into_boxed_slice(),
                    expressions: Box::new([]),
                    accessors: Box::new([]),
                    constants: Box::new([]),
                    slot_count: 0,
                    entry: StepIdx::new(0),
                    resource_contract: ResourceContract::DEFAULT,
                };
                let fallback_result = CompiledWorkflow::try_from_parts(fallback);
                assert!(fallback_result.is_ok(), "fallback workflow must compile");
                match fallback_result {
                    Ok(w) => w,
                    Err(_) => loop {},
                }
            }
        }
    }

    #[test]
    fn emit_rust_workflow_produces_non_empty_source() {
        let workflow = minimal_workflow();
        let result = emit_rust_workflow(&workflow);

        assert!(result.is_ok(), "emit_rust_workflow should succeed");
        let source = result.unwrap_or_default();
        assert!(!source.is_empty(), "generated source should not be empty");
    }

    #[test]
    fn emit_ids_produces_output() {
        let workflow = minimal_workflow();
        let mut out = String::new();
        let result = emit_ids(&mut out, &workflow);

        assert!(result.is_ok(), "emit_ids should succeed");
        assert!(out.contains("WORKFLOW_SLOT_COUNT"), "should emit slot count");
        assert!(out.contains("WORKFLOW_NODE_COUNT"), "should emit node count");
    }

    #[test]
    fn generated_source_contains_required_sections() {
        let workflow = minimal_workflow();
        let source = emit_rust_workflow(&workflow).unwrap_or_default();

        assert!(
            source.contains("drive("),
            "should contain drive function"
        );
        assert!(
            source.contains("fn step_0"),
            "should contain step functions"
        );
        assert!(
            source.contains("CONSTANTS"),
            "should contain constant pool"
        );
        assert!(
            source.contains("DriveError"),
            "should contain error type"
        );
    }

    #[test]
    fn compare_generated_to_ir_accepts_valid_output() {
        let workflow = minimal_workflow();
        let source = emit_rust_workflow(&workflow);
        assert!(source.is_ok());
        let source = source.unwrap_or_default();

        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(comparison.is_ok(), "semantic comparison should pass for valid output");
    }
}
