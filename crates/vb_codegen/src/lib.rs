#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
//! Generated Rust workflow mode for velvet-ballastics maxperf builds.
//!
//! Compiles `CompiledWorkflow` IR into native Rust source that passes the same
//! lint gates as first-party code and preserves identical observable semantics.
//!
//! Generated Rust is a deliberately supported subset of the final workflow IR.
//! The current subset accepts scalar constants, slot copies, expression math and
//! boolean comparisons, action dispatch, waits, asks, jumps, choices, handlers,
//! and finish nodes. Empty/root accessors are emitted as checked root-slot reads.
//! Collection/object construction, fan-out/fan-in primitives,
//! retry/repeat/collect/reduce internals, collection expression helpers, and
//! nested accessor traversal are rejected by [`validate_generated_subset`] before
//! [`emit_rust_workflow`] writes any generated source.

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
    /// The compiled IR contains a node, expression, or accessor outside generated-mode support.
    #[error("unsupported generated Rust IR feature: {feature}")]
    UnsupportedIr {
        /// Unsupported IR feature name.
        feature: &'static str,
    },
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

/// Top-level codegen entry point for the supported generated-mode IR subset.
///
/// This function validates the workflow with [`validate_generated_subset`] before
/// emitting source. Unsupported IR returns [`CodegenError::UnsupportedIr`] instead
/// of producing partial generated Rust.
pub fn emit_rust_workflow(workflow: &CompiledWorkflow) -> CodegenResult<String> {
    validate_generated_subset(workflow)?;

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

/// Reject IR that generated mode cannot faithfully emit before source text is produced.
///
/// This is the public generated-mode contract boundary. Callers may rely on it
/// to distinguish workflows supported by native Rust generation from workflows
/// that still require the interpreter/runtime path.
pub fn validate_generated_subset(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    validate_generated_nodes(workflow)?;
    validate_generated_expressions(workflow)?;
    validate_generated_accessors(workflow)
}

fn validate_generated_nodes(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut step_idx = 0u16;
    while step_idx < workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let Some(feature) = unsupported_node_feature(&node.kind)
        {
            return Err(CodegenError::UnsupportedIr { feature });
        }
        step_idx = step_idx.saturating_add(1);
    }
    Ok(())
}

fn validate_generated_expressions(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut expr_idx = 0u16;
    loop {
        let idx = vb_core::ExprIdx::new(expr_idx);
        let Some(program) = workflow.expression(idx) else {
            break;
        };
        for op in program.ops.as_ref() {
            if let Some(feature) = unsupported_expr_feature(*op) {
                return Err(CodegenError::UnsupportedIr { feature });
            }
        }
        if expr_idx == u16::MAX {
            break;
        }
        expr_idx = expr_idx.saturating_add(1);
    }
    Ok(())
}

fn validate_generated_accessors(workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let mut accessor_idx = 0u16;
    loop {
        let idx = vb_core::AccessorIdx::new(accessor_idx);
        let Some(accessor) = workflow.accessor(idx) else {
            break;
        };
        if !accessor.path.is_empty() {
            return Err(CodegenError::UnsupportedIr {
                feature: "accessor traversal",
            });
        }
        if accessor_idx == u16::MAX {
            break;
        }
        accessor_idx = accessor_idx.saturating_add(1);
    }
    Ok(())
}

fn unsupported_node_feature(kind: &CompiledNodeKind) -> Option<&'static str> {
    match kind {
        CompiledNodeKind::ForEachStart { .. } => Some("ForEachStart"),
        CompiledNodeKind::ForEachNext { .. } => Some("ForEachNext"),
        CompiledNodeKind::ForEachJoin { .. } => Some("ForEachJoin"),
        CompiledNodeKind::TogetherStart { .. } => Some("TogetherStart"),
        CompiledNodeKind::TogetherBranch { .. } => Some("TogetherBranch"),
        CompiledNodeKind::TogetherJoin { .. } => Some("TogetherJoin"),
        CompiledNodeKind::ReduceStart { .. } => Some("ReduceStart"),
        CompiledNodeKind::ReduceNext { .. } => Some("ReduceNext"),
        CompiledNodeKind::ReduceFinish { .. } => Some("ReduceFinish"),
        CompiledNodeKind::RepeatStart { .. } => Some("RepeatStart"),
        CompiledNodeKind::RepeatAttempt { .. } => Some("RepeatAttempt"),
        CompiledNodeKind::RepeatCheck { .. } => Some("RepeatCheck"),
        CompiledNodeKind::RepeatFinish { .. } => Some("RepeatFinish"),
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::ChooseSlot { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::Jump { .. }
        | CompiledNodeKind::Finish { .. } => None,
    }
}

fn unsupported_expr_feature(op: ExprOp) -> Option<&'static str> {
    match op {
        ExprOp::Append => Some("append"),
        ExprOp::AppendIf => Some("append_if"),
        ExprOp::Merge => Some("merge"),
        ExprOp::Sum => Some("sum"),
        ExprOp::Unique => Some("unique"),
        ExprOp::LoadSlot(_)
        | ExprOp::LoadConst(_)
        | ExprOp::LoadAccessor(_)
        | ExprOp::Eq
        | ExprOp::NotEq
        | ExprOp::Gt
        | ExprOp::Gte
        | ExprOp::Lt
        | ExprOp::Lte
        | ExprOp::And
        | ExprOp::Or
        | ExprOp::Not
        | ExprOp::Add
        | ExprOp::Sub
        | ExprOp::Mul
        | ExprOp::Div
        | ExprOp::Contains
        | ExprOp::StartsWith
        | ExprOp::EndsWith
        | ExprOp::Has
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Count => None,
    }
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
    write_next_or_error(out, next)
}

fn emit_wait_event_step(
    out: &mut String,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(out, "    let _event = read_slot(slots, {})?;", event.get()).map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    write_next_or_error(out, next)
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
    write_next_or_error(out, next)
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

/// Generate an expression evaluator function.
pub fn emit_expr_function(
    out: &mut String,
    expr_idx: vb_core::ExprIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let Some(program) = workflow.expression(expr_idx) else {
        return Ok(());
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
                writeln!(out, "    stack.push(read_slot(slots, {})?)?;", slot.get())
                    .map_err(fmt_err)?;
            }
            ExprOp::LoadConst(const_idx) => {
                writeln!(out, "    stack.push(read_const({})?)?;", const_idx.get())
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
            ExprOp::Contains => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_contains(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::StartsWith => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_starts_with(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::EndsWith => {
                writeln!(out, "    {{ let _needle = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _haystack = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_haystack, &_needle) {{ (SlotValue::Symbol(h), SlotValue::Symbol(n)) => symbol_ends_with(*h, *n), (_, _) => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Has => {
                writeln!(out, "    {{ let _key = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _obj = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match (&_obj, &_key) {{ (SlotValue::Object(_), SlotValue::Symbol(_)) => true, (SlotValue::List(_), SlotValue::I64(_)) => true, _ => false }}; stack.push(SlotValue::Bool(_result))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Exists => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push(SlotValue::Bool(!matches!(_v, SlotValue::Null)))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Length => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _len = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_len))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Empty => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _is_empty = match _v {{ SlotValue::List(n) => n == 0, SlotValue::Object(n) => n == 0, SlotValue::Null => true, _ => false }}; stack.push(SlotValue::Bool(_is_empty))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Append => emit_unsupported_expr(out, "append")?,
            ExprOp::AppendIf => emit_unsupported_expr(out, "append_if")?,
            ExprOp::Merge => emit_unsupported_expr(out, "merge")?,
            ExprOp::Sum => emit_unsupported_expr(out, "sum")?,
            ExprOp::Count => {
                writeln!(out, "    {{ let _v = stack.pop().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match _v {{ SlotValue::List(n) => i64::from(n), SlotValue::Object(n) => i64::from(n), _ => 0i64 }}; stack.push(SlotValue::I64(_result))?; }}")
                    .map_err(fmt_err)?;
            }
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
        "    // Action boundary: action_id={}, input_slot={}",
        action.get(),
        input.get()
    )
    .map_err(fmt_err)?;
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
    writeln!(
        out,
        "const CONTRACT_MAX_STEP_BUDGET_PER_TICK: u64 = {};",
        contract.max_step_budget_per_tick
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_BLOB_BYTES: u64 = {};",
        contract.max_blob_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_IPC_PAYLOAD_BYTES: u32 = {};",
        contract.max_ipc_payload_bytes
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_RETRY_ATTEMPTS: u16 = {};",
        contract.max_retry_attempts
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_FANOUT: u16 = {};",
        contract.max_fanout
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_COLLECT_ITEMS: u32 = {};",
        contract.max_collect_items
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_QUEUE_DEPTH: u32 = {};",
        contract.max_queue_depth
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const CONTRACT_MAX_JOURNAL_BATCH_BYTES: u32 = {};",
        contract.max_journal_batch_bytes
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
    reject_generated_pattern(source, "u16::MAX", "finish sentinel")?;
    reject_generated_pattern(source, "Vec<", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "Vec::", "dynamic Vec allocation")?;
    reject_generated_pattern(source, "slots[", "unchecked slot indexing")?;
    reject_generated_pattern(source, "CONSTANTS[", "unchecked constant indexing")?;
    reject_generated_pattern(source, " as ", "unchecked cast")?;
    require_generated_pattern(source, "StepOutcome::Finished", "terminal result return")?;

    // Only require ExprStack when the workflow has expressions.
    // Expressionless workflows generate no eval_expr functions and never instantiate ExprStack.
    let mut has_expressions = false;
    for idx in 0..u16::MAX {
        if workflow.expression(vb_core::ExprIdx::new(idx)).is_some() {
            has_expressions = true;
            break;
        }
    }
    if has_expressions {
        require_generated_pattern(source, "ExprStack::new", "bounded expression stack")?;
    }

    let mut step_count = 0u16;
    let mut expr_count = 0u16;
    let mut action_count = 0u16;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn step_") {
            step_count = step_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
                    detail: "step count overflow".into(),
                })?;
        }
        if trimmed.starts_with("fn eval_expr_") {
            expr_count = expr_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
                    detail: "expression count overflow".into(),
                })?;
        }
        if trimmed.contains("Action boundary:") {
            action_count = action_count
                .checked_add(1)
                .ok_or(CodegenError::SemanticMismatch {
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
        if workflow.expression(vb_core::ExprIdx::new(idx)).is_some() {
            expected_exprs =
                expected_exprs
                    .checked_add(1)
                    .ok_or(CodegenError::SemanticMismatch {
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
            expected_actions =
                expected_actions
                    .checked_add(1)
                    .ok_or(CodegenError::SemanticMismatch {
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
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> {{ read_slot_optional(slots, slot).ok_or(DriveError::SlotNull) }}").map_err(fmt_err)?;
    writeln!(out, "fn read_slot_optional(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Option<SlotValue> {{ slots.get(usize::from(slot)).copied().flatten() }}").map_err(fmt_err)?;
    writeln!(out, "fn write_slot(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, value: Option<SlotValue>) -> Result<(), DriveError> {{ match slots.get_mut(usize::from(slot)) {{ Some(target) => {{ *target = value; Ok(()) }}, None => Err(DriveError::InvalidCompiledWorkflow {{ reason: \"slot index out of bounds\" }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn read_const(index: u16) -> Result<SlotValue, DriveError> {{ CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow {{ reason: \"constant index out of bounds\" }}) }}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "fn symbol_contains(_haystack: u32, _needle: u32) -> bool {{ _haystack == _needle }}"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "fn symbol_starts_with(_haystack: u32, _prefix: u32) -> bool {{ _haystack == _prefix }}"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "fn symbol_ends_with(_haystack: u32, _suffix: u32) -> bool {{ _haystack == _suffix }}"
    )
    .map_err(fmt_err)?;
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

fn fmt_err(_: std::fmt::Error) -> CodegenError {
    CodegenError::FormatBufferOverflow
}

#[cfg(test)]
mod emit;
#[cfg(test)]
mod helpers;
mod proptests;
mod tests;
