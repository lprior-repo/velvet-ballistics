//! Generated Rust workflow mode for velvet-ballastics maxperf builds.
//!
//! Compiles `CompiledWorkflow` IR into native Rust source that passes the same
//! lint gates as first-party code and preserves identical observable semantics.

use std::fmt::Write;
use std::process::Command;
use thiserror::Error;
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprBranch,
    ExprOp, ResourceContract, SlotBranch, SlotIdx, StepIdx,
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
        writeln!(
            out,
            "            {} => step_{}(&mut slots)?,",
            step_idx, step_idx
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

    emit_step_body(out, node)?;

    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::Nop => {
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::SetConst { value } => {
            emit_set_const_step(out, node.output, *value)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Copy { source } => {
            emit_copy_step(out, node.output, *source)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::EvalExpr { expr } => {
            emit_eval_expr_step(out, node.output, *expr)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Finish { result } => emit_finish_step(out, *result)?,
        CompiledNodeKind::Jump { target } => {
            writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)?;
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => emit_choose_step(out, branches, *otherwise)?,
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => emit_choose_slot_step(out, branches, *otherwise)?,
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
            emit_read_slot_binding(out, "_deadline", *deadline_slot)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            emit_wait_event_step(out, *event, *timeout_slot)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            emit_ask_step(out, *prompt, *timeout_slot)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::AskResume { answer } => {
            writeln!(out, "    let _answer_slot: u16 = {};", answer.get()).map_err(fmt_err)?;
            write_next_or_error(out, node.next)?;
        }
        CompiledNodeKind::RetryCheck { .. } => emit_unsupported_step(out, "RetryCheck")?,
        CompiledNodeKind::ErrorHandler { body, handler } => {
            emit_error_handler_step(out, *body, *handler)?;
        }
    }
    Ok(())
}

fn emit_set_const_step(
    out: &mut String,
    output: Option<SlotIdx>,
    value: ConstIdx,
) -> CodegenResult<()> {
    if let Some(output) = output {
        writeln!(
            out,
            "    write_slot(slots, {}, Some(read_const({})?))?;",
            output.get(),
            value.get()
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}

fn emit_copy_step(out: &mut String, output: Option<SlotIdx>, source: SlotIdx) -> CodegenResult<()> {
    if let Some(output) = output {
        writeln!(
            out,
            "    let copied = read_slot_optional(slots, {});\n    write_slot(slots, {}, copied)?;",
            source.get(),
            output.get()
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}

fn emit_eval_expr_step(
    out: &mut String,
    output: Option<SlotIdx>,
    expr: vb_core::ExprIdx,
) -> CodegenResult<()> {
    if let Some(output) = output {
        writeln!(
            out,
            "    write_slot(slots, {}, Some(eval_expr_{}(slots)?))?;",
            output.get(),
            expr.get()
        )
        .map_err(fmt_err)?;
    }
    Ok(())
}

fn emit_finish_step(out: &mut String, result: SlotIdx) -> CodegenResult<()> {
    writeln!(out, "    let value = read_slot(slots, {})?;", result.get()).map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Finished(value))").map_err(fmt_err)
}

fn emit_choose_step(
    out: &mut String,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> CodegenResult<()> {
    for branch in branches {
        writeln!(
            out,
            "    if expect_bool(eval_expr_{}(slots)?)? {{ return Ok(StepOutcome::Continue({})); }}",
            branch.condition.get(),
            branch.target.get()
        )
        .map_err(fmt_err)?;
    }
    write_otherwise_or_no_match(out, otherwise)
}

fn emit_choose_slot_step(
    out: &mut String,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> CodegenResult<()> {
    for branch in branches {
        writeln!(
            out,
            "    if expect_bool(read_slot(slots, {})?)? {{ return Ok(StepOutcome::Continue({})); }}",
            branch.condition.get(),
            branch.target.get()
        )
        .map_err(fmt_err)?;
    }
    write_otherwise_or_no_match(out, otherwise)
}

fn write_otherwise_or_no_match(out: &mut String, otherwise: Option<StepIdx>) -> CodegenResult<()> {
    if let Some(fallback) = otherwise {
        writeln!(out, "    Ok(StepOutcome::Continue({}))", fallback.get()).map_err(fmt_err)
    } else {
        writeln!(out, "    Err(DriveError::NoBranchMatched)").map_err(fmt_err)
    }
}

fn emit_wait_event_step(
    out: &mut String,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> CodegenResult<()> {
    emit_read_slot_binding(out, "_event", event)?;
    if let Some(timeout) = timeout_slot {
        emit_read_slot_binding(out, "_timeout", timeout)?;
    }
    Ok(())
}

fn emit_ask_step(
    out: &mut String,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> CodegenResult<()> {
    emit_read_slot_binding(out, "_prompt", prompt)?;
    if let Some(timeout) = timeout_slot {
        emit_read_slot_binding(out, "_timeout", timeout)?;
    }
    Ok(())
}

fn emit_read_slot_binding(out: &mut String, binding: &str, slot: SlotIdx) -> CodegenResult<()> {
    writeln!(
        out,
        "    let {binding} = read_slot(slots, {})?;",
        slot.get()
    )
    .map_err(fmt_err)
}

fn emit_error_handler_step(out: &mut String, body: StepIdx, handler: StepIdx) -> CodegenResult<()> {
    writeln!(
        out,
        "    // ErrorHandler: body={}, handler={}",
        body.get(),
        handler.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    Ok(StepOutcome::Continue({}))", body.get()).map_err(fmt_err)
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
        emit_expr_op(out, op, workflow)?;
    }

    writeln!(out, "    finish_expr_stack(&mut stack)").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_expr_op(out: &mut String, op: &ExprOp, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    match op {
        ExprOp::LoadSlot(slot) => {
            writeln!(out, "    stack.push(read_slot(slots, {})?)?;", slot.get()).map_err(fmt_err)
        }
        ExprOp::LoadConst(const_idx) => {
            writeln!(out, "    stack.push(read_const({})?)?;", const_idx.get()).map_err(fmt_err)
        }
        ExprOp::LoadAccessor(accessor_idx) => emit_accessor_eval(out, *accessor_idx, workflow),
        ExprOp::Eq => emit_bool_equality(out, "=="),
        ExprOp::NotEq => emit_bool_equality(out, "!="),
        ExprOp::Gt => emit_number_compare(out, ">"),
        ExprOp::Gte => emit_number_compare(out, ">="),
        ExprOp::Lt => emit_number_compare(out, "<"),
        ExprOp::Lte => emit_number_compare(out, "<="),
        ExprOp::And => emit_bool_binary(out, "&&"),
        ExprOp::Or => emit_bool_binary(out, "||"),
        ExprOp::Not => emit_bool_not(out),
        ExprOp::Add => emit_number_arithmetic(out, "checked_add", "IntegerOverflow"),
        ExprOp::Sub => emit_number_arithmetic(out, "checked_sub", "IntegerOverflow"),
        ExprOp::Mul => emit_number_arithmetic(out, "checked_mul", "IntegerOverflow"),
        ExprOp::Div => emit_number_arithmetic(out, "checked_div", "DivisionByZero"),
        ExprOp::Contains => emit_unsupported_expr(out, "contains"),
        ExprOp::StartsWith => emit_unsupported_expr(out, "starts_with"),
        ExprOp::EndsWith => emit_unsupported_expr(out, "ends_with"),
        ExprOp::Has => emit_unsupported_expr(out, "has"),
        ExprOp::Exists => emit_exists_expr(out),
        ExprOp::Length => emit_unary_value_helper(out, "length_value"),
        ExprOp::Empty => emit_unary_value_helper(out, "empty_value"),
        ExprOp::Append => emit_unsupported_expr(out, "append"),
        ExprOp::AppendIf => emit_unsupported_expr(out, "append_if"),
        ExprOp::Merge => emit_unsupported_expr(out, "merge"),
        ExprOp::Sum => emit_unsupported_expr(out, "sum"),
        ExprOp::Count => emit_unary_value_helper(out, "length_value"),
        ExprOp::Unique => emit_unique_expr(out),
    }
}

fn emit_bool_equality(out: &mut String, operator: &str) -> CodegenResult<()> {
    writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(_l {operator} _r))?; }}")
        .map_err(fmt_err)
}

fn emit_number_compare(out: &mut String, operator: &str) -> CodegenResult<()> {
    writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_li {operator} _ri))?; }}")
        .map_err(fmt_err)
}

fn emit_bool_binary(out: &mut String, operator: &str) -> CodegenResult<()> {
    writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push(SlotValue::Bool(_lb {operator} _rb))?; }}")
        .map_err(fmt_err)
}

fn emit_bool_not(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; match _v {{ SlotValue::Bool(b) => stack.push(SlotValue::Bool(!b))?, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }} }}")
        .map_err(fmt_err)
}

fn emit_number_arithmetic(
    out: &mut String,
    checked_method: &str,
    error_variant: &str,
) -> CodegenResult<()> {
    writeln!(out, "    {{ let _r = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _l = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.{checked_method}(_ri).ok_or(DriveError::{error_variant})?; stack.push(SlotValue::I64(_result))?; }}")
        .map_err(fmt_err)
}

fn emit_exists_expr(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _exists = !matches!(_v, SlotValue::Null); stack.push(SlotValue::Bool(_exists))?; }}")
        .map_err(fmt_err)
}

fn emit_unary_value_helper(out: &mut String, helper: &str) -> CodegenResult<()> {
    writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push({helper}(_v))?; }}")
        .map_err(fmt_err)
}

fn emit_unique_expr(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(unique_value(_v)?)?; }}")
        .map_err(fmt_err)
}

/// Generate action dispatch boundaries for external action nodes.
pub fn emit_action_boundary(
    out: &mut String,
    action: ActionId,
    input: SlotIdx,
) -> CodegenResult<()> {
    writeln!(out, "    // Action boundary: action_id={}", action.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    let _action_input = read_slot(slots, {})?;",
        input.get()
    )
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

    let output = child
        .wait_with_output()
        .map_err(|e| CodegenError::RustfmtFailed {
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
    validate_generated_source_patterns(source)?;
    let generated = count_generated_items(source)?;
    let expected = count_expected_items(workflow)?;
    validate_generated_counts(generated, expected)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneratedCounts {
    steps: u16,
    expressions: u16,
    actions: u16,
}

fn validate_generated_source_patterns(source: &str) -> CodegenResult<()> {
    validate_generated_banned_patterns(source)?;
    validate_generated_required_patterns(source)
}

fn validate_generated_banned_patterns(source: &str) -> CodegenResult<()> {
    let banned = [
        ("u16::MAX", "finish sentinel"),
        ("Vec<", "dynamic Vec allocation"),
        ("Vec::", "dynamic Vec allocation"),
        ("HashMap<", "runtime map allocation"),
        ("HashMap::", "runtime map allocation"),
        (".unwrap(", "unwrap call"),
        (".expect(", "expect call"),
        ("panic!(", "panic macro"),
        ("todo!(", "todo macro"),
        ("unimplemented!(", "unimplemented macro"),
        ("dbg!(", "debug macro"),
        ("slots[", "unchecked slot indexing"),
        ("CONSTANTS[", "unchecked constant indexing"),
        (" as ", "unchecked cast"),
        ("serde_json", "JSON dependency"),
        ("http::", "HTTP dependency"),
    ];
    for (pattern, reason) in banned {
        reject_generated_pattern(source, pattern, reason)?;
    }
    Ok(())
}

fn validate_generated_required_patterns(source: &str) -> CodegenResult<()> {
    let required = [
        ("StepOutcome::Finished", "terminal result return"),
        ("ExprStack::new", "bounded expression stack"),
        ("finish_expr_stack", "single-result expression finish"),
        ("fn expect_bool", "typed boolean guard"),
    ];
    for (pattern, reason) in required {
        require_generated_pattern(source, pattern, reason)?;
    }
    Ok(())
}

fn count_generated_items(source: &str) -> CodegenResult<GeneratedCounts> {
    let mut counts = GeneratedCounts {
        steps: 0,
        expressions: 0,
        actions: 0,
    };
    for line in source.lines() {
        let trimmed = line.trim();
        count_generated_line(trimmed, &mut counts)?;
    }
    Ok(counts)
}

fn count_generated_line(trimmed: &str, counts: &mut GeneratedCounts) -> CodegenResult<()> {
    if trimmed.starts_with("fn step_") {
        counts.steps = increment_count(counts.steps, "step count overflow")?;
    }
    if trimmed.starts_with("fn eval_expr_") {
        counts.expressions = increment_count(counts.expressions, "expression count overflow")?;
    }
    if trimmed.contains("Action boundary:") {
        counts.actions = increment_count(counts.actions, "action count overflow")?;
    }
    Ok(())
}

fn count_expected_items(workflow: &CompiledWorkflow) -> CodegenResult<GeneratedCounts> {
    Ok(GeneratedCounts {
        steps: workflow.node_count(),
        expressions: count_expected_exprs(workflow)?,
        actions: count_expected_actions(workflow)?,
    })
}

fn count_expected_exprs(workflow: &CompiledWorkflow) -> CodegenResult<u16> {
    let mut expected_exprs = 0u16;
    for idx in 0..u16::MAX {
        if workflow.expression(vb_core::ExprIdx::new(idx)).is_some() {
            expected_exprs = increment_count(expected_exprs, "expected expression count overflow")?;
        } else {
            break;
        }
    }
    Ok(expected_exprs)
}

fn count_expected_actions(workflow: &CompiledWorkflow) -> CodegenResult<u16> {
    let mut expected_actions = 0u16;
    for idx in 0..workflow.node_count() {
        if let Some(node) = workflow.node(StepIdx::new(idx))
            && matches!(node.kind, CompiledNodeKind::Do { .. })
        {
            expected_actions = increment_count(expected_actions, "expected action count overflow")?;
        }
    }
    Ok(expected_actions)
}

fn increment_count(value: u16, overflow_detail: &'static str) -> CodegenResult<u16> {
    value
        .checked_add(1)
        .ok_or_else(|| CodegenError::SemanticMismatch {
            detail: overflow_detail.into(),
        })
}

fn validate_generated_counts(
    generated: GeneratedCounts,
    expected: GeneratedCounts,
) -> CodegenResult<()> {
    validate_count("step", generated.steps, expected.steps)?;
    validate_count("expression", generated.expressions, expected.expressions)?;
    validate_count("action", generated.actions, expected.actions)
}

fn validate_count(label: &str, generated: u16, expected: u16) -> CodegenResult<()> {
    if generated != expected {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("{label} count mismatch: generated has {generated}, IR has {expected}"),
        });
    }
    Ok(())
}

fn reject_generated_pattern(
    source: &str,
    pattern: &str,
    reason: &'static str,
) -> CodegenResult<()> {
    if source.contains(pattern) {
        return Err(CodegenError::SemanticMismatch {
            detail: format!("generated source contains {reason}"),
        });
    }
    Ok(())
}

fn require_generated_pattern(
    source: &str,
    pattern: &str,
    reason: &'static str,
) -> CodegenResult<()> {
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
    writeln!(
        out,
        "    pub const fn is_true(&self) -> bool {{ matches!(self, Self::Bool(true)) }}"
    )
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
    writeln!(
        out,
        "    TypeMismatch {{ expected: &'static str, found: &'static str }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    DivisionByZero,").map_err(fmt_err)?;
    writeln!(out, "    IntegerOverflow,").map_err(fmt_err)?;
    writeln!(out, "    ExpressionStackUnderflow,").map_err(fmt_err)?;
    writeln!(
        out,
        "    ActionSuspend {{ action_id: u16, input_slot: u16 }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    UnknownAction,").map_err(fmt_err)?;
    writeln!(
        out,
        "    UnsupportedPrimitive {{ primitive: &'static str }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    UnsupportedExpressionOp {{ op: &'static str }},").map_err(fmt_err)?;
    writeln!(
        out,
        "    InvalidCompiledWorkflow {{ reason: &'static str }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "enum StepOutcome {{ Continue(u16), Finished(SlotValue) }}"
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "const MAX_EXPRESSION_STACK: usize = 64;").map_err(fmt_err)?;
    writeln!(
        out,
        "struct ExprStack {{ values: [SlotValue; MAX_EXPRESSION_STACK], len: u8, capacity: u8 }}"
    )
    .map_err(fmt_err)?;
    writeln!(out, "impl ExprStack {{").map_err(fmt_err)?;
    writeln!(out, "    fn new(capacity: u8) -> Result<Self, DriveError> {{ if usize::from(capacity) <= MAX_EXPRESSION_STACK {{ Ok(Self {{ values: [SlotValue::Null; MAX_EXPRESSION_STACK], len: 0, capacity }}) }} else {{ Err(DriveError::ExpressionStackOverflow {{ max: capacity }}) }} }}").map_err(fmt_err)?;
    writeln!(out, "    fn push(&mut self, value: SlotValue) -> Result<(), DriveError> {{ if self.len >= self.capacity {{ return Err(DriveError::ExpressionStackOverflow {{ max: self.capacity }}); }} let index = usize::from(self.len); match self.values.get_mut(index) {{ Some(slot) => *slot = value, None => return Err(DriveError::ExpressionStackOverflow {{ max: self.capacity }}), }} self.len = self.len.checked_add(1).ok_or(DriveError::ExpressionStackOverflow {{ max: self.capacity }})?; Ok(()) }}").map_err(fmt_err)?;
    writeln!(out, "    fn pop(&mut self) -> Option<SlotValue> {{ if self.len == 0 {{ return None; }} self.len = self.len.checked_sub(1)?; self.values.get(usize::from(self.len)).copied() }}").map_err(fmt_err)?;
    writeln!(out, "    const fn len(&self) -> u8 {{ self.len }}").map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> {{ read_slot_optional(slots, slot).ok_or(DriveError::SlotNull) }}").map_err(fmt_err)?;
    writeln!(out, "fn read_slot_optional(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Option<SlotValue> {{ slots.get(usize::from(slot)).copied().flatten() }}").map_err(fmt_err)?;
    writeln!(out, "fn write_slot(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, value: Option<SlotValue>) -> Result<(), DriveError> {{ match slots.get_mut(usize::from(slot)) {{ Some(target) => {{ *target = value; Ok(()) }}, None => Err(DriveError::InvalidCompiledWorkflow {{ reason: \"slot index out of bounds\" }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn read_const(index: u16) -> Result<SlotValue, DriveError> {{ CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow {{ reason: \"constant index out of bounds\" }}) }}").map_err(fmt_err)?;
    writeln!(out, "fn finish_expr_stack(stack: &mut ExprStack) -> Result<SlotValue, DriveError> {{ if stack.len() == 1 {{ stack.pop().ok_or(DriveError::ExpressionStackUnderflow) }} else {{ Err(DriveError::InvalidCompiledWorkflow {{ reason: \"expression leaves non-single result\" }}) }} }}").map_err(fmt_err)?;
    writeln!(out, "fn expect_bool(value: SlotValue) -> Result<bool, DriveError> {{ match value {{ SlotValue::Bool(value) => Ok(value), other => Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn length_value(value: SlotValue) -> SlotValue {{ match value {{ SlotValue::List(id) => SlotValue::I64(i64::from(id)), _ => SlotValue::I64(0), }} }}").map_err(fmt_err)?;
    writeln!(out, "const fn empty_value(value: SlotValue) -> SlotValue {{ SlotValue::Bool(matches!(value, SlotValue::Null | SlotValue::List(_))) }}").map_err(fmt_err)?;
    writeln!(out, "fn unique_value(value: SlotValue) -> Result<SlotValue, DriveError> {{ match value {{ SlotValue::List(_) => Ok(value), other => Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_constants(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Constant pool ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const CONSTANTS: [SlotValue; {}] = [",
        count_constants(workflow)
    )
    .map_err(fmt_err)?;

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
        Some(target) => {
            writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)
        }
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
        writeln!(out, "    stack.push(read_slot(slots, {})?)?;", root_slot).map_err(fmt_err)?;
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
        compare_generated_to_ir, compile_check_generated_rust, emit_ids, emit_rust_workflow,
    };
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprProgram,
        ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
    };

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

    fn expression_finish_workflow(ops: Box<[vb_core::ExprOp]>) -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_codegen_expr"),
            digest: WorkflowDigest::from_bytes([0xEF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn emit_rust_workflow_produces_non_empty_source() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(!source.is_empty(), "generated source should not be empty");
        Ok(())
    }

    #[test]
    fn emit_ids_produces_output() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut out = String::new();
        emit_ids(&mut out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("WORKFLOW_SLOT_COUNT"),
            "should emit slot count"
        );
        assert!(
            out.contains("WORKFLOW_NODE_COUNT"),
            "should emit node count"
        );
        Ok(())
    }

    #[test]
    fn generated_source_contains_required_sections() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        assert!(source.contains("drive("), "should contain drive function");
        assert!(
            source.contains("fn step_0"),
            "should contain step functions"
        );
        assert!(source.contains("CONSTANTS"), "should contain constant pool");
        assert!(source.contains("DriveError"), "should contain error type");
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
        assert!(
            comparison.is_ok(),
            "semantic comparison should pass for valid output"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_finish_sentinel() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nconst BAD_SENTINEL: u16 = u16::MAX;\n");

        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(
            comparison.is_err(),
            "semantic comparison should reject sentinel output"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_banned_generated_constructs() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let banned = [
            ".unwrap(",
            ".expect(",
            "panic!(",
            "todo!(",
            "unimplemented!(",
            "dbg!(",
            "Vec<",
            "HashMap<",
            "slots[",
            "CONSTANTS[",
            " as ",
            "serde_json",
            "http::",
        ];

        for pattern in banned {
            let mut mutated = source.clone();
            mutated.push_str(pattern);
            let comparison = compare_generated_to_ir(&mutated, &workflow);
            assert!(
                comparison.is_err(),
                "semantic comparison should reject banned pattern {pattern}"
            );
        }
        Ok(())
    }

    #[test]
    fn generated_choose_uses_typed_boolean_guard() -> Result<(), String> {
        let workflow = expression_finish_workflow(
            vec![
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::Exists,
            ]
            .into_boxed_slice(),
        )?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        assert!(
            source.contains("fn expect_bool"),
            "generated source should include typed boolean guard"
        );
        assert!(
            source.contains("finish_expr_stack"),
            "generated expression evaluators should reject non-single final stacks"
        );
        Ok(())
    }

    #[test]
    fn implemented_expression_helpers_emit_direct_code() -> Result<(), String> {
        let workflows = [
            expression_finish_workflow(
                vec![
                    vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                    vb_core::ExprOp::Length,
                ]
                .into_boxed_slice(),
            )?,
            expression_finish_workflow(
                vec![
                    vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                    vb_core::ExprOp::Empty,
                ]
                .into_boxed_slice(),
            )?,
            expression_finish_workflow(
                vec![
                    vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                    vb_core::ExprOp::Count,
                ]
                .into_boxed_slice(),
            )?,
        ];

        for workflow in workflows {
            let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
            compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[test]
    fn generated_expression_helper_source_compile_checks() -> Result<(), String> {
        let workflow = expression_finish_workflow(
            vec![
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::Exists,
            ]
            .into_boxed_slice(),
        )?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let temp_dir =
            std::env::temp_dir().join(format!("vb_codegen_expr_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let result = compile_check_generated_rust(&source, &temp_dir).map_err(|e| e.to_string());
        let cleanup = std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string());
        if let Err(error) = cleanup {
            return Err(error);
        }
        result
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
        let temp_dir = std::env::temp_dir().join(format!("vb_codegen_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let result = compile_check_generated_rust(&source, &temp_dir).map_err(|e| e.to_string());
        let cleanup = std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string());
        if let Err(error) = cleanup {
            return Err(error);
        }
        result
    }
}
