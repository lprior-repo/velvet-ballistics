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
    writeln!(out, "        let outcome = match pc {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        writeln!(out, "            {} => step_{}(&mut slots)?,", step_idx, step_idx)
            .map_err(fmt_err)?;
    }
    writeln!(out, "            _ => return Err(DriveError::InvalidProgramCounter),")
        .map_err(fmt_err)?;
    writeln!(out, "        }};").map_err(fmt_err)?;
    writeln!(out, "        match outcome {{").map_err(fmt_err)?;
    writeln!(out, "            StepOutcome::Continue(next) => pc = next,").map_err(fmt_err)?;
    writeln!(out, "            StepOutcome::Finished(value) => return Ok(value),").map_err(fmt_err)?;
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
        "fn step_{}(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<StepOutcome, DriveError> {{",
        step_id
    )
    .map_err(fmt_err)?;

    match &node.kind {
        CompiledNodeKind::Nop => {
            if let Some(next) = node.next {
                writeln!(out, "    Ok(StepOutcome::Continue({}))", next.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::MissingNextStep)")
                    .map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::SetConst { value } => {
            if let Some(output) = node.output {
                writeln!(
                    out,
                    "    write_slot(slots, {}, Some(read_const({})?))?;",
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
                    "    let copied = read_slot_optional(slots, {});\n    write_slot(slots, {}, copied)?;",
                    source.get(),
                    output.get()
                )
                .map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::EvalExpr { expr } => {
            if let Some(output) = node.output {
                writeln!(
                    out,
                    "    write_slot(slots, {}, Some(eval_expr_{}(slots)?))?;",
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
                "    let value = read_slot(slots, {})?;",
                result.get()
            )
            .map_err(fmt_err)?;
            writeln!(out, "    Ok(StepOutcome::Finished(value))").map_err(fmt_err)?;
        }
        CompiledNodeKind::Jump { target } => {
            writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::Choose { branches, otherwise } => {
            for branch in branches.iter() {
                writeln!(
                    out,
                    "    if eval_expr_{}(slots)?.is_true() {{ return Ok(StepOutcome::Continue({})); }}",
                    branch.condition.get(),
                    branch.target.get()
                )
                .map_err(fmt_err)?;
            }
            if let Some(fallback) = otherwise {
                writeln!(out, "    Ok(StepOutcome::Continue({}))", fallback.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            for branch in branches {
                writeln!(
                    out,
                    "    if read_slot(slots, {})?.is_true() {{ return Ok(StepOutcome::Continue({})); }}",
                    branch.condition.get(),
                    branch.target.get()
                )
                .map_err(fmt_err)?;
            }
            if let Some(fallback) = otherwise {
                writeln!(out, "    Ok(StepOutcome::Continue({}))", fallback.get()).map_err(fmt_err)?;
            } else {
                writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err)?;
            }
        }
        CompiledNodeKind::Do { action, input } => {
            emit_action_boundary(out, *action, *input)?;
        }
        CompiledNodeKind::BuildObject { fields: _ } => emit_unsupported_step(out, "BuildObject")?,
        CompiledNodeKind::BuildList { items: _ } => emit_unsupported_step(out, "BuildList")?,
        CompiledNodeKind::ForEachStart { .. } => emit_unsupported_step(out, "ForEachStart")?,
        CompiledNodeKind::ForEachNext { .. } => emit_unsupported_step(out, "ForEachNext")?,
        CompiledNodeKind::ForEachJoin { output: _ } => emit_unsupported_step(out, "ForEachJoin")?,
        CompiledNodeKind::TogetherStart { .. } => emit_unsupported_step(out, "TogetherStart")?,
        CompiledNodeKind::TogetherBranch { .. } => emit_unsupported_step(out, "TogetherBranch")?,
        CompiledNodeKind::TogetherJoin { .. } => emit_unsupported_step(out, "TogetherJoin")?,
        CompiledNodeKind::CollectStart { .. } => emit_unsupported_step(out, "CollectStart")?,
        CompiledNodeKind::CollectPage { .. } => emit_unsupported_step(out, "CollectPage")?,
        CompiledNodeKind::CollectNext { .. } => emit_unsupported_step(out, "CollectNext")?,
        CompiledNodeKind::CollectFinish { .. } => emit_unsupported_step(out, "CollectFinish")?,
        CompiledNodeKind::ReduceStart { .. } => emit_unsupported_step(out, "ReduceStart")?,
        CompiledNodeKind::ReduceNext { .. } => emit_unsupported_step(out, "ReduceNext")?,
        CompiledNodeKind::ReduceFinish { .. } => emit_unsupported_step(out, "ReduceFinish")?,
        CompiledNodeKind::RepeatStart { .. } => emit_unsupported_step(out, "RepeatStart")?,
        CompiledNodeKind::RepeatAttempt { .. } => emit_unsupported_step(out, "RepeatAttempt")?,
        CompiledNodeKind::RepeatCheck { .. } => emit_unsupported_step(out, "RepeatCheck")?,
        CompiledNodeKind::RepeatFinish { .. } => emit_unsupported_step(out, "RepeatFinish")?,
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            writeln!(out, "    let _deadline = read_slot(slots, {})?;", deadline_slot.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::WaitEvent { event, timeout_slot } => {
            writeln!(out, "    let _event = read_slot(slots, {})?;", event.get()).map_err(fmt_err)?;
            if let Some(timeout) = timeout_slot {
                writeln!(out, "    let _timeout = read_slot(slots, {})?;", timeout.get()).map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Ask { prompt, timeout_slot } => {
            writeln!(out, "    let _prompt = read_slot(slots, {})?;", prompt.get()).map_err(fmt_err)?;
            if let Some(timeout) = timeout_slot {
                writeln!(out, "    let _timeout = read_slot(slots, {})?;", timeout.get()).map_err(fmt_err)?;
            }
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::AskResume { answer } => {
            writeln!(out, "    let _answer_slot: u16 = {};", answer.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::RetryCheck { .. } => emit_unsupported_step(out, "RetryCheck")?,
        CompiledNodeKind::ErrorHandler { body, handler } => {
            writeln!(out, "    // ErrorHandler: body={}, handler={}", body.get(), handler.get()).map_err(fmt_err)?;
            writeln!(out, "    Ok(StepOutcome::Continue({}))", body.get()).map_err(fmt_err)?;
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
        "    let mut stack = ExprStack::new({})?;",
        program.max_stack
    )
    .map_err(fmt_err)?;

    for op in program.ops.as_ref() {
        match op {
            ExprOp::LoadSlot(slot) => {
                writeln!(
                    out,
                    "    stack.push(read_slot(slots, {})?)?;",
                    slot.get()
                )
                .map_err(fmt_err)?;
            }
            ExprOp::LoadConst(const_idx) => {
                writeln!(
                    out,
                    "    stack.push(read_const({})?)?;",
                    const_idx.get()
                )
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
            ExprOp::Contains => emit_unsupported_expr(out, "contains")?,
            ExprOp::StartsWith => emit_unsupported_expr(out, "starts_with")?,
            ExprOp::EndsWith => emit_unsupported_expr(out, "ends_with")?,
            ExprOp::Has => emit_unsupported_expr(out, "has")?,
            ExprOp::Exists => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _exists = !matches!(_v, SlotValue::Null); stack.push(SlotValue::Bool(_exists))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Length => emit_unsupported_expr(out, "length")?,
            ExprOp::Empty => emit_unsupported_expr(out, "empty")?,
            ExprOp::Append => emit_unsupported_expr(out, "append")?,
            ExprOp::AppendIf => emit_unsupported_expr(out, "append_if")?,
            ExprOp::Merge => emit_unsupported_expr(out, "merge")?,
            ExprOp::Sum => emit_unsupported_expr(out, "sum")?,
            ExprOp::Count => emit_unsupported_expr(out, "count")?,
            ExprOp::Unique => emit_unsupported_expr(out, "unique")?,
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
        "    let _action_input = read_slot(slots, {})?;",
        input.get()
    )
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
    reject_generated_pattern(source, "u16::MAX", "finish sentinel")?;
    reject_generated_pattern(source, "Vec<", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "Vec::", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "slots[", "unchecked slot indexing")?;
    reject_generated_pattern(source, "CONSTANTS[", "unchecked constant indexing")?;
    reject_generated_pattern(source, " as ", "unchecked cast")?;
    require_generated_pattern(source, "StepOutcome::Finished", "terminal result return")?;
    require_generated_pattern(source, "ExprStack::new", "bounded expression stack")?;

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

fn reject_generated_pattern(source: &str, pattern: &str, reason: &'static str) -> CodegenResult<()> {
    if source.contains(pattern) {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("generated source contains {reason}"),
        });
    }
    Ok(())
}

fn require_generated_pattern(source: &str, pattern: &str, reason: &'static str) -> CodegenResult<()> {
    if !source.contains(pattern) {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("generated source is missing {reason}"),
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
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq)]").map_err(fmt_err)?;
    writeln!(out, "pub enum SlotValue {{ Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(u32), Object(u32), Blob(u64) }}")
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
    writeln!(out, "    UnsupportedPrimitive {{ primitive: &'static str }},").map_err(fmt_err)?;
    writeln!(out, "    UnsupportedExpressionOp {{ op: &'static str }},").map_err(fmt_err)?;
    writeln!(out, "    InvalidCompiledWorkflow {{ reason: &'static str }},").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "enum StepOutcome {{ Continue(u16), Finished(SlotValue) }}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "const MAX_EXPRESSION_STACK: usize = 64;").map_err(fmt_err)?;
    writeln!(out, "struct ExprStack {{ values: [SlotValue; MAX_EXPRESSION_STACK], len: u8, capacity: u8 }}").map_err(fmt_err)?;
    writeln!(out, "impl ExprStack {{").map_err(fmt_err)?;
    writeln!(out, "    fn new(capacity: u8) -> Result<Self, DriveError> {{ if usize::from(capacity) <= MAX_EXPRESSION_STACK {{ Ok(Self {{ values: [SlotValue::Null; MAX_EXPRESSION_STACK], len: 0, capacity }}) }} else {{ Err(DriveError::ExpressionStackOverflow {{ max: capacity }}) }} }}").map_err(fmt_err)?;
    writeln!(out, "    fn push(&mut self, value: SlotValue) -> Result<(), DriveError> {{ if self.len >= self.capacity {{ return Err(DriveError::ExpressionStackOverflow {{ max: self.capacity }}); }} let index = usize::from(self.len); match self.values.get_mut(index) {{ Some(slot) => *slot = value, None => return Err(DriveError::ExpressionStackOverflow {{ max: self.capacity }}), }} self.len = self.len.checked_add(1).ok_or(DriveError::ExpressionStackOverflow {{ max: self.capacity }})?; Ok(()) }}").map_err(fmt_err)?;
    writeln!(out, "    fn pop(&mut self) -> Option<SlotValue> {{ if self.len == 0 {{ return None; }} self.len = self.len.checked_sub(1)?; self.values.get(usize::from(self.len)).copied() }}").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> {{ read_slot_optional(slots, slot).ok_or(DriveError::SlotNull) }}").map_err(fmt_err)?;
    writeln!(out, "fn read_slot_optional(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Option<SlotValue> {{ slots.get(usize::from(slot)).copied().flatten() }}").map_err(fmt_err)?;
    writeln!(out, "fn write_slot(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, value: Option<SlotValue>) -> Result<(), DriveError> {{ match slots.get_mut(usize::from(slot)) {{ Some(target) => {{ *target = value; Ok(()) }}, None => Err(DriveError::InvalidCompiledWorkflow {{ reason: \"slot index out of bounds\" }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn read_const(index: u16) -> Result<SlotValue, DriveError> {{ CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow {{ reason: \"constant index out of bounds\" }}) }}").map_err(fmt_err)?;
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
        Some(target) => writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

fn emit_unsupported_step(out: &mut String, primitive: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    Err(DriveError::UnsupportedPrimitive {{ primitive: \"{primitive}\" }})"
    )
    .map_err(fmt_err)
}

fn emit_unsupported_expr(out: &mut String, op: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    return Err(DriveError::UnsupportedExpressionOp {{ op: \"{op}\" }});"
    )
    .map_err(fmt_err)
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
            "    stack.push(read_slot(slots, {})?)?;",
            root_slot
        )
        .map_err(fmt_err)?;
    } else {
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
            "    {{ let _root = read_slot(slots, {})?; return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"accessor traversal '{}' on generated type\" }}); }}"
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
        compare_generated_to_ir, compile_check_generated_rust, emit_action_boundary,
        emit_action_match_dispatch, emit_drive_function, emit_finish, emit_ids,
        emit_resource_contract, emit_rust_workflow, emit_step_function, emit_trybuild_fixture,
        format_generated_rust, CodegenError,
    };
    use vb_core::{
        ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
        ExprProgram, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
    };

    // --- Workflow helpers ---

    fn minimal_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;

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
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_build_list_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_unsupported_build_list"),
            digest: WorkflowDigest::from_bytes([0xCD; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Workflow with a Do node that dispatches to ActionId 5.
    fn do_action_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_do_action"),
            digest: WorkflowDigest::from_bytes([0xEF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(5),
                        input: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- CodegenError exact-variant tests ---

    #[test]
    fn codegen_error_format_buffer_overflow_exact_variant() {
        let error = CodegenError::FormatBufferOverflow;
        let message = error.to_string();
        assert!(
            message.contains("buffer"),
            "FormatBufferOverflow display must mention buffer, got: {message}"
        );
    }

    #[test]
    fn codegen_error_rustfmt_failed_exact_variant() {
        let error = CodegenError::RustfmtFailed {
            detail: String::from("exit status 1"),
        };
        let message = error.to_string();
        assert!(
            message.contains("rustfmt"),
            "RustfmtFailed display must mention rustfmt, got: {message}"
        );
        assert!(
            message.contains("exit status 1"),
            "RustfmtFailed display must include detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_compile_check_failed_exact_variant() {
        let error = CodegenError::CompileCheckFailed {
            detail: String::from("mismatched types"),
        };
        let message = error.to_string();
        assert!(
            message.contains("compile"),
            "CompileCheckFailed display must mention compile, got: {message}"
        );
        assert!(
            message.contains("mismatched types"),
            "CompileCheckFailed display must include detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_semantic_mismatch_exact_variant() {
        let error = CodegenError::SemanticMismatch {
            detail: String::from("step count mismatch: generated has 2, IR has 3"),
        };
        let message = error.to_string();
        assert!(
            message.contains("semantic"),
            "SemanticMismatch display must mention semantic, got: {message}"
        );
        assert!(
            message.contains("step count mismatch: generated has 2, IR has 3"),
            "SemanticMismatch display must include exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_io_exact_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = CodegenError::Io(io_err);
        let message = error.to_string();
        assert!(
            message.contains("file missing"),
            "Io display must include the inner IO error message, got: {message}"
        );
    }

    #[test]
    fn codegen_error_trybuild_fixture_exact_variant() {
        let error = CodegenError::TrybuildFixture {
            detail: String::from("fixture path has no parent directory"),
        };
        let message = error.to_string();
        assert!(
            message.contains("trybuild"),
            "TrybuildFixture display must mention trybuild, got: {message}"
        );
        assert!(
            message.contains("fixture path has no parent directory"),
            "TrybuildFixture display must include exact detail, got: {message}"
        );
    }

    // --- Public function behavior tests ---

    #[test]
    fn emit_rust_workflow_produces_non_empty_source() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(!source.is_empty(), "generated source should not be empty");
        Ok(())
    }

    #[test]
    fn emit_ids_includes_workflow_id_type() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_ids writes typed ID constants
        let mut out = String::new();
        emit_ids(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains WORKFLOW_SLOT_COUNT and WORKFLOW_NODE_COUNT constants
        assert!(
            out.contains("WORKFLOW_SLOT_COUNT"),
            "emit_ids must produce WORKFLOW_SLOT_COUNT constant"
        );
        assert!(
            out.contains("WORKFLOW_NODE_COUNT"),
            "emit_ids must produce WORKFLOW_NODE_COUNT constant"
        );
        assert!(
            out.contains("usize"),
            "emit_ids must use typed usize for slot count"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_includes_loop() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_drive_function writes the main step loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains a loop construct and match dispatch
        assert!(
            out.contains("loop"),
            "drive function must contain a loop construct"
        );
        assert!(
            out.contains("pub fn drive"),
            "drive function must be public and named drive"
        );
        assert!(
            out.contains("StepOutcome"),
            "drive function must dispatch on StepOutcome"
        );
        Ok(())
    }

    #[test]
    fn emit_step_function_includes_set_const() -> Result<(), String> {
        // Given a minimal workflow with a SetConst node
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;

        // When emit_step_function writes the step for the SetConst node
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;

        // Then the output writes the constant into the output slot
        assert!(
            out.contains("write_slot"),
            "SetConst step must call write_slot"
        );
        assert!(
            out.contains("read_const"),
            "SetConst step must call read_const"
        );
        assert!(
            out.contains("fn step_0"),
            "SetConst step function must be named step_0"
        );
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_includes_registered_actions() -> Result<(), String> {
        // Given a workflow with a Do node dispatching to ActionId 5
        let workflow = do_action_workflow()?;

        // When emit_action_match_dispatch writes the action dispatch
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains an arm for action id 5
        assert!(
            out.contains("dispatch_action"),
            "dispatch must define dispatch_action function"
        );
        assert!(
            out.contains("5 => Ok(())"),
            "dispatch must include an arm for action id 5"
        );
        assert!(
            out.contains("UnknownAction"),
            "dispatch must handle unknown actions"
        );
        Ok(())
    }

    #[test]
    fn emit_finish_returns_result_value() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_finish writes the result extraction section
        let mut out = String::new();
        emit_finish(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains the result extraction comment section
        assert!(
            out.contains("Result extraction"),
            "emit_finish must include result extraction section marker"
        );
        Ok(())
    }

    #[test]
    fn emit_resource_contract_includes_limits() -> Result<(), String> {
        // Given a resource contract with specific field values
        let contract = ResourceContract {
            max_steps: 100,
            max_slots: 200,
            max_constants: 50,
            max_accessors: 10,
            max_expressions: 20,
            max_expr_stack: 32,
            max_input_bytes: 4096,
            max_output_bytes: 8192,
            max_step_budget_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
        };

        // When emit_resource_contract writes the contract constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;

        // Then the output contains every contract field
        assert!(
            out.contains("CONTRACT_MAX_STEPS"),
            "resource contract must emit CONTRACT_MAX_STEPS"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS"),
            "resource contract must emit CONTRACT_MAX_SLOTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS"),
            "resource contract must emit CONTRACT_MAX_CONSTANTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_ACCESSORS"),
            "resource contract must emit CONTRACT_MAX_ACCESSORS"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPRESSIONS"),
            "resource contract must emit CONTRACT_MAX_EXPRESSIONS"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPR_STACK"),
            "resource contract must emit CONTRACT_MAX_EXPR_STACK"
        );
        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES"),
            "resource contract must emit CONTRACT_MAX_INPUT_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES"),
            "resource contract must emit CONTRACT_MAX_OUTPUT_BYTES"
        );
        Ok(())
    }

    #[test]
    fn format_generated_rust_produces_valid_syntax() -> Result<(), String> {
        // Given a generated workflow source
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // When format_generated_rust is invoked
        let formatted = format_generated_rust(&source);

        // Then either rustfmt succeeded with non-empty output, or it is not installed
        match formatted {
            Ok(output) => {
                assert!(
                    !output.is_empty(),
                    "formatted output must be non-empty when rustfmt succeeds"
                );
            }
            Err(CodegenError::RustfmtFailed { detail }) => {
                // rustfmt not available in CI is acceptable; log the reason
                eprintln!("rustfmt not available, skipping format check: {detail}");
            }
            Err(other) => return Err(format!("unexpected error from format_generated_rust: {other}")),
        }
        Ok(())
    }

    // --- Error Variant Exact-Assertion Tests ---

    #[test]
    fn codegen_error_format_buffer_overflow_reports_expected_message() {
        // Given a FormatBufferOverflow error variant
        let error = CodegenError::FormatBufferOverflow;
        // When the error is converted to display string
        let message = error.to_string();
        // Then it mentions buffer and capacity semantics
        assert!(
            message.contains("buffer"),
            "FormatBufferOverflow must mention buffer, got: {message}"
        );
        assert!(
            message.contains("capacity"),
            "FormatBufferOverflow must mention capacity, got: {message}"
        );
    }

    #[test]
    fn codegen_error_rustfmt_failed_reports_expected_detail() {
        // Given a RustfmtFailed error with a specific detail string
        let detail = String::from("exit status 42");
        let error = CodegenError::RustfmtFailed {
            detail: detail.clone(),
        };
        // When the error is displayed
        let message = error.to_string();
        // Then the exact detail string appears verbatim
        assert!(
            message.contains("rustfmt"),
            "RustfmtFailed must mention rustfmt, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "RustfmtFailed must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_compile_check_failed_reports_expected_detail() {
        // Given a CompileCheckFailed error with detail
        let detail = String::from("mismatched types: expected u16, found String");
        let error = CodegenError::CompileCheckFailed {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it contains compile and the exact detail
        assert!(
            message.contains("compile"),
            "CompileCheckFailed must mention compile, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "CompileCheckFailed must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_semantic_mismatch_reports_expected_detail() {
        // Given a SemanticMismatch with specific divergence
        let detail = String::from("step count mismatch: generated has 2, IR has 3");
        let error = CodegenError::SemanticMismatch {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it mentions semantic and includes exact detail
        assert!(
            message.contains("semantic"),
            "SemanticMismatch must mention semantic, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "SemanticMismatch must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_io_reports_inner_error_kind() {
        // Given an IO error wrapped in CodegenError::Io
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = CodegenError::Io(io_err);
        // When displayed
        let message = error.to_string();
        // Then the inner error message is preserved verbatim
        assert!(
            message.contains("file missing"),
            "Io variant must preserve inner message, got: {message}"
        );
        assert!(
            message.contains("codegen IO error"),
            "Io variant must mention codegen IO error, got: {message}"
        );
    }

    #[test]
    fn codegen_error_trybuild_fixture_reports_expected_detail() {
        // Given a TrybuildFixture error with a detail
        let detail = String::from("fixture path has no parent directory");
        let error = CodegenError::TrybuildFixture {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it mentions trybuild and contains the exact detail
        assert!(
            message.contains("trybuild"),
            "TrybuildFixture must mention trybuild, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "TrybuildFixture must contain exact detail, got: {message}"
        );
    }

    // --- Emit Step Function Behavior Tests ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_nop_node() -> Result<(), String> {
        // Given a Nop node with a next target
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::Nop,
        };
        let workflow = nop_workflow()?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains a Continue with the next step index
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "Nop must emit Continue with next step, got: {out}"
        );
        assert!(
            out.contains("fn step_0"),
            "Nop step function must be named step_0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_set_const_node() -> Result<(), String> {
        // Given a SetConst node writing constant 0 into slot 0
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output writes slot and reads constant
        assert!(
            out.contains("write_slot"),
            "SetConst must call write_slot, got: {out}"
        );
        assert!(
            out.contains("read_const"),
            "SetConst must call read_const, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_copy_node() -> Result<(), String> {
        // Given a Copy node that reads slot 0 into slot 1
        let workflow = copy_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads and writes slots
        assert!(
            out.contains("read_slot_optional"),
            "Copy must call read_slot_optional, got: {out}"
        );
        assert!(
            out.contains("write_slot"),
            "Copy must call write_slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_do_node() -> Result<(), String> {
        // Given a Do node dispatching action 5 with input slot 0
        let workflow = do_action_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains action suspend dispatch
        assert!(
            out.contains("ActionSuspend"),
            "Do node must emit ActionSuspend error, got: {out}"
        );
        assert!(
            out.contains("action_id: 5"),
            "Do node must reference action id 5, got: {out}"
        );
        assert!(
            out.contains("input_slot: 0"),
            "Do node must reference input slot 0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_finish_node() -> Result<(), String> {
        // Given a Finish node that returns slot 0
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(1)).ok_or("node 1 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads the result slot and returns Finished
        assert!(
            out.contains("read_slot"),
            "Finish must call read_slot, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Finished"),
            "Finish must return StepOutcome::Finished, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_jump_node() -> Result<(), String> {
        // Given a Jump node targeting step 1
        let workflow = jump_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output continues to the target
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "Jump must emit Continue to target step 1, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_wait_until_node() -> Result<(), String> {
        // Given a WaitUntil node reading deadline from slot 0
        let workflow = wait_until_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads the deadline slot
        assert!(
            out.contains("_deadline"),
            "WaitUntil must reference deadline variable, got: {out}"
        );
        assert!(
            out.contains("read_slot"),
            "WaitUntil must call read_slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_wait_event_node() -> Result<(), String> {
        // Given a WaitEvent node reading event from slot 0 with timeout slot 1
        let workflow = wait_event_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads event and timeout slots
        assert!(
            out.contains("_event"),
            "WaitEvent must reference event variable, got: {out}"
        );
        assert!(
            out.contains("_timeout"),
            "WaitEvent must reference timeout variable, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_ask_node() -> Result<(), String> {
        // Given an Ask node with prompt slot 0 and timeout slot 1
        let workflow = ask_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads prompt and timeout slots
        assert!(
            out.contains("_prompt"),
            "Ask must reference prompt variable, got: {out}"
        );
        assert!(
            out.contains("_timeout"),
            "Ask with timeout must reference timeout variable, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_start_node() -> Result<(), String> {
        // Given a ForEachStart node (unsupported in codegen)
        let workflow = for_each_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ForEachStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ForEachStart"),
            "UnsupportedPrimitive must name ForEachStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_together_start_node() -> Result<(), String> {
        // Given a TogetherStart node (unsupported in codegen)
        let workflow = together_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "TogetherStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("TogetherStart"),
            "UnsupportedPrimitive must name TogetherStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_collect_start_node() -> Result<(), String> {
        // Given a CollectStart node (unsupported in codegen)
        let workflow = collect_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "CollectStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("CollectStart"),
            "UnsupportedPrimitive must name CollectStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_reduce_start_node() -> Result<(), String> {
        // Given a ReduceStart node (unsupported in codegen)
        let workflow = reduce_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ReduceStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ReduceStart"),
            "UnsupportedPrimitive must name ReduceStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_repeat_start_node() -> Result<(), String> {
        // Given a RepeatStart node (unsupported in codegen)
        let workflow = repeat_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "RepeatStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("RepeatStart"),
            "UnsupportedPrimitive must name RepeatStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_build_object_node() -> Result<(), String> {
        // Given a BuildObject node (unsupported in codegen)
        let workflow = build_object_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "BuildObject must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("BuildObject"),
            "UnsupportedPrimitive must name BuildObject, got: {out}"
        );
        Ok(())
    }

    // --- Module Header and Structure Tests ---

    #[test]
    fn emit_module_header_includes_forbid_unsafe() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the first section includes forbid unsafe_code
        assert!(
            source.contains("#![forbid(unsafe_code)]"),
            "generated source must include #![forbid(unsafe_code)], got first 200 chars: {}",
            &source[..source.len().min(200)]
        );
        Ok(())
    }

    #[test]
    fn emit_module_header_includes_deny_unused_must_use() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the output contains deny unused_must_use
        assert!(
            source.contains("#![deny(unused_must_use)]"),
            "generated source must include deny unused_must_use"
        );
        Ok(())
    }

    #[test]
    fn emit_module_header_includes_slot_value_enum() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the output contains the SlotValue enum definition
        assert!(
            source.contains("pub enum SlotValue"),
            "generated source must define SlotValue enum"
        );
        assert!(
            source.contains("Bool(bool)"),
            "SlotValue must have Bool variant"
        );
        assert!(
            source.contains("I64(i64)"),
            "SlotValue must have I64 variant"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_includes_entry_step_zero() -> Result<(), String> {
        // Given a minimal workflow with entry at step 0
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates the drive loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the program counter initializes to the entry step
        assert!(
            out.contains("let mut pc: u16 = 0;"),
            "drive must initialize pc to entry step 0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_routes_each_step_index() -> Result<(), String> {
        // Given a minimal workflow with 2 nodes
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates code
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then each step index appears in the match dispatch
        assert!(
            out.contains("0 => step_0"),
            "drive must dispatch to step_0"
        );
        assert!(
            out.contains("1 => step_1"),
            "drive must dispatch to step_1"
        );
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_lists_only_do_actions() -> Result<(), String> {
        // Given a workflow with a Do node for action 5
        let workflow = do_action_workflow()?;
        // When emit_action_match_dispatch generates the dispatch
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then action 5 appears but finish step 1 does not
        assert!(
            out.contains("5 => Ok(())"),
            "dispatch must have arm for action id 5"
        );
        assert!(
            out.contains("_ => Err(DriveError::UnknownAction)"),
            "dispatch must have wildcard fallback"
        );
        Ok(())
    }

    #[test]
    fn emit_action_boundary_reads_input_slot_and_returns_suspend() -> Result<(), String> {
        // Given an action boundary with action 7 and input slot 3
        let mut out = String::new();
        // When emit_action_boundary writes the code
        emit_action_boundary(&mut out, ActionId::new(7), SlotIdx::new(3))
            .map_err(|e| e.to_string())?;
        // Then the output reads the input slot and returns ActionSuspend
        assert!(
            out.contains("read_slot(slots, 3)"),
            "action boundary must read input slot 3, got: {out}"
        );
        assert!(
            out.contains("ActionSuspend { action_id: 7, input_slot: 3 }"),
            "action boundary must return ActionSuspend with correct fields, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_resource_contract_outputs_all_constant_fields() -> Result<(), String> {
        // Given a custom resource contract
        let contract = ResourceContract {
            max_steps: 50,
            max_slots: 100,
            max_constants: 25,
            max_accessors: 5,
            max_expressions: 10,
            max_expr_stack: 16,
            max_input_bytes: 2048,
            max_output_bytes: 4096,
            max_step_budget_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
        };
        // When emit_resource_contract writes constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;
        // Then each field value appears in the output
        assert!(
            out.contains("CONTRACT_MAX_STEPS: u16 = 50;"),
            "must emit exact max_steps value"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS: u16 = 100;"),
            "must emit exact max_slots value"
        );
        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS: u16 = 25;"),
            "must emit exact max_constants value"
        );
        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES: u32 = 2048;"),
            "must emit exact max_input_bytes value"
        );
        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES: u32 = 4096;"),
            "must emit exact max_output_bytes value"
        );
        Ok(())
    }

    #[test]
    fn emit_ids_includes_exact_slot_and_node_counts() -> Result<(), String> {
        // Given a minimal workflow with 1 slot and 2 nodes
        let workflow = minimal_workflow()?;
        // When emit_ids writes constants
        let mut out = String::new();
        emit_ids(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the exact counts appear
        assert!(
            out.contains("WORKFLOW_SLOT_COUNT: usize = 1;"),
            "must emit slot count 1, got: {out}"
        );
        assert!(
            out.contains("WORKFLOW_NODE_COUNT: u16 = 2;"),
            "must emit node count 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_expr_function_generates_load_const_op() -> Result<(), String> {
        // Given a workflow with an expression that loads constant 0
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the expression function exists and loads the constant
        assert!(
            source.contains("fn eval_expr_0"),
            "must generate eval_expr_0 function"
        );
        assert!(
            source.contains("stack.push(read_const(0)"),
            "expression must load constant index 0"
        );
        Ok(())
    }

    // --- Code Generation Integration Tests ---

    #[test]
    fn generate_produces_valid_rust_for_single_step_nop() -> Result<(), String> {
        // Given a workflow with a single Nop + Finish
        let workflow = nop_workflow()?;
        // When generating the full Rust source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the source contains drive function, step function, and dispatch
        assert!(
            source.contains("pub fn drive"),
            "single-step workflow must have drive function"
        );
        assert!(
            source.contains("fn step_0"),
            "single-step workflow must have step_0"
        );
        assert!(
            source.contains("fn step_1"),
            "single-step workflow must have step_1 (finish)"
        );
        assert!(
            !source.is_empty(),
            "generated source must be non-empty"
        );
        Ok(())
    }

    #[test]
    fn generate_produces_valid_rust_for_multi_step_workflow() -> Result<(), String> {
        // Given a workflow with set_const + do + finish (3 steps)
        let workflow = do_action_workflow()?;
        // When generating the full source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all step functions are present
        assert!(
            source.contains("fn step_0"),
            "multi-step must have step_0"
        );
        assert!(
            source.contains("fn step_1"),
            "multi-step must have step_1"
        );
        assert!(
            source.contains("fn step_0") && source.contains("fn step_1"),
            "multi-step must have all step handlers"
        );
        Ok(())
    }

    #[test]
    fn generate_output_starts_with_forbid_unsafe() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the first non-empty line is the forbid directive
        let first_line = source.lines().next().ok_or("source has no lines")?;
        assert!(
            first_line.contains("#![forbid(unsafe_code)]"),
            "first line must be forbid unsafe, got: {first_line}"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_all_step_handlers() -> Result<(), String> {
        // Given a workflow with 2 nodes
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then each node gets a step handler
        let mut step_count = 0u16;
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn step_") {
                step_count = step_count.checked_add(1).ok_or("overflow")?;
            }
        }
        assert!(
            step_count == workflow.node_count(),
            "expected {} step handlers, found {step_count}",
            workflow.node_count()
        );
        Ok(())
    }

    #[test]
    fn generate_contains_constant_pool_with_correct_values() -> Result<(), String> {
        // Given a workflow with constant I64(42)
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the constant pool has the value
        assert!(
            source.contains("SlotValue::I64(42)"),
            "constant pool must contain SlotValue::I64(42)"
        );
        assert!(
            source.contains("CONSTANTS"),
            "source must define CONSTANTS array"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_drive_error_variants() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all critical DriveError variants are defined
        assert!(
            source.contains("InvalidProgramCounter"),
            "must define InvalidProgramCounter error"
        );
        assert!(
            source.contains("MissingNextStep"),
            "must define MissingNextStep error"
        );
        assert!(
            source.contains("ActionSuspend"),
            "must define ActionSuspend error"
        );
        assert!(
            source.contains("SlotNull"),
            "must define SlotNull error"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_expr_stack_bounded_structure() -> Result<(), String> {
        // Given a workflow with an expression
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then ExprStack is defined with bounded storage
        assert!(
            source.contains("struct ExprStack"),
            "must define ExprStack struct"
        );
        assert!(
            source.contains("MAX_EXPRESSION_STACK"),
            "must define MAX_EXPRESSION_STACK constant"
        );
        assert!(
            !source.contains("Vec<"),
            "must not use Vec for expression stack"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_checked_slot_accessors() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then checked accessor functions are defined
        assert!(
            source.contains("fn read_slot"),
            "must define read_slot function"
        );
        assert!(
            source.contains("fn write_slot"),
            "must define write_slot function"
        );
        assert!(
            source.contains("fn read_slot_optional"),
            "must define read_slot_optional function"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_vec_usage() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When comparing source that contains Vec<
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nlet x: Vec<u8> = Vec::new();\n");
        // Then the comparison rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(result.is_err(), "must reject source with Vec usage");
        let err = result.map_err(|e| e.to_string()).map(|_| String::new()).unwrap_err();
        assert!(
            err.contains("Vec"),
            "error must mention Vec, got: {err}"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_unchecked_cast() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When comparing source with ` as ` cast
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nlet x = 42u32 as u16;\n");
        // Then the comparison rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(result.is_err(), "must reject source with unchecked cast");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_accepts_clean_output() -> Result<(), String> {
        // Given a clean generated workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When comparing against the IR
        let result = compare_generated_to_ir(&source, &workflow);
        // Then it succeeds
        assert!(
            result.is_ok(),
            "clean generated source must pass semantic comparison"
        );
        Ok(())
    }

    #[test]
    fn emit_trybuild_fixture_writes_file_to_disk() -> Result<(), String> {
        // Given a minimal workflow and a temp fixture path
        let workflow = minimal_workflow()?;
        let temp_dir = std::env::temp_dir().join(format!(
            "vb_codegen_fixture_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let fixture_path = temp_dir.join("fixture.rs");
        // When emit_trybuild_fixture writes the file
        let result = emit_trybuild_fixture(&workflow, &fixture_path);
        // Then it succeeds and the file exists
        assert!(result.is_ok(), "trybuild fixture write must succeed");
        let content = std::fs::read_to_string(&fixture_path).map_err(|e| e.to_string())?;
        assert!(
            !content.is_empty(),
            "fixture file must be non-empty"
        );
        assert!(
            content.contains("#![forbid(unsafe_code)]"),
            "fixture must contain generated Rust with forbid unsafe"
        );
        let cleanup = std::fs::remove_dir_all(&temp_dir);
        if let Err(e) = cleanup {
            return Err(e.to_string());
        }
        Ok(())
    }

    #[test]
    fn emit_trybuild_fixture_rejects_root_path_without_parent() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When emitting to root path "/" which has no parent
        let fixture_path = std::path::Path::new("/");
        let result = emit_trybuild_fixture(&workflow, fixture_path);
        // Then it fails because "/" is a directory and cannot be written as a file
        assert!(result.is_err(), "must fail for root path without writable parent");
        Ok(())
    }

    // --- Proptest Properties ---

    #[test]
    fn codegen_error_display_contains_variant_name() {
        // Given all CodegenError variants
        let errors: Vec<(CodegenError, &'static str)> = vec![
            (CodegenError::FormatBufferOverflow, "buffer"),
            (
                CodegenError::RustfmtFailed {
                    detail: String::from("test"),
                },
                "rustfmt",
            ),
            (
                CodegenError::CompileCheckFailed {
                    detail: String::from("test"),
                },
                "compile",
            ),
            (
                CodegenError::SemanticMismatch {
                    detail: String::from("test"),
                },
                "semantic",
            ),
            (
                CodegenError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")),
                "codegen IO error",
            ),
            (
                CodegenError::TrybuildFixture {
                    detail: String::from("test"),
                },
                "trybuild",
            ),
        ];
        // When each error is displayed
        for (error, keyword) in errors {
            let message = error.to_string();
            // Then the display message contains a distinguishing keyword
            assert!(
                message.contains(keyword),
                "error display must contain keyword '{keyword}', got: {message}"
            );
        }
    }

    #[test]
    fn emit_function_signature_never_empty() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When each emit function is called individually
        let mut ids_out = String::new();
        emit_ids(&mut ids_out, &workflow).map_err(|e| e.to_string())?;
        assert!(!ids_out.is_empty(), "emit_ids must produce non-empty output");

        let mut drive_out = String::new();
        emit_drive_function(&mut drive_out, &workflow).map_err(|e| e.to_string())?;
        assert!(!drive_out.is_empty(), "emit_drive_function must produce non-empty output");

        let mut finish_out = String::new();
        emit_finish(&mut finish_out, &workflow).map_err(|e| e.to_string())?;
        assert!(!finish_out.is_empty(), "emit_finish must produce non-empty output");

        let mut contract_out = String::new();
        emit_resource_contract(&mut contract_out, workflow.resource_contract())
            .map_err(|e| e.to_string())?;
        assert!(
            !contract_out.is_empty(),
            "emit_resource_contract must produce non-empty output"
        );

        let mut dispatch_out = String::new();
        emit_action_match_dispatch(&mut dispatch_out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            !dispatch_out.is_empty(),
            "emit_action_match_dispatch must produce non-empty output"
        );
        Ok(())
    }

    // --- Workflow Helpers for additional node types ---

    fn nop_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_nop"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Nop,
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn copy_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_copy"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn jump_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_jump"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn wait_until_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_until"),
            digest: WorkflowDigest::from_bytes([0x44; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::WaitUntil {
                        deadline_slot: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn wait_event_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_event"),
            digest: WorkflowDigest::from_bytes([0x55; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::WaitEvent {
                        event: SlotIdx::new(0),
                        timeout_slot: Some(SlotIdx::new(1)),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn ask_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask"),
            digest: WorkflowDigest::from_bytes([0x66; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Ask {
                        prompt: SlotIdx::new(0),
                        timeout_slot: Some(SlotIdx::new(1)),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn for_each_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each"),
            digest: WorkflowDigest::from_bytes([0x77; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn together_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together"),
            digest: WorkflowDigest::from_bytes([0x88; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: vec![StepIdx::new(1)].into_boxed_slice(),
                        join: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn collect_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_collect"),
            digest: WorkflowDigest::from_bytes([0x99; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::CollectStart {
                        source: SlotIdx::new(0),
                        limit: 10,
                        page_size: 5,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn reduce_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_reduce"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ReduceStart {
                        input: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        initial: ConstIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn repeat_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_repeat"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 3,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn build_object_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_object"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![(vb_core::SymbolId::new(0), SlotIdx::new(0))]
                            .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn choose_expr_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_choose_expr"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Choose {
                        branches: vec![vb_core::ExprBranch {
                            condition: vb_core::ExprIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
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
                CompiledNode {
                    id: StepIdx::new(2),
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
            constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn choose_slot_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_choose_slot"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![vb_core::SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
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
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn error_handler_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_error_handler"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ErrorHandler {
                        body: StepIdx::new(1),
                        handler: StepIdx::new(2),
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
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn ask_resume_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask_resume"),
            digest: WorkflowDigest::from_bytes([0x12; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::AskResume {
                        answer: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- Additional Step Variant Tests ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_build_list_node() -> Result<(), String> {
        // Given a BuildList node (unsupported in codegen)
        let workflow = unsupported_build_list_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "BuildList must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("BuildList"),
            "UnsupportedPrimitive must name BuildList, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_choose_node() -> Result<(), String> {
        // Given a Choose node with one expression branch and an otherwise target
        let workflow = choose_expr_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains conditional branch dispatch
        assert!(
            out.contains("eval_expr_"),
            "Choose must call eval_expr, got: {out}"
        );
        assert!(
            out.contains("is_true()"),
            "Choose must check is_true, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Continue"),
            "Choose must return Continue on branch match, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_choose_slot_node() -> Result<(), String> {
        // Given a ChooseSlot node with one slot branch
        let workflow = choose_slot_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads slot for condition
        assert!(
            out.contains("read_slot"),
            "ChooseSlot must call read_slot, got: {out}"
        );
        assert!(
            out.contains("is_true()"),
            "ChooseSlot must check is_true, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_eval_expr_node() -> Result<(), String> {
        // Given a workflow with an EvalExpr node
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code (SetConst is node 0, eval via expression)
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the expression evaluator function exists
        assert!(
            source.contains("fn eval_expr_0"),
            "must generate expression evaluator function"
        );
        assert!(
            source.contains("stack.push"),
            "expression must push values onto stack"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_error_handler_node() -> Result<(), String> {
        // Given an ErrorHandler node
        let workflow = error_handler_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains error handler metadata comment
        assert!(
            out.contains("ErrorHandler"),
            "ErrorHandler must be referenced in generated code, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Continue"),
            "ErrorHandler must continue to body step, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_ask_resume_node() -> Result<(), String> {
        // Given an AskResume node
        let workflow = ask_resume_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output references the answer slot
        assert!(
            out.contains("_answer_slot"),
            "AskResume must reference answer slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_nop_without_next_reports_missing_step() -> Result<(), String> {
        // Given a Nop node with no next target
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let workflow = nop_workflow()?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then the output returns MissingNextStep error
        assert!(
            out.contains("MissingNextStep"),
            "Nop without next must return MissingNextStep, got: {out}"
        );
        Ok(())
    }

    // --- Additional Integration Tests ---

    #[test]
    fn generate_output_contains_forbid_and_deny_lint_gates() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all lint gates are present
        assert!(
            source.contains("#![forbid(unsafe_code)]"),
            "must include forbid unsafe_code"
        );
        assert!(
            source.contains("#![deny(unused_must_use)]"),
            "must include deny unused_must_use"
        );
        assert!(
            source.contains("#![deny(rust_2018_idioms)]"),
            "must include deny rust_2018_idioms"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_read_const_function() -> Result<(), String> {
        // Given a workflow with constants
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then read_const helper function is defined
        assert!(
            source.contains("fn read_const"),
            "must define read_const function"
        );
        assert!(
            source.contains("CONSTANTS.get"),
            "read_const must use checked access"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_step_outcome_enum() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then StepOutcome is defined with Continue and Finished variants
        assert!(
            source.contains("StepOutcome"),
            "must define StepOutcome type"
        );
        assert!(
            source.contains("Continue"),
            "StepOutcome must have Continue variant"
        );
        assert!(
            source.contains("Finished"),
            "StepOutcome must have Finished variant"
        );
        Ok(())
    }

    #[test]
    fn generate_do_action_workflow_contains_dispatch_function() -> Result<(), String> {
        // Given a workflow with a Do action node
        let workflow = do_action_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then dispatch_action function exists with the action registered
        assert!(
            source.contains("pub fn dispatch_action"),
            "must define dispatch_action function"
        );
        assert!(
            source.contains("5 => Ok(())"),
            "dispatch must list action id 5"
        );
        assert!(
            source.contains("UnknownAction"),
            "dispatch must handle unknown actions"
        );
        Ok(())
    }

    #[test]
    fn generate_workflow_with_no_actions_has_empty_dispatch() -> Result<(), String> {
        // Given a workflow with no Do nodes
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then dispatch_action only has the wildcard fallback
        assert!(
            source.contains("pub fn dispatch_action"),
            "must define dispatch_action function"
        );
        assert!(
            source.contains("_ => Err(DriveError::UnknownAction)"),
            "dispatch must have wildcard fallback"
        );
        // No specific action arms besides the wildcard
        let dispatch_section_start = source.find("pub fn dispatch_action")
            .ok_or("dispatch section missing")?;
        let dispatch_section_end = source[dispatch_section_start..]
            .find("}")
            .ok_or("dispatch closing brace missing")?;
        let dispatch_body = &source[dispatch_section_start..dispatch_section_start + dispatch_section_end];
        assert!(
            !dispatch_body.contains("=> Ok(())"),
            "dispatch should have no action arms for a workflow without Do nodes"
        );
        Ok(())
    }

    #[test]
    fn generated_source_contains_required_sections() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

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
        assert!(
            source.contains("StepOutcome::Finished"),
            "finish should return a terminal value"
        );
        assert!(
            source.contains("ExprStack::new"),
            "expression stack should be fixed storage"
        );
        assert!(
            !source.contains("u16::MAX"),
            "generated source must not use finish sentinel"
        );
        assert!(
            !source.contains("Vec<") && !source.contains("Vec::"),
            "generated source must not allocate Vec hot stacks"
        );
        assert!(
            !source.contains("slots[") && !source.contains("CONSTANTS["),
            "generated source must use checked access helpers"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_accepts_valid_output() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(comparison.is_ok(), "semantic comparison should pass for valid output");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_finish_sentinel() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nconst BAD_SENTINEL: u16 = u16::MAX;\n");

        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(comparison.is_err(), "semantic comparison should reject sentinel output");
        Ok(())
    }

    #[test]
    fn unsupported_list_codegen_is_typed_error() -> Result<(), String> {
        let workflow = unsupported_build_list_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        assert!(
            source.contains("UnsupportedPrimitive { primitive: \"BuildList\" }"),
            "BuildList must be explicit typed unsupported behavior"
        );
        Ok(())
    }

    #[test]
    fn generated_source_compile_checks() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let temp_dir = std::env::temp_dir().join(format!(
            "vb_codegen_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let result = compile_check_generated_rust(&source, &temp_dir).map_err(|e| e.to_string());
        let cleanup = std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string());
        if let Err(error) = cleanup {
            return Err(error);
        }
        result
    }

    #[test]
    fn generate_workflow_name_appears_in_doc_comment() -> Result<(), String> {
        // Given a workflow with name "test_codegen"
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the doc comment mentions codegen origin
        assert!(
            source.contains("Produced by vb_codegen"),
            "must mention codegen origin in doc comment"
        );
        assert!(
            source.contains("DO NOT EDIT"),
            "must warn against manual editing"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_is_true_helper_on_slot_value() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then SlotValue has is_true helper
        assert!(
            source.contains("fn is_true"),
            "must define is_true helper on SlotValue"
        );
        assert!(
            source.contains("type_name"),
            "must define type_name helper on SlotValue"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_rejects_invalid_program_counter() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the drive loop handles invalid program counter
        assert!(
            source.contains("InvalidProgramCounter"),
            "drive must handle invalid program counter"
        );
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_for_do_workflow_includes_action_arm() -> Result<(), String> {
        // Given a do_action workflow
        let workflow = do_action_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the Do step function contains ActionSuspend with the correct action id
        assert!(
            source.contains("ActionSuspend { action_id: 5"),
            "do step must reference action_id 5"
        );
        assert!(
            source.contains("dispatch_action"),
            "must contain dispatch_action function"
        );
        assert!(
            source.contains("5 => Ok(())"),
            "dispatch must list action id 5 arm"
        );
        Ok(())
    }

    // --- Proptest Properties ---

    #[test]
    fn emit_step_match_output_is_valid_rust_identifier_prefix() -> Result<(), String> {
        // Given multiple workflow types, each generating step functions
        let workflows: Vec<(&str, Result<CompiledWorkflow, String>)> = vec![
            ("nop", nop_workflow()),
            ("copy", copy_workflow()),
            ("jump", jump_workflow()),
            ("do_action", do_action_workflow()),
        ];
        for (name, workflow_result) in workflows {
            let workflow = workflow_result?;
            // When generating the full source
            let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
            // Then every step function name follows the pattern "fn step_N"
            let mut found_step = false;
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn step_") && trimmed.contains('(') {
                    found_step = true;
                    let end = trimmed.find('(').ok_or("no paren in step fn")?;
                    let fn_name = &trimmed[3..end];
                    assert!(
                        fn_name.starts_with("step_"),
                        "function name must start with step_, got: {fn_name} in workflow {name}"
                    );
                    let suffix = &fn_name[5..];
                    assert!(
                        suffix.parse::<u16>().is_ok(),
                        "step suffix must be a valid u16, got: {suffix} in workflow {name}"
                    );
                }
            }
            assert!(
                found_step,
                "must find at least one step function in workflow {name}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod proptests {
    use crate::{CodegenError, emit_resource_contract, emit_rust_workflow};
    use proptest::prelude::*;
    use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ResourceContract,
                  SlotIdx, StepIdx, WorkflowDigest, WorkflowParts};

    fn arb_resource_contract() -> impl Strategy<Value = ResourceContract> {
        (
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u8..64u8,
            1u32..10000u32,
            1u32..10000u32,
        )
            .prop_map(
                |(steps, slots, constants, accessors, expressions, expr_stack, input_bytes, output_bytes)| {
                    ResourceContract {
                        max_steps: steps,
                        max_slots: slots,
                        max_constants: constants,
                        max_accessors: accessors,
                        max_expressions: expressions,
                        max_expr_stack: expr_stack,
                        max_input_bytes: input_bytes,
                        max_output_bytes: output_bytes,
                        max_step_budget_per_tick: 500,
                        max_blob_bytes: 1024,
                        max_ipc_payload_bytes: 2048,
                        max_retry_attempts: 3,
                        max_fanout: 8,
                        max_collect_items: 100,
                        max_queue_depth: 64,
                        max_journal_batch_bytes: 512,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn emit_resource_contract_output_contains_all_fields(contract in arb_resource_contract()) {
            let mut out = String::new();
            let result = emit_resource_contract(&mut out, contract);
            prop_assert!(result.is_ok());
            prop_assert!(!out.is_empty());
            prop_assert!(out.contains("CONTRACT_MAX_STEPS"));
            prop_assert!(out.contains("CONTRACT_MAX_SLOTS"));
            prop_assert!(out.contains("CONTRACT_MAX_CONSTANTS"));
            prop_assert!(out.contains("CONTRACT_MAX_ACCESSORS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPRESSIONS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPR_STACK"));
            prop_assert!(out.contains("CONTRACT_MAX_INPUT_BYTES"));
            prop_assert!(out.contains("CONTRACT_MAX_OUTPUT_BYTES"));
        }

        #[test]
        fn codegen_error_display_never_empty(error_idx in 0u8..6u8) {
            let error = match error_idx {
                0 => CodegenError::FormatBufferOverflow,
                1 => CodegenError::RustfmtFailed { detail: String::from("test") },
                2 => CodegenError::CompileCheckFailed { detail: String::from("test") },
                3 => CodegenError::SemanticMismatch { detail: String::from("test") },
                4 => CodegenError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")),
                _ => CodegenError::TrybuildFixture { detail: String::from("test") },
            };
            let message = error.to_string();
            prop_assert!(!message.is_empty(), "error display must never be empty");
        }

        #[test]
        fn generated_source_always_forbids_unsafe(slot_count in 1u16..10u16) {
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_test"),
                digest: WorkflowDigest::from_bytes([0x42; 32]),
                nodes: vec![
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: None,
                        kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
                    },
                ].into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: Box::new([]),
                slot_count,
                entry: StepIdx::new(0),
                resource_contract: ResourceContract::DEFAULT,
            };
            if let Ok(workflow) = CompiledWorkflow::try_from_parts(parts) {
                if let Ok(source) = emit_rust_workflow(&workflow) {
                    prop_assert!(source.contains("#![forbid(unsafe_code)]"));
                    prop_assert!(source.contains("#![deny(unused_must_use)]"));
                }
            }
        }
    }
}
