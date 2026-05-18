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
//! The current subset accepts scalar constants, slot copies, generated list
//! payloads, bounded for-each loops, expression math and boolean comparisons,
//! action dispatch, waits, asks, jumps, choices, handlers, expression helpers,
//! and finish nodes. Accessors are emitted as bounded checked root/field/list
//! traversal. Fan-in/fan-out families beyond ForEach and collect/reduce/repeat
//! internals are rejected by [`validate_generated_subset`] before
//! [`emit_rust_workflow`] writes any generated source.

use std::fmt::Write;
use std::process::Command;
use thiserror::Error;
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprBranch,
    ExprOp, ResourceContract, SlotBranch, SlotIdx, StepIdx,
};

#[cfg(kani)]
mod kani_generated_runtime;

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

const ACCESSOR_MAX_PATH_DEPTH: u16 = 16;

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
    emit_value_store_contract(&mut out, workflow)?;
    emit_journal_contract(&mut out, workflow)?;
    emit_constants(&mut out, workflow)?;
    emit_drive_function(&mut out, workflow)?;
    emit_generated_runtime_api(&mut out, workflow)?;
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
        if accessor.root.get() >= workflow.slot_count() {
            return Err(CodegenError::UnsupportedIr {
                feature: "accessor root slot out of bounds",
            });
        }
        if accessor.path.len() > usize::from(ACCESSOR_MAX_PATH_DEPTH) {
            return Err(CodegenError::UnsupportedIr {
                feature: "accessor path too deep",
            });
        }
        for segment in accessor.path.as_ref() {
            if let vb_core::PathSegment::Field(symbol) = segment
                && symbol.get() >= workflow.symbols_count()
            {
                return Err(CodegenError::UnsupportedIr {
                    feature: "accessor field symbol out of bounds",
                });
            }
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
        | CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::Jump { .. }
        | CompiledNodeKind::Finish { .. } => None,
        _ => Some("future compiled node kind"),
    }
}

fn unsupported_expr_feature(op: ExprOp) -> Option<&'static str> {
    match op {
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
        | ExprOp::Has
        | ExprOp::Exists
        | ExprOp::Append
        | ExprOp::AppendIf
        | ExprOp::Merge
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => None,
        ExprOp::Contains => Some("text helper contains requires runtime symbol store"),
        ExprOp::StartsWith => Some("text helper starts_with requires runtime symbol store"),
        ExprOp::EndsWith => Some("text helper ends_with requires runtime symbol store"),
        ExprOp::Length => None,
        ExprOp::Empty => None,
        _ => Some("future expression op"),
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
    writeln!(
        out,
        "const WORKFLOW_NODE_COUNT_USIZE: usize = {};",
        workflow.node_count()
    )
    .map_err(fmt_err)?;
    for symbol in 0..workflow.symbols_count() {
        writeln!(out, "const _sym_{symbol}: u32 = {symbol};").map_err(fmt_err)?;
    }
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
    writeln!(
        out,
        "    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let mut pc: u16 = {};", workflow.entry().get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    let mut step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let mut list_store = ListStore::new();").map_err(fmt_err)?;
    writeln!(out, "    let mut object_store = ObjectStore::new();").map_err(fmt_err)?;
    writeln!(
        out,
        "    let mut collect_states = CollectStateStore::new();"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    loop {{").map_err(fmt_err)?;
    writeln!(out, "        if step_budget_remaining == 0 {{").map_err(fmt_err)?;
    writeln!(
        out,
        "            return Err(DriveError::StepBudgetExhausted);"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        }}").map_err(fmt_err)?;
    writeln!(out, "        step_budget_remaining = step_budget_remaining.checked_sub(1).ok_or(DriveError::StepBudgetExhausted)?;").map_err(fmt_err)?;
    writeln!(out, "        let outcome = match pc {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        writeln!(
            out,
            "            {step_idx} => step_{step_idx}(&mut slots, &mut slot_taints, &mut list_store, &mut object_store, &mut collect_states)?,"
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

fn emit_journal_contract(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    let event_capacity = checked_metric_add(
        checked_metric_mul(
            usize::from(workflow.node_count()).max(1),
            4,
            "journal capacity overflow",
        )?,
        checked_metric_mul(
            usize::from(workflow.slot_count().max(1)),
            2,
            "journal capacity overflow",
        )?,
        "journal capacity overflow",
    )?;
    writeln!(out, "// --- Generated journal contract ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const GENERATED_JOURNAL_CAPACITY: usize = {event_capacity};"
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn emit_generated_runtime_api(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "// --- Rich generated runtime API ---").map_err(fmt_err)?;
    writeln!(out, "pub fn drive_with_journal(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<GeneratedRunStatus, DriveError> {{ let mut state = GeneratedRunState::new(slots); state.run_until_blocked() }}").map_err(fmt_err)?;
    writeln!(out, "impl GeneratedRunState {{").map_err(fmt_err)?;
    writeln!(out, "    pub fn new(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Self {{ Self::new_with_run_id(0, slots, [Taint::Clean; WORKFLOW_SLOT_COUNT]) }}").map_err(fmt_err)?;
    writeln!(out, "    pub fn new_with_taints(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: [Taint; WORKFLOW_SLOT_COUNT]) -> Self {{ Self::new_with_run_id(0, slots, slot_taints) }}").map_err(fmt_err)?;
    writeln!(out, "    pub fn new_with_run_id(run_id: u64, slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: [Taint; WORKFLOW_SLOT_COUNT]) -> Self {{ let mut collect_states = CollectStateStore::new(); collect_states.set_run_id(run_id); Self {{ run_id, slots, slot_taints, step_states: [StepState::Pending; WORKFLOW_NODE_COUNT_USIZE], pc: {}, step_budget_remaining: CONTRACT_MAX_STEP_BUDGET_PER_TICK, list_store: ListStore::new(), object_store: ObjectStore::new(), collect_states, journal: Journal::new(), pending: None }} }}", workflow.entry().get()).map_err(fmt_err)?;
    emit_run_until_blocked(out, workflow)?;
    emit_action_resume_api(out)?;
    emit_ask_resume_api(out)?;
    writeln!(out, "}}\n").map_err(fmt_err)?;
    emit_action_completion_spec(out, workflow)?;
    emit_ask_answer_spec(out, workflow)?;
    emit_step_output_slot(out, workflow)?;
    emit_journal_observed_slot(out, workflow)?;
    emit_finish_result_slot(out, workflow)?;
    Ok(())
}

fn emit_run_until_blocked(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(
        out,
        "    pub fn run_until_blocked(&mut self) -> Result<GeneratedRunStatus, DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        if let Some(pending) = self.pending {{ return Err(DriveError::InvalidResume {{ step: pending.step() }}); }}").map_err(fmt_err)?;
    writeln!(out, "        loop {{").map_err(fmt_err)?;
    writeln!(out, "            if self.step_budget_remaining == 0 {{ return Err(DriveError::StepBudgetExhausted); }}").map_err(fmt_err)?;
    writeln!(
        out,
        "            self.journal.ensure_capacity(WORKFLOW_SLOT_COUNT)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "            self.step_budget_remaining = self.step_budget_remaining.checked_sub(1).ok_or(DriveError::StepBudgetExhausted)?;").map_err(fmt_err)?;
    writeln!(out, "            let before_slots = self.slots;").map_err(fmt_err)?;
    writeln!(out, "            let before_taints = self.slot_taints;").map_err(fmt_err)?;
    writeln!(out, "            let current_pc = self.pc;").map_err(fmt_err)?;
    writeln!(out, "            self.journal.ensure_capacity(1)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "            self.journal.push(JournalEvent::StepStarted {{ step: current_pc }})?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "            set_step_state(&mut self.step_states, current_pc, StepState::Running)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "            let outcome = match current_pc {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        writeln!(out, "                {step_idx} => step_{step_idx}(&mut self.slots, &mut self.slot_taints, &mut self.list_store, &mut self.object_store, &mut self.collect_states),").map_err(fmt_err)?;
    }
    writeln!(
        out,
        "                _ => Err(DriveError::InvalidProgramCounter),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "            }};").map_err(fmt_err)?;
    writeln!(out, "            match outcome {{").map_err(fmt_err)?;
    writeln!(
        out,
        "                Ok(StepOutcome::Continue(next)) => {{ self.record_slot_changes(current_pc, &before_slots, &before_taints)?; set_step_state(&mut self.step_states, current_pc, StepState::Succeeded)?; self.journal.ensure_capacity(1)?; self.journal.push(JournalEvent::StepSucceeded {{ step: current_pc, output: output_slot_for_step(current_pc)? }})?; self.pc = next; }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "                Ok(StepOutcome::Finished(value)) => {{ self.record_slot_changes(current_pc, &before_slots, &before_taints)?; set_step_state(&mut self.step_states, current_pc, StepState::Succeeded)?; let taint = read_taint(&self.slot_taints, finish_result_slot(current_pc)?)?; self.journal.ensure_capacity(2)?; self.journal.push(JournalEvent::StepSucceeded {{ step: current_pc, output: output_slot_for_step(current_pc)? }})?; self.journal.push(JournalEvent::RunFinished {{ step: current_pc, value, taint }})?; return Ok(GeneratedRunStatus::Finished(DriveOutput {{ value, taint, journal: self.journal }})); }}").map_err(fmt_err)?;
    writeln!(
        out,
        "                Err(error) => return self.suspend_from_error(error),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "            }}").map_err(fmt_err)?;
    writeln!(out, "        }}").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)
}

fn emit_action_resume_api(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "    pub fn complete_action(&mut self, step: u16, action_id: u16, output_slot: u16, value: SlotValue, taint: Taint) -> Result<GeneratedRunStatus, DriveError> {{").map_err(fmt_err)?;
    writeln!(
        out,
        "        let next = action_completion_next(step, action_id, output_slot)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        match self.pending {{ Some(PendingResume::Action {{ step: pending_step, action_id: pending_action_id, resume_pc }}) if pending_step == step && pending_action_id == action_id && resume_pc == next => {{}}, _ => return Err(DriveError::InvalidResume {{ step }}), }}").map_err(fmt_err)?;
    writeln!(out, "        self.journal.ensure_capacity(2)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "        self.write_slot_with_journal(output_slot, Some(value), taint)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        self.journal.push(JournalEvent::ActionCompleted {{ step, action_id, output_slot, value, taint }})?;").map_err(fmt_err)?;
    writeln!(out, "        self.pending = None;").map_err(fmt_err)?;
    writeln!(out, "        self.pc = next;").map_err(fmt_err)?;
    writeln!(out, "        self.run_until_blocked()").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)
}

fn emit_ask_resume_api(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "    pub fn answer_ask(&mut self, ask_step: u16, resume_step: u16, value: SlotValue, taint: Taint) -> Result<GeneratedRunStatus, DriveError> {{").map_err(fmt_err)?;
    writeln!(
        out,
        "        let (answer_slot, next) = ask_answer_spec(ask_step, resume_step)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        match self.pending {{ Some(PendingResume::Ask {{ ask_step: pending_ask_step, resume_pc }}) if pending_ask_step == ask_step && resume_pc == resume_step => {{}}, _ => return Err(DriveError::InvalidResume {{ step: ask_step }}), }}").map_err(fmt_err)?;
    writeln!(out, "        self.journal.ensure_capacity(2)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "        self.write_slot_with_journal(answer_slot, Some(value), taint)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        self.journal.push(JournalEvent::AskAnswered {{ ask_step, resume_step, answer_slot, value, taint }})?;").map_err(fmt_err)?;
    writeln!(out, "        self.pending = None;").map_err(fmt_err)?;
    writeln!(out, "        self.pc = next;").map_err(fmt_err)?;
    writeln!(out, "        self.run_until_blocked()").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)
}

fn emit_action_completion_spec(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(out, "fn action_completion_next(step: u16, action_id: u16, output_slot: u16) -> Result<u16, DriveError> {{").map_err(fmt_err)?;
    writeln!(out, "    match (step, action_id, output_slot) {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let CompiledNodeKind::Do { action, .. } = node.kind
            && let (Some(output), Some(next)) = (node.output, node.next)
        {
            writeln!(
                out,
                "        ({}, {}, {}) => Ok({}),",
                step.get(),
                action.get(),
                output.get(),
                next.get()
            )
            .map_err(fmt_err)?;
        }
    }
    writeln!(
        out,
        "        (step, _, _) => Err(DriveError::InvalidResume {{ step }}),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}\n").map_err(fmt_err)
}

fn emit_ask_answer_spec(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(
        out,
        "fn ask_answer_spec(ask_step: u16, resume_step: u16) -> Result<(u16, u16), DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match (ask_step, resume_step) {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let ask_step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(ask_step)
            && matches!(node.kind, CompiledNodeKind::Ask { .. })
            && let Some(resume_step) = node.next
            && let Some(resume_node) = workflow.node(resume_step)
            && let CompiledNodeKind::AskResume { answer } = resume_node.kind
            && let Some(next) = resume_node.next
        {
            writeln!(
                out,
                "        ({}, {}) => Ok(({}, {})),",
                ask_step.get(),
                resume_step.get(),
                answer.get(),
                next.get()
            )
            .map_err(fmt_err)?;
        }
    }
    writeln!(
        out,
        "        (step, _) => Err(DriveError::InvalidResume {{ step }}),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}\n").map_err(fmt_err)
}

fn emit_finish_result_slot(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(
        out,
        "fn finish_result_slot(step: u16) -> Result<u16, DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match step {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && let CompiledNodeKind::Finish { result } = node.kind
        {
            writeln!(out, "        {} => Ok({}),", step.get(), result.get()).map_err(fmt_err)?;
        }
    }
    writeln!(
        out,
        "        step => Err(DriveError::InvalidResume {{ step }}),"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}\n").map_err(fmt_err)
}

fn emit_step_output_slot(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(
        out,
        "fn output_slot_for_step(step: u16) -> Result<Option<u16>, DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match step {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step) {
            match node.output {
                Some(output) => {
                    writeln!(out, "        {} => Ok(Some({})),", step.get(), output.get())
                        .map_err(fmt_err)?
                }
                None => writeln!(out, "        {} => Ok(None),", step.get()).map_err(fmt_err)?,
            }
        }
    }
    writeln!(out, "        _ => Err(DriveError::InvalidProgramCounter),").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}\n").map_err(fmt_err)
}

fn emit_journal_observed_slot(out: &mut String, workflow: &CompiledWorkflow) -> CodegenResult<()> {
    writeln!(
        out,
        "fn journal_observed_slot(step: u16) -> Result<Option<u16>, DriveError> {{"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    match step {{").map_err(fmt_err)?;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step) {
            let observed = match &node.kind {
                CompiledNodeKind::CollectStart { source, .. } => node.output.or(Some(*source)),
                CompiledNodeKind::CollectNext { collector_slot, .. }
                | CompiledNodeKind::CollectFinish { collector_slot } => Some(*collector_slot),
                _ => node.output,
            };
            match observed {
                Some(slot) => writeln!(out, "        {} => Ok(Some({})),", step.get(), slot.get())
                    .map_err(fmt_err)?,
                None => writeln!(out, "        {} => Ok(None),", step.get()).map_err(fmt_err)?,
            }
        }
    }
    writeln!(out, "        _ => Err(DriveError::InvalidProgramCounter),").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(out, "}}\n").map_err(fmt_err)
}

/// Generate a per-step function for one compiled node.
pub fn emit_step_function(
    out: &mut String,
    node: &CompiledNode,
    _workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let step_id = node.id.get();
    let slots_param = step_slots_param(node);
    let slot_taints_param = step_slot_taints_param(node);
    let list_store_param = step_list_store_param(node);
    let object_store_param = step_object_store_param(node);
    let collect_states_param = step_collect_states_param(node);
    writeln!(
        out,
        "fn step_{step_id}({slots_param}: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], {slot_taints_param}: &mut [Taint; WORKFLOW_SLOT_COUNT], {list_store_param}: &mut ListStore, {object_store_param}: &mut ObjectStore, {collect_states_param}: &mut CollectStateStore) -> Result<StepOutcome, DriveError> {{"
    )
    .map_err(fmt_err)?;

    emit_step_body(out, node)?;

    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn step_slots_param(node: &CompiledNode) -> &'static str {
    match &node.kind {
        CompiledNodeKind::Nop | CompiledNodeKind::Jump { .. } => "_slots",
        _ => "slots",
    }
}

fn step_slot_taints_param(node: &CompiledNode) -> &'static str {
    match &node.kind {
        CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
            if node.output.is_some() =>
        {
            "slot_taints"
        }
        CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ErrorHandler { .. } => "slot_taints",
        _ => "_slot_taints",
    }
}

fn step_list_store_param(node: &CompiledNode) -> &'static str {
    match &node.kind {
        CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::ErrorHandler { .. } => "list_store",
        _ => "_list_store",
    }
}

fn step_object_store_param(node: &CompiledNode) -> &'static str {
    match &node.kind {
        CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::ErrorHandler { .. } => "object_store",
        _ => "_object_store",
    }
}

fn step_collect_states_param(node: &CompiledNode) -> &'static str {
    match &node.kind {
        CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. } => "collect_states",
        _ => "_collect_states",
    }
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
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. } => emit_for_each_step_body(out, node),
        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => emit_together_step_body(out, node),
        CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. } => emit_reduce_step_body(out, node),
        CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. } => emit_repeat_step_body(out, node),
        CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. } => emit_collect_step_body(out, node),
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ErrorHandler { .. } => emit_boundary_step_body(out, node),
        CompiledNodeKind::RetryCheck { .. } => emit_retry_check_step_body(out, &node.kind),
        _ => emit_unsupported_step(out, "UnsupportedStep"),
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
        CompiledNodeKind::Do { action, input } => {
            emit_action_boundary(out, node.id, *action, *input, node.next)
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            emit_wait_until_step(out, node.id, *deadline_slot, node.next)
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => emit_wait_event_step(out, node.id, *event, *timeout_slot, node.next),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => emit_ask_step(out, node.id, *prompt, *timeout_slot, node.next),
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
            "    // write_slot(slots, {}, Some(read_const({})?))",
            output_slot.get(),
            value.get()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    write_slot_with_taint(slots, slot_taints, {}, Some(read_const({})?), Taint::Clean)?;",
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
            "    let copied = read_slot_optional(slots, {})?;\n    let copied_taint = read_taint(slot_taints, {})?;\n    write_slot_with_taint(slots, slot_taints, {}, copied, copied_taint)?;",
            source.get(),
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
            "    let (_expr_value, _expr_taint) = eval_expr_{}(slots, slot_taints, list_store, object_store)?;\n    write_slot_with_taint(slots, slot_taints, {}, Some(_expr_value), _expr_taint)?;",
            expr.get(),
            output_slot.get()
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
            "    let (_condition, _) = eval_expr_{}(slots, slot_taints, list_store, object_store)?;\n    match _condition {{ SlotValue::Bool(true) => return Ok(StepOutcome::Continue({})), SlotValue::Bool(false) => {{}}, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}), }}",
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
            "    let _condition = read_slot(slots, {})?;\n    match _condition {{ SlotValue::Bool(true) => return Ok(StepOutcome::Continue({})), SlotValue::Bool(false) => {{}}, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}), }}",
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
    step: StepIdx,
    deadline_slot: SlotIdx,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _deadline = read_slot(slots, {})?;",
        deadline_slot.get()
    )
    .map_err(fmt_err)?;
    emit_wait_until_suspend(out, step, deadline_slot, next)
}

fn emit_wait_event_step(
    out: &mut String,
    step: StepIdx,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    writeln!(out, "    let _event = read_slot(slots, {})?;", event.get()).map_err(fmt_err)?;
    emit_optional_timeout_read(out, timeout_slot)?;
    emit_wait_event_suspend(out, step, event, timeout_slot, next)
}

fn emit_ask_step(
    out: &mut String,
    step: StepIdx,
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
    emit_ask_suspend(out, step, prompt, timeout_slot, next)
}

fn emit_wait_until_suspend(
    out: &mut String,
    step: StepIdx,
    deadline_slot: SlotIdx,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    match next {
        Some(resume_pc) => writeln!(out, "    Err(SuspensionOutcome::WaitUntil {{ step: {}, deadline_slot: {}, resume_pc: {} }}.into_drive_error())", step.get(), deadline_slot.get(), resume_pc.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

fn emit_wait_event_suspend(
    out: &mut String,
    step: StepIdx,
    event_slot: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    match next {
        Some(resume_pc) => writeln!(out, "    Err(SuspensionOutcome::WaitEvent {{ step: {}, event_slot: {}, timeout_slot: {}, resume_pc: {} }}.into_drive_error())", step.get(), event_slot.get(), optional_slot_literal(timeout_slot), resume_pc.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

fn emit_ask_suspend(
    out: &mut String,
    step: StepIdx,
    prompt_slot: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    match next {
        Some(resume_pc) => writeln!(out, "    Err(SuspensionOutcome::AskPending {{ step: {}, prompt_slot: {}, timeout_slot: {}, resume_pc: {} }}.into_drive_error())", step.get(), prompt_slot.get(), optional_slot_literal(timeout_slot), resume_pc.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

fn optional_slot_literal(slot: Option<SlotIdx>) -> String {
    match slot {
        Some(slot) => format!("Some({})", slot.get()),
        None => "None".into(),
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
    writeln!(out, "    // step_{}(slots, list_store)", body.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    match step_{}(slots, slot_taints, list_store, object_store, _collect_states) {{",
        body.get()
    )
    .map_err(fmt_err)?;
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
            "    let _f{} = ObjectField {{ key: _sym_{}, value: read_slot(slots, {})?, taint: read_taint(slot_taints, {})? }};",
            i,
            sym.get(),
            slot.get(),
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if let Some(output_slot) = output {
        let joined_fields = object_field_bindings(fields.len());
        let joined_taints = object_field_taint_bindings(fields.len());
        writeln!(
            out,
            "    let _object_taint = join_taints(&[{joined_taints}]);"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    let _object_handle = object_store.insert_fields(&[{joined_fields}])?;"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    // write_slot(slots, {}, Some(SlotValue::Object(_object_handle)))",
            output_slot.get(),
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::Object(_object_handle)), _object_taint)?;",
            output_slot.get(),
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
            "    let _item{} = read_slot(slots, {})?;\n    let _item{}_taint = read_taint(slot_taints, {})?;",
            i,
            slot.get(),
            i,
            slot.get()
        )
        .map_err(fmt_err)?;
    }
    if let Some(output_slot) = output {
        let joined_items = list_item_bindings(items.len());
        let joined_taints = list_taint_bindings(items.len());
        writeln!(
            out,
            "    let _list_taint = join_taints(&[{joined_taints}]);"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    let _list_handle = list_store.insert_items_with_taints(&[{joined_items}], &[{joined_taints}])?;"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    // write_slot(slots, {}, Some(SlotValue::List(_list_handle)))",
            output_slot.get(),
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_list_handle)), _list_taint)?;",
            output_slot.get(),
        )
        .map_err(fmt_err)?;
    }
    write_next_or_error(out, next)
}

fn object_field_bindings(len: usize) -> String {
    (0..len)
        .map(|index| format!("_f{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn object_field_taint_bindings(len: usize) -> String {
    (0..len)
        .map(|index| format!("_f{index}.taint"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn list_item_bindings(len: usize) -> String {
    (0..len)
        .map(|index| format!("_item{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn list_taint_bindings(len: usize) -> String {
    (0..len)
        .map(|index| format!("_item{index}_taint"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_for_each_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => emit_for_each_start_step(
            out,
            ForEachStartEmit {
                step: node.id,
                input: *input,
                item_slot: *item_slot,
                limit: *limit,
                body: *body,
                done: *done,
                output: node.output,
            },
        ),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => emit_for_each_next_step(out, node.id, *iterator_slot, *body, *done, node.output),
        CompiledNodeKind::ForEachJoin { output } => {
            emit_for_each_join_step(out, node.id, *output, node.output, node.next)
        }
        _ => emit_unsupported_step(out, "UnsupportedStep"),
    }
}

#[derive(Clone, Copy)]
struct ForEachStartEmit {
    step: StepIdx,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
}

fn emit_for_each_start_step(out: &mut String, spec: ForEachStartEmit) -> CodegenResult<()> {
    let Some(iterator_slot) = spec.output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            spec.step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _input = read_slot(slots, {})?;",
        spec.input.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _list = expect_list_value(_input)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    if _item_count > {} {{ return Err(DriveError::IterationLimitExceeded {{ resource: \"for_each_limit\" }}); }}",
        spec.limit
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if _item_count == 0 {{").map_err(fmt_err)?;
    writeln!(
        out,
        "        let _tail = tail_list_handle(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "        write_slot(slots, {}, Some(SlotValue::List(_tail)))?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "        return Ok(StepOutcome::Continue({}));",
        spec.done.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _first = first_list_item(list_store, _list, _item_count)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot(slots, {}, Some(_first))?;",
        spec.item_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _tail = tail_list_handle(list_store, _list)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot(slots, {}, Some(SlotValue::List(_tail)))?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    emit_continue_step(out, spec.body)
}

fn emit_for_each_next_step(
    out: &mut String,
    step: StepIdx,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    let Some(item_output) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _iterator = read_slot(slots, {})?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _list = expect_list_value(_iterator)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if _item_count == 0 {{").map_err(fmt_err)?;
    writeln!(
        out,
        "        return Ok(StepOutcome::Continue({}));",
        done.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _first = first_list_item(list_store, _list, _item_count)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot(slots, {}, Some(_first))?;",
        item_output.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _tail = tail_list_handle(list_store, _list)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot(slots, {}, Some(SlotValue::List(_tail)))?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    emit_continue_step(out, body)
}

fn emit_for_each_join_step(
    out: &mut String,
    step: StepIdx,
    materialized: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    let Some(output_slot) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _materialized = read_slot(slots, {})?;",
        materialized.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _list = expect_list_value(_materialized)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot(slots, {}, Some(_materialized))?;",
        output_slot.get()
    )
    .map_err(fmt_err)?;
    write_next_or_error(out, next)
}

fn emit_together_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::TogetherStart { branches, .. } => {
            emit_together_start_step(out, node.id, branches, node.output)
        }
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            accumulator,
            ..
        } => emit_together_branch_step(out, *branch, *entry, *accumulator, node.output),
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => emit_together_join_step(out, *branch_count, *accumulator, node.output, node.next),
        _ => emit_unsupported_step(out, "Together"),
    }
}

fn emit_together_start_step(
    out: &mut String,
    step: StepIdx,
    branches: &[StepIdx],
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    let Some(first_branch) = branches.first().copied() else {
        return writeln!(out, "    Err(DriveError::InvalidCompiledWorkflow {{ reason: \"together_start requires at least one branch\" }})").map_err(fmt_err);
    };
    let Some(output_slot) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _accumulator = list_store.insert_items_with_taints(&[], &[])?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_accumulator)), Taint::Clean)?;", output_slot.get()).map_err(fmt_err)?;
    emit_continue_step(out, first_branch)
}

fn emit_together_branch_step(
    out: &mut String,
    branch: u16,
    entry: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    if branch != 0
        && let Some(output_slot) = output
    {
        writeln!(
            out,
            "    let _branch_value = read_slot(slots, {})?;",
            output_slot.get()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    let _branch_taint = read_taint(slot_taints, {})?;",
            output_slot.get()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    let _accumulator = expect_list_value(read_slot(slots, {})?)?;",
            accumulator.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "    let _updated = append_list_item(list_store, _accumulator, _branch_value, _branch_taint)?;").map_err(fmt_err)?;
        writeln!(
            out,
            "    let _accumulator_taint = join_taint(read_taint(slot_taints, {})?, _branch_taint);",
            accumulator.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_updated)), _accumulator_taint)?;", accumulator.get()).map_err(fmt_err)?;
    }
    emit_continue_step(out, entry)
}

fn emit_together_join_step(
    out: &mut String,
    _branch_count: u16,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    let _accumulator_handle = expect_list_value(read_slot(slots, {})?)?;",
            accumulator.get()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    let _last_result = read_slot(slots, {})?;",
            output_slot.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "    let _final_value = match _last_result {{ SlotValue::List(_) | SlotValue::Null => SlotValue::List(_accumulator_handle), other => {{ let _last_taint = read_taint(slot_taints, {})?; let _updated = append_list_item(list_store, _accumulator_handle, other, _last_taint)?; write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_updated)), join_taint(read_taint(slot_taints, {})?, _last_taint))?; SlotValue::List(_updated) }} }};", output_slot.get(), accumulator.get(), accumulator.get()).map_err(fmt_err)?;
        writeln!(out, "    let _joined_taint = join_taint(read_taint(slot_taints, {})?, read_taint(slot_taints, {})?);", accumulator.get(), output_slot.get()).map_err(fmt_err)?;
        writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(_final_value), _joined_taint)?;", output_slot.get()).map_err(fmt_err)?;
    }
    write_next_or_error(out, next)
}

fn emit_reduce_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => emit_reduce_start_step(
            out,
            ReduceStartStep {
                step: node.id,
                input: *input,
                accumulator: *accumulator,
                initial: *initial,
                body: *body,
                done: *done,
                output: node.output,
            },
        ),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            body,
            done,
            ..
        } => emit_reduce_next_step(out, node.id, *iterator_slot, *body, *done, node.output),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            emit_copy_to_output_then_next(out, *accumulator, node.output, node.next, node.id)
        }
        _ => emit_unsupported_step(out, "Reduce"),
    }
}

struct ReduceStartStep {
    step: StepIdx,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
}

fn emit_reduce_start_step(out: &mut String, step: ReduceStartStep) -> CodegenResult<()> {
    let ReduceStartStep {
        step,
        input,
        accumulator,
        initial,
        body,
        done,
        output,
    } = step;
    writeln!(out, "    let _initial = read_const({})?;", initial.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot_with_taint(slots, slot_taints, {}, Some(_initial), Taint::Clean)?;",
        accumulator.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _list = expect_list_value(read_slot(slots, {})?)?;",
        input.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _list_taint = read_taint(slot_taints, {})?;",
        input.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    if _item_count == 0 {{ return Ok(StepOutcome::Continue({})); }}",
        done.get()
    )
    .map_err(fmt_err)?;
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    let _first = first_list_item(list_store, _list, _item_count)?;"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    write_slot_with_taint(slots, slot_taints, {}, Some(_first), _list_taint)?;",
            input.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "    let _tail = tail_list_handle(list_store, _list)?;").map_err(fmt_err)?;
        writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_tail)), _list_taint)?;", output_slot.get()).map_err(fmt_err)?;
        emit_continue_step(out, body)
    } else {
        writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err)
    }
}

fn emit_reduce_next_step(
    out: &mut String,
    step: StepIdx,
    iterator_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _list = expect_list_value(read_slot(slots, {})?)?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _list_taint = read_taint(slot_taints, {})?;",
        iterator_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _list)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    if _item_count == 0 {{ return Ok(StepOutcome::Continue({})); }}",
        done.get()
    )
    .map_err(fmt_err)?;
    if let Some(output_slot) = output {
        writeln!(
            out,
            "    let _first = first_list_item(list_store, _list, _item_count)?;"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    write_slot_with_taint(slots, slot_taints, {}, Some(_first), _list_taint)?;",
            output_slot.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "    let _tail = tail_list_handle(list_store, _list)?;").map_err(fmt_err)?;
        writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_tail)), _list_taint)?;", iterator_slot.get()).map_err(fmt_err)?;
        emit_continue_step(out, body)
    } else {
        writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err)
    }
}

fn emit_repeat_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts, body, ..
        } => emit_repeat_start_step(out, node.id, *max_attempts, *body, node.output),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot, body, ..
        } => emit_repeat_attempt_step(out, *attempt_slot, *body),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            emit_repeat_check_step_body(out, *attempt_slot, *done, node.next)
        }
        CompiledNodeKind::RepeatFinish { result } => {
            emit_copy_to_output_then_next(out, *result, node.output, node.next, node.id)
        }
        _ => emit_unsupported_step(out, "Repeat"),
    }
}

fn emit_repeat_start_step(
    out: &mut String,
    step: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _repeat_state = encode_repeat_state({}, 0)?;",
        max_attempts
    )
    .map_err(fmt_err)?;
    let Some(output_slot) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::I64(_repeat_state)), Taint::Clean)?;", output_slot.get()).map_err(fmt_err)?;
    emit_continue_step(out, body)
}

fn emit_repeat_attempt_step(
    out: &mut String,
    attempt_slot: SlotIdx,
    body: StepIdx,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _packed = expect_i64_value(read_slot(slots, {})?)?;",
        attempt_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _state = decode_repeat_state(_packed)?;").map_err(fmt_err)?;
    emit_continue_step(out, body)
}

fn emit_repeat_check_step_body(
    out: &mut String,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
) -> CodegenResult<()> {
    let Some(body) = next else {
        return writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _packed = expect_i64_value(read_slot(slots, {})?)?;",
        attempt_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let (_max_attempts, _current_attempt) = decode_repeat_state(_packed)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _next_attempt = match _current_attempt.checked_add(1) {{ Some(value) => value, None => _current_attempt }};").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _updated = encode_repeat_state(_max_attempts, _next_attempt)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::I64(_updated)), read_taint(slot_taints, {})?)?;", attempt_slot.get(), attempt_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    if _next_attempt >= _max_attempts {{ Ok(StepOutcome::Continue({})) }} else {{ Ok(StepOutcome::Continue({})) }}", done.get(), body.get()).map_err(fmt_err)
}

fn emit_collect_step_body(out: &mut String, node: &CompiledNode) -> CodegenResult<()> {
    match &node.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => emit_collect_start_step(out, *source, *limit, *page_size, *body, *done, node.output),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            ..
        } => {
            writeln!(
                out,
                "    let _collector = expect_list_value(read_slot(slots, {})?)?;",
                collector_slot.get()
            )
            .map_err(fmt_err)?;
            emit_continue_step(out, *body)
        }
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
            ..
        } => emit_collect_next_step(out, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            emit_collect_finish_step(out, *collector_slot, node.output, node.next, node.id)
        }
        _ => emit_unsupported_step(out, "Collect"),
    }
}

fn emit_collect_start_step(
    out: &mut String,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> CodegenResult<()> {
    let collector = output.unwrap_or(source);
    writeln!(
        out,
        "    let _source = expect_list_value(read_slot(slots, {})?)?;",
        source.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _source_taint = read_taint(slot_taints, {})?;",
        source.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _item_count = list_item_count(list_store, _source)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    if _item_count > {} {{ return Err(DriveError::CollectItemLimitExceeded); }}",
        limit
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if {} == 0 {{ return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"collect page_size must be nonzero\" }}); }}", page_size).map_err(fmt_err)?;
    writeln!(
        out,
        "    if {} > {} {{ return Err(DriveError::CollectPageLimitExceeded); }}",
        page_size, limit
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _page = collect_page_handle(list_store, _source, 0, {})?;",
        page_size
    )
    .map_err(fmt_err)?;
    writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_page)), _source_taint)?;", collector.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    let _page_len = list_item_count(list_store, _page)?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _cursor = checked_add_u32(0, _page_len, DriveError::CollectItemLimitExceeded)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if _page_len == 0 || _cursor >= _item_count {{ collect_states.remove({}); Ok(StepOutcome::Continue({})) }} else {{ collect_states.upsert({}, CollectState {{ source: _source, current_page: _page, cursor: _cursor, page_size: {}, item_count: _item_count, limit: {} }})?; Ok(StepOutcome::Continue({})) }}", collector.get(), done.get(), collector.get(), page_size, limit, body.get()).map_err(fmt_err)
}

fn emit_collect_next_step(
    out: &mut String,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> CodegenResult<()> {
    writeln!(
        out,
        "    let _current_page = expect_list_value(read_slot(slots, {})?)?;",
        collector_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _collector_taint = read_taint(slot_taints, {})?;",
        collector_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if list_item_count(list_store, _current_page)? == 0 {{ collect_states.remove({}); return Ok(StepOutcome::Continue({})); }}", collector_slot.get(), done.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    let _state = collect_states.require_current_page({}, _current_page)?;",
        collector_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    if _state.cursor >= _state.item_count {{ let _terminal = collect_page_handle(list_store, _state.source, _state.item_count, _state.page_size)?; write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_terminal)), Taint::Clean)?; collect_states.remove({}); return Ok(StepOutcome::Continue({})); }}", collector_slot.get(), collector_slot.get(), done.get()).map_err(fmt_err)?;
    writeln!(out, "    let _page = collect_page_handle(list_store, _state.source, _state.cursor, _state.page_size)?;").map_err(fmt_err)?;
    writeln!(
        out,
        "    let _page_len = list_item_count(list_store, _page)?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    let _cursor = checked_add_u32(_state.cursor, _page_len, DriveError::CollectItemLimitExceeded)?;").map_err(fmt_err)?;
    writeln!(out, "    write_slot_with_taint(slots, slot_taints, {}, Some(SlotValue::List(_page)), Taint::Clean)?;", collector_slot.get()).map_err(fmt_err)?;
    writeln!(out, "    collect_states.upsert({}, CollectState {{ current_page: _page, cursor: _cursor, .._state }})?;", collector_slot.get()).map_err(fmt_err)?;
    emit_continue_step(out, body)
}

fn emit_collect_finish_step(
    out: &mut String,
    collector_slot: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> CodegenResult<()> {
    let Some(output_slot) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(
        out,
        "    let _value = read_slot(slots, {})?;",
        collector_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    let _taint = read_taint(slot_taints, {})?;",
        collector_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot_with_taint(slots, slot_taints, {}, Some(_value), _taint)?;",
        output_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(out, "    collect_states.remove({});", collector_slot.get()).map_err(fmt_err)?;
    write_next_or_error(out, next)
}

fn emit_copy_to_output_then_next(
    out: &mut String,
    source: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> CodegenResult<()> {
    let Some(output_slot) = output else {
        return writeln!(
            out,
            "    Err(DriveError::MissingOutputSlot {{ step: {} }})",
            step.get()
        )
        .map_err(fmt_err);
    };
    writeln!(out, "    let _value = read_slot(slots, {})?;", source.get()).map_err(fmt_err)?;
    writeln!(
        out,
        "    let _taint = read_taint(slot_taints, {})?;",
        source.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    write_slot_with_taint(slots, slot_taints, {}, Some(_value), _taint)?;",
        output_slot.get()
    )
    .map_err(fmt_err)?;
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
        "    let _retry_state = read_retry_state_from_slot(slots, {}, CONTRACT_MAX_RETRY_ATTEMPTS)?;",
        policy_slot.get()
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    retry_check_target(_retry_state.current_attempt(), CONTRACT_MAX_RETRY_ATTEMPTS, {}, {})",
        body.get(),
        exhausted.get()
    )
    .map_err(fmt_err)
}

/// Generate an expression evaluator function.
pub fn emit_expr_function(
    out: &mut String,
    expr_idx: vb_core::ExprIdx,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    let Some(program) = workflow.expression(expr_idx) else {
        writeln!(
            out,
            "fn eval_expr_{}(_slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], _slot_taints: &[Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore) -> Result<(SlotValue, Taint), DriveError> {{",
            expr_idx.get()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "    Err(DriveError::ExprOutOfBounds {{ expr: {} }})",
            expr_idx.get()
        )
        .map_err(fmt_err)?;
        writeln!(out, "}}").map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
        return Ok(());
    };

    let slots_param = expr_slots_param(program);
    let slot_taints_param = expr_slot_taints_param(program);
    let list_store_param = expr_list_store_param(program);
    let object_store_param = expr_object_store_param(program);
    writeln!(
        out,
        "fn eval_expr_{}({slots_param}: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], {slot_taints_param}: &[Taint; WORKFLOW_SLOT_COUNT], {list_store_param}: &mut ListStore, {object_store_param}: &mut ObjectStore) -> Result<(SlotValue, Taint), DriveError> {{",
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
                    "    stack.push_tainted(read_slot(slots, {})?, read_taint(slot_taints, {})?)?;",
                    slot.get(),
                    slot.get()
                )
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
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push_tainted(SlotValue::Bool(_l == _r), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::NotEq => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push_tainted(SlotValue::Bool(_l != _r), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gt => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_li > _ri), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Gte => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_li >= _ri), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lt => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_li < _ri), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Lte => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_li <= _ri), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::And => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_lb && _rb), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Or => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _rb = match _r {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; let _lb = match _l {{ SlotValue::Bool(b) => b, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }}; stack.push_tainted(SlotValue::Bool(_lb || _rb), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Not => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; match _v {{ SlotValue::Bool(b) => stack.push_tainted(SlotValue::Bool(!b), _taint)?, other => return Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}) }} }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Add => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_add(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push_tainted(SlotValue::I64(_result), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Sub => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_sub(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push_tainted(SlotValue::I64(_result), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Mul => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_mul(_ri).ok_or(DriveError::IntegerOverflow)?; stack.push_tainted(SlotValue::I64(_result), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Div => {
                writeln!(out, "    {{ let (_r, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_l, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _ri = match _r {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _li = match _l {{ SlotValue::I64(v) => v, other => return Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}) }}; let _result = _li.checked_div(_ri).ok_or(DriveError::DivisionByZero)?; stack.push_tainted(SlotValue::I64(_result), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Contains => {
                writeln!(out, "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"text helper contains requires runtime symbol store\" }});")
                    .map_err(fmt_err)?;
            }
            ExprOp::StartsWith => {
                writeln!(out, "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"text helper starts_with requires runtime symbol store\" }});")
                    .map_err(fmt_err)?;
            }
            ExprOp::EndsWith => {
                writeln!(out, "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"text helper ends_with requires runtime symbol store\" }});")
                    .map_err(fmt_err)?;
            }
            ExprOp::Has => {
                writeln!(out, "    {{ let (_item, _it) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_container, _ct) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _handle = match _container {{ SlotValue::List(handle) => handle, other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}), }}; let _result = list_contains_item(list_store, _handle, _item)?; stack.push_tainted(SlotValue::Bool(_result), join_taint(_ct, _it))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Exists => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; stack.push_tainted(SlotValue::Bool(!matches!(_v, SlotValue::Null)), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Length => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _len = match _v {{ SlotValue::List(handle) => i64::from(list_item_count(list_store, handle)?), SlotValue::Object(handle) => i64::from(object_field_count(object_store, handle)?), other => return Err(DriveError::TypeMismatch {{ expected: \"list or object\", found: other.type_name() }}), }}; stack.push_tainted(SlotValue::I64(_len), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Empty => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _is_empty = match _v {{ SlotValue::List(handle) => list_item_count(list_store, handle)? == 0, SlotValue::Object(handle) => object_field_count(object_store, handle)? == 0, SlotValue::Null => true, other => return Err(DriveError::TypeMismatch {{ expected: \"list, object, or null\", found: other.type_name() }}), }}; stack.push_tainted(SlotValue::Bool(_is_empty), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Append => {
                writeln!(out, "    {{ let (_item, _item_taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_list, _list_taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _handle = expect_list_value(_list)?; let _new_list = append_list_item(list_store, _handle, _item, _item_taint)?; stack.push_tainted(SlotValue::List(_new_list), join_taint(_list_taint, _item_taint))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::AppendIf => {
                writeln!(out, "    {{ let (_condition, _ct) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_item, _item_taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_list, _list_taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _handle = expect_list_value(_list)?; let _cond = expect_bool_value(_condition)?; let _new_list = if _cond {{ append_list_item(list_store, _handle, _item, _item_taint)? }} else {{ clone_list_items(list_store, _handle)? }}; stack.push_tainted(SlotValue::List(_new_list), join_taints(&[_list_taint, _item_taint, _ct]))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Merge => {
                writeln!(out, "    {{ let (_right, _rt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let (_left, _lt) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _left_handle = expect_object_value(_left)?; let _right_handle = expect_object_value(_right)?; let _merged = merge_object_records(object_store, _left_handle, _right_handle)?; stack.push_tainted(SlotValue::Object(_merged), join_taint(_lt, _rt))?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Sum => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _handle = expect_list_value(_v)?; let _sum = sum_list_items(list_store, _handle)?; stack.push_tainted(SlotValue::I64(_sum), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Count => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _result = match _v {{ SlotValue::List(handle) => i64::from(list_item_count(list_store, handle)?), other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}), }}; stack.push_tainted(SlotValue::I64(_result), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            ExprOp::Unique => {
                writeln!(out, "    {{ let (_v, _taint) = stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)?; let _handle = expect_list_value(_v)?; let _unique = unique_list_items(list_store, _handle)?; stack.push_tainted(SlotValue::List(_unique), _taint)?; }}")
                    .map_err(fmt_err)?;
            }
            _ => {
                writeln!(out, "    return Err(DriveError::InvalidCompiledWorkflow {{ reason: \"future expression op\" }});")
                    .map_err(fmt_err)?;
            }
        }
    }

    writeln!(
        out,
        "    stack.pop_tainted().ok_or(DriveError::ExpressionStackUnderflow)"
    )
    .map_err(fmt_err)?;
    writeln!(out, "}}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

fn expr_slot_taints_param(program: &vb_core::ExprProgram) -> &'static str {
    if program
        .ops
        .as_ref()
        .iter()
        .any(|op| matches!(op, ExprOp::LoadSlot(_) | ExprOp::LoadAccessor(_)))
    {
        "slot_taints"
    } else {
        "_slot_taints"
    }
}

fn expr_slots_param(program: &vb_core::ExprProgram) -> &'static str {
    if program
        .ops
        .as_ref()
        .iter()
        .any(|op| matches!(op, ExprOp::LoadSlot(_) | ExprOp::LoadAccessor(_)))
    {
        "slots"
    } else {
        "_slots"
    }
}

fn expr_list_store_param(program: &vb_core::ExprProgram) -> &'static str {
    if program.ops.as_ref().iter().any(|op| {
        matches!(
            op,
            ExprOp::Has
                | ExprOp::Length
                | ExprOp::Empty
                | ExprOp::Append
                | ExprOp::AppendIf
                | ExprOp::Sum
                | ExprOp::Count
                | ExprOp::Unique
                | ExprOp::LoadAccessor(_)
        )
    }) {
        "list_store"
    } else {
        "_list_store"
    }
}

fn expr_object_store_param(program: &vb_core::ExprProgram) -> &'static str {
    if program.ops.as_ref().iter().any(|op| {
        matches!(
            op,
            ExprOp::Has | ExprOp::Length | ExprOp::Empty | ExprOp::Merge | ExprOp::LoadAccessor(_)
        )
    }) {
        "object_store"
    } else {
        "_object_store"
    }
}

/// Generate action dispatch boundaries for external action nodes.
pub fn emit_action_boundary(
    out: &mut String,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    next: Option<StepIdx>,
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
        "    if read_taint(slot_taints, {})? != Taint::Clean {{ return Err(DriveError::TaintViolation {{ step: {} }}); }}",
        input.get(),
        step.get()
    )
    .map_err(fmt_err)?;
    match next {
        Some(resume_pc) => writeln!(out, "    Err(SuspensionOutcome::ActionPending {{ step: {}, action_id: {}, input_slot: {}, resume_pc: {} }}.into_drive_error())", step.get(), action.get(), input.get(), resume_pc.get()).map_err(fmt_err),
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
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

/// Emit fixed-capacity value arena bounds for generated workflows.
pub fn emit_value_store_contract(
    out: &mut String,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    writeln!(out, "// --- Generated value arena contract ---").map_err(fmt_err)?;
    writeln!(
        out,
        "const LIST_STORE_RECORD_CAPACITY: usize = {};",
        list_store_record_capacity(workflow)?
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const LIST_STORE_VALUE_CAPACITY: usize = {};",
        list_store_value_capacity(workflow)?
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const OBJECT_STORE_RECORD_CAPACITY: usize = {};",
        object_store_record_capacity(workflow)?
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const OBJECT_STORE_FIELD_CAPACITY: usize = {};",
        object_store_field_capacity(workflow)?
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "const COLLECT_STATE_CAPACITY: usize = {};",
        collect_state_capacity(workflow)?
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// Backwards-compatible list store contract emitter.
pub fn emit_list_store_contract(
    out: &mut String,
    workflow: &CompiledWorkflow,
) -> CodegenResult<()> {
    emit_value_store_contract(out, workflow)
}

fn list_store_record_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    let metrics = list_store_metrics(workflow)?;
    let expr_metrics = expression_store_metrics(workflow)?;
    let foreach_tail_capacity = checked_metric_mul(
        metrics.for_each_steps,
        metrics.total_build_list_items.max(1),
        "list store foreach tail capacity overflow",
    )?;
    let collect_capacity = collect_list_record_capacity(workflow)?;
    checked_metric_add(
        checked_metric_add(
            checked_metric_add(
                metrics.build_list_count,
                foreach_tail_capacity,
                "list store record capacity overflow",
            )?,
            expr_metrics.list_allocating_ops,
            "list store record capacity overflow",
        )?,
        checked_metric_add(1, collect_capacity, "list store record capacity overflow")?,
        "list store record capacity overflow",
    )
    .map(|capacity| capacity.max(1))
}

fn list_store_value_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    let metrics = list_store_metrics(workflow)?;
    let expr_metrics = expression_store_metrics(workflow)?;
    let expression_value_capacity = checked_metric_mul(
        expr_metrics.list_allocating_ops,
        checked_metric_add(
            metrics.total_build_list_items.max(1),
            expr_metrics.list_allocating_ops,
            "list store expression value capacity overflow",
        )?,
        "list store expression value capacity overflow",
    )?;
    let collect_capacity = collect_list_value_capacity(workflow)?;
    checked_metric_add(
        checked_metric_mul(
            metrics.build_list_count,
            metrics.total_build_list_items.max(1),
            "list store value capacity overflow",
        )?,
        expression_value_capacity,
        "list store value capacity overflow",
    )
    .and_then(|capacity| {
        checked_metric_add(
            capacity,
            collect_capacity,
            "list store value capacity overflow",
        )
    })
    .map(|capacity| capacity.max(1))
}

fn collect_state_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    let mut count = 0usize;
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step)
            && matches!(node.kind, CompiledNodeKind::CollectStart { .. })
        {
            count = checked_metric_add(count, 1, "collect state capacity overflow")?;
        }
    }
    Ok(count.max(1))
}

fn collect_list_record_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    checked_metric_mul(
        collect_state_capacity(workflow)?,
        checked_metric_add(
            usize::try_from(workflow.resource_contract().max_collect_items).map_err(|_| {
                CodegenError::SemanticMismatch {
                    detail: "collect item capacity overflow".into(),
                }
            })?,
            3,
            "collect record capacity overflow",
        )?,
        "collect record capacity overflow",
    )
}

fn collect_list_value_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    checked_metric_mul(
        collect_state_capacity(workflow)?,
        checked_metric_add(
            usize::try_from(workflow.resource_contract().max_collect_items).map_err(|_| {
                CodegenError::SemanticMismatch {
                    detail: "collect item capacity overflow".into(),
                }
            })?,
            1,
            "collect value capacity overflow",
        )?,
        "collect value capacity overflow",
    )
}

fn object_store_record_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    let metrics = value_store_metrics(workflow)?;
    let expr_metrics = expression_store_metrics(workflow)?;
    checked_metric_add(
        metrics.build_object_count,
        expr_metrics.object_allocating_ops,
        "object store record capacity overflow",
    )
    .map(|capacity| capacity.max(1))
}

fn object_store_field_capacity(workflow: &CompiledWorkflow) -> CodegenResult<usize> {
    let metrics = value_store_metrics(workflow)?;
    let expr_metrics = expression_store_metrics(workflow)?;
    let expression_field_capacity = checked_metric_mul(
        expr_metrics.object_allocating_ops,
        checked_metric_mul(
            metrics.total_build_object_fields.max(1),
            2,
            "object store expression field capacity overflow",
        )?,
        "object store expression field capacity overflow",
    )?;
    checked_metric_add(
        checked_metric_mul(
            metrics.build_object_count.max(1),
            metrics.total_build_object_fields.max(1),
            "object store field capacity overflow",
        )?,
        expression_field_capacity,
        "object store field capacity overflow",
    )
    .map(|capacity| capacity.max(1))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ExpressionStoreMetrics {
    list_allocating_ops: usize,
    object_allocating_ops: usize,
}

fn expression_store_metrics(workflow: &CompiledWorkflow) -> CodegenResult<ExpressionStoreMetrics> {
    let mut metrics = ExpressionStoreMetrics::default();
    let mut expr_idx = 0u16;
    while let Some(program) = workflow.expression(vb_core::ExprIdx::new(expr_idx)) {
        update_expression_store_metrics(&mut metrics, program)?;
        if expr_idx == u16::MAX {
            break;
        }
        expr_idx = expr_idx.saturating_add(1);
    }
    Ok(metrics)
}

fn update_expression_store_metrics(
    metrics: &mut ExpressionStoreMetrics,
    program: &vb_core::ExprProgram,
) -> CodegenResult<()> {
    for op in program.ops.as_ref() {
        match op {
            ExprOp::Append | ExprOp::AppendIf | ExprOp::Unique => {
                metrics.list_allocating_ops = checked_metric_add(
                    metrics.list_allocating_ops,
                    1,
                    "list store expression capacity overflow",
                )?;
            }
            ExprOp::Merge => {
                metrics.object_allocating_ops = checked_metric_add(
                    metrics.object_allocating_ops,
                    1,
                    "object store expression capacity overflow",
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn checked_metric_add(left: usize, right: usize, detail: &'static str) -> CodegenResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| CodegenError::SemanticMismatch {
            detail: detail.into(),
        })
}

fn checked_metric_mul(left: usize, right: usize, detail: &'static str) -> CodegenResult<usize> {
    left.checked_mul(right)
        .ok_or_else(|| CodegenError::SemanticMismatch {
            detail: detail.into(),
        })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ListStoreMetrics {
    build_list_count: usize,
    total_build_list_items: usize,
    for_each_steps: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ValueStoreMetrics {
    build_object_count: usize,
    total_build_object_fields: usize,
}

fn value_store_metrics(workflow: &CompiledWorkflow) -> CodegenResult<ValueStoreMetrics> {
    let mut metrics = ValueStoreMetrics::default();
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step) {
            update_value_store_metrics(&mut metrics, &node.kind)?;
        }
    }
    Ok(metrics)
}

fn update_value_store_metrics(
    metrics: &mut ValueStoreMetrics,
    kind: &CompiledNodeKind,
) -> CodegenResult<()> {
    if let CompiledNodeKind::BuildObject { fields } = kind {
        metrics.build_object_count = checked_metric_add(
            metrics.build_object_count,
            1,
            "object store record capacity overflow",
        )?;
        metrics.total_build_object_fields = checked_metric_add(
            metrics.total_build_object_fields,
            fields.len(),
            "object store field capacity overflow",
        )?;
    }
    Ok(())
}

fn list_store_metrics(workflow: &CompiledWorkflow) -> CodegenResult<ListStoreMetrics> {
    let mut metrics = ListStoreMetrics::default();
    for step_idx in 0..workflow.node_count() {
        let step = StepIdx::new(step_idx);
        if let Some(node) = workflow.node(step) {
            update_list_store_metrics(&mut metrics, &node.kind)?;
        }
    }
    Ok(metrics)
}

fn update_list_store_metrics(
    metrics: &mut ListStoreMetrics,
    kind: &CompiledNodeKind,
) -> CodegenResult<()> {
    match kind {
        CompiledNodeKind::BuildList { items } => {
            metrics.build_list_count = checked_metric_add(
                metrics.build_list_count,
                1,
                "list store record capacity overflow",
            )?;
            metrics.total_build_list_items = checked_metric_add(
                metrics.total_build_list_items,
                items.len(),
                "list store value capacity overflow",
            )?;
        }
        CompiledNodeKind::ForEachStart { .. } | CompiledNodeKind::ForEachNext { .. } => {
            metrics.for_each_steps = checked_metric_add(
                metrics.for_each_steps,
                1,
                "list store foreach capacity overflow",
            )?;
        }
        _ => {}
    }
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

const GENERATED_STORAGE_HELPERS: &str = include_str!("generated_storage_helpers.rs.txt");

fn write_header(out: &mut String) -> CodegenResult<()> {
    writeln!(out, "#![forbid(unsafe_code)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(unused_must_use)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(unreachable_pub)]").map_err(fmt_err)?;
    writeln!(out, "#![deny(rust_2018_idioms)]").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "//! Generated workflow - DO NOT EDIT").map_err(fmt_err)?;
    writeln!(out, "//! Produced by vb_codegen emit_rust_workflow").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "use std::convert::TryFrom;").map_err(fmt_err)?;
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
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").map_err(fmt_err)?;
    writeln!(out, "pub enum Taint {{ Clean, DerivedFromSecret, Secret }}").map_err(fmt_err)?;
    writeln!(out, "const fn join_taint(left: Taint, right: Taint) -> Taint {{ match (left, right) {{ (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret, (Taint::DerivedFromSecret, _) | (_, Taint::DerivedFromSecret) => Taint::DerivedFromSecret, (Taint::Clean, Taint::Clean) => Taint::Clean }} }}").map_err(fmt_err)?;
    writeln!(out, "fn join_taints(values: &[Taint]) -> Taint {{ values.iter().copied().fold(Taint::Clean, join_taint) }}").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").map_err(fmt_err)?;
    writeln!(out, "pub enum DriveError {{").map_err(fmt_err)?;
    writeln!(out, "    InvalidProgramCounter,").map_err(fmt_err)?;
    writeln!(out, "    MissingNextStep,").map_err(fmt_err)?;
    writeln!(out, "    MissingOutputSlot {{ step: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    SlotOutOfBounds {{ slot: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    ExprOutOfBounds {{ expr: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    StepBudgetExhausted,").map_err(fmt_err)?;
    writeln!(out, "    TaintViolation {{ step: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    JournalOverflow,").map_err(fmt_err)?;
    writeln!(out, "    InvalidResume {{ step: u16 }},").map_err(fmt_err)?;
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
        "    IterationLimitExceeded {{ resource: &'static str }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    ListStoreOverflow,").map_err(fmt_err)?;
    writeln!(out, "    InvalidListHandle,").map_err(fmt_err)?;
    writeln!(out, "    ObjectStoreOverflow,").map_err(fmt_err)?;
    writeln!(out, "    InvalidObjectHandle,").map_err(fmt_err)?;
    writeln!(out, "    ObjectFieldOutOfBounds,").map_err(fmt_err)?;
    writeln!(out, "    ObjectFieldOffsetOverflow,").map_err(fmt_err)?;
    writeln!(out, "    MissingField {{ field: u32 }},").map_err(fmt_err)?;
    writeln!(out, "    ListIndexOutOfBounds {{ index: u32 }},").map_err(fmt_err)?;
    writeln!(out, "    AccessorPathTooDeep {{ depth: u16, max: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    InvalidRetryState,").map_err(fmt_err)?;
    writeln!(out, "    InvalidRetryPolicy,").map_err(fmt_err)?;
    writeln!(out, "    CollectItemLimitExceeded,").map_err(fmt_err)?;
    writeln!(out, "    CollectPageLimitExceeded,").map_err(fmt_err)?;
    writeln!(out, "    CollectPageOrderViolation {{ kind: CollectPageOrderViolationKind, run_id: u64, collector_slot: u16, expected_page: u32, observed_page: u32 }},").map_err(fmt_err)?;
    writeln!(
        out,
        "    ActionSuspend {{ step: u16, action_id: u16, input_slot: u16, resume_pc: u16 }},"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    WaitUntilSuspend {{ step: u16, deadline_slot: u16, resume_pc: u16 }},"
    )
    .map_err(fmt_err)?;
    writeln!(out, "    WaitEventSuspend {{ step: u16, event_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    AskSuspend {{ step: u16, prompt_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 }},").map_err(fmt_err)?;
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
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").map_err(fmt_err)?;
    writeln!(
        out,
        "pub enum CollectPageOrderViolationKind {{ Duplicate, Stale, OutOfOrder }}"
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").map_err(fmt_err)?;
    writeln!(out, "pub enum GeneratedSuspension {{ ActionPending {{ step: u16, action_id: u16, input_slot: u16, resume_pc: u16 }}, WaitUntil {{ step: u16, deadline_slot: u16, resume_pc: u16 }}, WaitEvent {{ step: u16, event_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 }}, AskPending {{ step: u16, prompt_slot: u16, timeout_slot: Option<u16>, resume_pc: u16 }} }}").map_err(fmt_err)?;
    writeln!(out, "type SuspensionOutcome = GeneratedSuspension;").map_err(fmt_err)?;
    writeln!(out, "impl SuspensionOutcome {{ fn into_drive_error(self) -> DriveError {{ match self {{ Self::ActionPending {{ step, action_id, input_slot, resume_pc }} => DriveError::ActionSuspend {{ step, action_id, input_slot, resume_pc }}, Self::WaitUntil {{ step, deadline_slot, resume_pc }} => DriveError::WaitUntilSuspend {{ step, deadline_slot, resume_pc }}, Self::WaitEvent {{ step, event_slot, timeout_slot, resume_pc }} => DriveError::WaitEventSuspend {{ step, event_slot, timeout_slot, resume_pc }}, Self::AskPending {{ step, prompt_slot, timeout_slot, resume_pc }} => DriveError::AskSuspend {{ step, prompt_slot, timeout_slot, resume_pc }}, }} }} }}").map_err(fmt_err)?;
    writeln!(
        out,
        "enum StepOutcome {{ Continue(u16), Finished(SlotValue) }}"
    )
    .map_err(fmt_err)?;
    out.write_str(r#"
#[derive(Clone, Copy, PartialEq)]
pub enum JournalEvent {
    StepStarted { step: u16 },
    SlotWritten { slot: u16, value: Option<SlotValue>, taint: Taint, extra: Option<CollectState> },
    StepSucceeded { step: u16, output: Option<u16> },
    ActionScheduled { step: u16, action_id: u16, input_slot: u16, resume_pc: u16 },
    ActionCompleted { step: u16, action_id: u16, output_slot: u16, value: SlotValue, taint: Taint },
    AskAnswered { ask_step: u16, resume_step: u16, answer_slot: u16, value: SlotValue, taint: Taint },
    RunFinished { step: u16, value: SlotValue, taint: Taint },
}

impl core::fmt::Debug for JournalEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StepStarted { step } => f.debug_struct("StepStarted").field("step", step).finish(),
            Self::SlotWritten { slot, value, taint, extra } => {
                let mut event = f.debug_struct("SlotWritten");
                event.field("slot", slot).field("value", value).field("taint", taint);
                if let Some(state) = extra {
                    event.field("extra", state);
                }
                event.finish()
            }
            Self::StepSucceeded { step, output } => f.debug_struct("StepSucceeded").field("step", step).field("output", output).finish(),
            Self::ActionScheduled { step, action_id, input_slot, resume_pc } => f.debug_struct("ActionScheduled").field("step", step).field("action_id", action_id).field("input_slot", input_slot).field("resume_pc", resume_pc).finish(),
            Self::ActionCompleted { step, action_id, output_slot, value, taint } => f.debug_struct("ActionCompleted").field("step", step).field("action_id", action_id).field("output_slot", output_slot).field("value", value).field("taint", taint).finish(),
            Self::AskAnswered { ask_step, resume_step, answer_slot, value, taint } => f.debug_struct("AskAnswered").field("ask_step", ask_step).field("resume_step", resume_step).field("answer_slot", answer_slot).field("value", value).field("taint", taint).finish(),
            Self::RunFinished { step, value, taint } => f.debug_struct("RunFinished").field("step", step).field("value", value).field("taint", taint).finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journal { events: [Option<JournalEvent>; GENERATED_JOURNAL_CAPACITY], len: u16 }
impl Journal {
    pub const fn new() -> Self { Self { events: [None; GENERATED_JOURNAL_CAPACITY], len: 0 } }
    pub const fn len(&self) -> u16 { self.len }
    fn ensure_capacity(&self, needed: usize) -> Result<(), DriveError> {
        let used = usize::from(self.len);
        let available = GENERATED_JOURNAL_CAPACITY.checked_sub(used).ok_or(DriveError::JournalOverflow)?;
        if available < needed { return Err(DriveError::JournalOverflow); }
        Ok(())
    }
    pub fn event(&self, index: u16) -> Option<JournalEvent> {
        if index >= self.len { return None; }
        self.events.get(usize::from(index)).copied().flatten()
    }
    fn push(&mut self, event: JournalEvent) -> Result<(), DriveError> {
        self.ensure_capacity(1)?;
        let index = usize::from(self.len);
        match self.events.get_mut(index) {
            Some(slot) => *slot = Some(event),
            None => return Err(DriveError::JournalOverflow),
        }
        self.len = self.len.checked_add(1).ok_or(DriveError::JournalOverflow)?;
        Ok(())
    }
}

impl Default for Journal {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveOutput { pub value: SlotValue, pub taint: Taint, pub journal: Journal }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuspendedRun { pub suspension: GeneratedSuspension, pub journal: Journal }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneratedRunStatus { Finished(DriveOutput), Suspended(SuspendedRun) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingResume {
    Action { step: u16, action_id: u16, resume_pc: u16 },
    Ask { ask_step: u16, resume_pc: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState { Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled }

impl PendingResume {
    const fn step(self) -> u16 {
        match self {
            Self::Action { step, .. } => step,
            Self::Ask { ask_step, .. } => ask_step,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectState { source: u32, current_page: u32, cursor: u32, page_size: u32, item_count: u32, limit: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CollectStateRecord { collector_slot: u16, state: CollectState }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CollectLineage { collector_slot: u16, previous_page: Option<u32>, stale_pages: [Option<u32>; COLLECT_STATE_CAPACITY], stale_len: u16 }

impl CollectLineage {
    const fn empty() -> Self { Self { collector_slot: 0, previous_page: None, stale_pages: [None; COLLECT_STATE_CAPACITY], stale_len: 0 } }

    fn record_transition(&mut self, next_page: u32, current_page: Option<u32>) -> Result<(), DriveError> {
        let Some(current) = current_page else { return Ok(()); };
        if current == next_page { return Ok(()); }
        if let Some(previous) = self.previous_page {
            let index = usize::from(self.stale_len);
            match self.stale_pages.get_mut(index) {
                Some(slot) => *slot = Some(previous),
                None => return Err(DriveError::CollectItemLimitExceeded),
            }
            self.stale_len = self.stale_len.checked_add(1).ok_or(DriveError::CollectItemLimitExceeded)?;
        }
        self.previous_page = Some(current);
        Ok(())
    }

    fn is_stale(&self, page: u32) -> Result<bool, DriveError> {
        let mut index = 0u16;
        while index < self.stale_len {
            let page_slot = self.stale_pages.get(usize::from(index)).copied().flatten();
            if page_slot == Some(page) { return Ok(true); }
            index = index.checked_add(1).ok_or(DriveError::CollectItemLimitExceeded)?;
        }
        Ok(false)
    }
}

pub struct CollectStateStore { run_id: u64, entries: [Option<CollectStateRecord>; COLLECT_STATE_CAPACITY], lineages: [CollectLineage; COLLECT_STATE_CAPACITY] }

impl CollectStateStore {
    const fn new() -> Self { Self { run_id: 0, entries: [None; COLLECT_STATE_CAPACITY], lineages: [CollectLineage::empty(); COLLECT_STATE_CAPACITY] } }

    fn set_run_id(&mut self, run_id: u64) { self.run_id = run_id; }

    fn entry_index(&self, collector_slot: u16) -> Option<usize> {
        let mut index = 0usize;
        while index < COLLECT_STATE_CAPACITY {
            match self.entries.get(index).copied().flatten() {
                Some(record) if record.collector_slot == collector_slot => return Some(index),
                _ => {}
            }
            match index.checked_add(1) { Some(next) => index = next, None => return None }
        }
        None
    }

    fn lineage_mut(&mut self, collector_slot: u16) -> Result<&mut CollectLineage, DriveError> {
        let mut empty_index: Option<usize> = None;
        let mut index = 0usize;
        while index < COLLECT_STATE_CAPACITY {
            let lineage = self.lineages.get(index).copied().ok_or(DriveError::CollectItemLimitExceeded)?;
            if lineage.collector_slot == collector_slot { return self.lineages.get_mut(index).ok_or(DriveError::CollectItemLimitExceeded); }
            if lineage.previous_page.is_none() && lineage.stale_len == 0 && empty_index.is_none() { empty_index = Some(index); }
            index = index.checked_add(1).ok_or(DriveError::CollectItemLimitExceeded)?;
        }
        let slot_index = empty_index.ok_or(DriveError::CollectItemLimitExceeded)?;
        let slot = self.lineages.get_mut(slot_index).ok_or(DriveError::CollectItemLimitExceeded)?;
        slot.collector_slot = collector_slot;
        Ok(slot)
    }

    fn upsert(&mut self, collector_slot: u16, state: CollectState) -> Result<(), DriveError> {
        let old_page = self.entry_index(collector_slot).and_then(|index| self.entries.get(index).copied().flatten().map(|record| record.state.current_page));
        self.lineage_mut(collector_slot)?.record_transition(state.current_page, old_page)?;
        if let Some(index) = self.entry_index(collector_slot) {
            match self.entries.get_mut(index) { Some(slot) => *slot = Some(CollectStateRecord { collector_slot, state }), None => return Err(DriveError::CollectItemLimitExceeded) }
            return Ok(());
        }
        let mut index = 0usize;
        while index < COLLECT_STATE_CAPACITY {
            match self.entries.get_mut(index) {
                Some(slot @ None) => { *slot = Some(CollectStateRecord { collector_slot, state }); return Ok(()); }
                Some(Some(_)) => {}
                None => return Err(DriveError::CollectItemLimitExceeded),
            }
            index = index.checked_add(1).ok_or(DriveError::CollectItemLimitExceeded)?;
        }
        Err(DriveError::CollectItemLimitExceeded)
    }

    fn require_current_page(&self, collector_slot: u16, observed_page: u32) -> Result<CollectState, DriveError> {
        if let Some(index) = self.entry_index(collector_slot) {
            let record = self.entries.get(index).copied().flatten().ok_or(DriveError::CollectItemLimitExceeded)?;
            if record.state.current_page == observed_page { return Ok(record.state); }
            return Err(DriveError::CollectPageOrderViolation { kind: self.classify_observed_page(collector_slot, observed_page)?, run_id: self.run_id, collector_slot, expected_page: record.state.current_page, observed_page });
        }
        Err(DriveError::InvalidCompiledWorkflow { reason: "collect pagination state missing" })
    }

    fn classify_observed_page(&self, collector_slot: u16, observed_page: u32) -> Result<CollectPageOrderViolationKind, DriveError> {
        let mut index = 0usize;
        while index < COLLECT_STATE_CAPACITY {
            let lineage = self.lineages.get(index).copied().ok_or(DriveError::CollectItemLimitExceeded)?;
            if lineage.collector_slot == collector_slot {
                if lineage.previous_page == Some(observed_page) { return Ok(CollectPageOrderViolationKind::Duplicate); }
                if lineage.is_stale(observed_page)? { return Ok(CollectPageOrderViolationKind::Stale); }
                return Ok(CollectPageOrderViolationKind::OutOfOrder);
            }
            index = index.checked_add(1).ok_or(DriveError::CollectItemLimitExceeded)?;
        }
        Ok(CollectPageOrderViolationKind::OutOfOrder)
    }

    fn remove(&mut self, collector_slot: u16) {
        if let Some(index) = self.entry_index(collector_slot) {
            if let Some(slot) = self.entries.get_mut(index) { *slot = None; }
        }
    }

    fn capture_state(&self, collector_slot: u16) -> Option<CollectState> {
        self.entry_index(collector_slot)
            .and_then(|index| self.entries.get(index).copied().flatten())
            .map(|record| record.state)
    }
}

fn set_step_state(states: &mut [StepState; WORKFLOW_NODE_COUNT_USIZE], step: u16, state: StepState) -> Result<(), DriveError> {
    let index = usize::from(step);
    match states.get_mut(index) {
        Some(slot) => { *slot = state; Ok(()) }
        None => Err(DriveError::InvalidProgramCounter),
    }
}

pub struct GeneratedRunState {
    run_id: u64,
    slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT],
    slot_taints: [Taint; WORKFLOW_SLOT_COUNT],
    step_states: [StepState; WORKFLOW_NODE_COUNT_USIZE],
    pc: u16,
    step_budget_remaining: u64,
    list_store: ListStore,
    object_store: ObjectStore,
    collect_states: CollectStateStore,
    journal: Journal,
    pending: Option<PendingResume>,
}

impl GeneratedRunState {
    fn record_slot_changes(
        &mut self,
        current_pc: u16,
        before_slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT],
        before_taints: &[Taint; WORKFLOW_SLOT_COUNT],
    ) -> Result<(), DriveError> {
        let Some(slot) = journal_observed_slot(current_pc)? else { return Ok(()); };
        self.journal.ensure_capacity(1)?;
        let index = usize::from(slot);
        let before_value = before_slots.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
        let after_value = self.slots.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
        let before_taint = before_taints.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
        let after_taint = self.slot_taints.get(index).copied().ok_or(DriveError::SlotOutOfBounds { slot })?;
        if after_value.is_some() || before_value != after_value || before_taint != after_taint {
            let extra = self.collect_states.capture_state(slot);
            self.journal.push(JournalEvent::SlotWritten { slot, value: after_value, taint: after_taint, extra })?;
        }
        Ok(())
    }

    fn write_slot_with_journal(&mut self, slot: u16, value: Option<SlotValue>, taint: Taint) -> Result<(), DriveError> {
        self.journal.ensure_capacity(1)?;
        write_slot_with_taint(&mut self.slots, &mut self.slot_taints, slot, value, taint)?;
        self.journal.push(JournalEvent::SlotWritten { slot, value, taint, extra: None })
    }

    fn suspend_from_error(&mut self, error: DriveError) -> Result<GeneratedRunStatus, DriveError> {
        match error {
            DriveError::ActionSuspend { step, action_id, input_slot, resume_pc } => {
                if self.pending.is_some() { return Err(DriveError::InvalidResume { step }); }
                self.journal.ensure_capacity(1)?;
                let suspension = GeneratedSuspension::ActionPending { step, action_id, input_slot, resume_pc };
                self.journal.push(JournalEvent::ActionScheduled { step, action_id, input_slot, resume_pc })?;
                self.pending = Some(PendingResume::Action { step, action_id, resume_pc });
                Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension, journal: self.journal }))
            }
            DriveError::WaitUntilSuspend { step, deadline_slot, resume_pc } => Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::WaitUntil { step, deadline_slot, resume_pc }, journal: self.journal })),
            DriveError::WaitEventSuspend { step, event_slot, timeout_slot, resume_pc } => Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::WaitEvent { step, event_slot, timeout_slot, resume_pc }, journal: self.journal })),
            DriveError::AskSuspend { step, prompt_slot, timeout_slot, resume_pc } => {
                if self.pending.is_some() { return Err(DriveError::InvalidResume { step }); }
                self.pending = Some(PendingResume::Ask { ask_step: step, resume_pc });
                Ok(GeneratedRunStatus::Suspended(SuspendedRun { suspension: GeneratedSuspension::AskPending { step, prompt_slot, timeout_slot, resume_pc }, journal: self.journal }))
            }
            other => Err(other),
        }
    }
}

"#).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    out.write_str(r"#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryState { current_attempt: u16, remaining: u16, current_delay_ms: u32 }
impl RetryState {
    const fn from_parts(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> Self {
        Self { current_attempt, remaining, current_delay_ms }
    }

    pub fn new(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> Result<Self, DriveError> {
        if retry_state_is_legal(current_attempt, remaining, current_delay_ms) {
            Ok(Self::from_parts(current_attempt, remaining, current_delay_ms))
        } else {
            Err(DriveError::InvalidRetryState)
        }
    }

    pub const fn current_attempt(&self) -> u16 { self.current_attempt }
    pub const fn remaining(&self) -> u16 { self.remaining }
    pub const fn current_delay_ms(&self) -> u32 { self.current_delay_ms }

    pub fn decode(packed: i64, max_attempts: u16) -> Result<Self, DriveError> {
        let unsigned = retry_unsigned_bits(packed)?;
        Self::from_decoded_parts(
            retry_attempt_bits(unsigned)?,
            retry_remaining_bits(unsigned)?,
            retry_delay_bits(unsigned)?,
            max_attempts,
        )
    }

    fn from_decoded_parts(current_attempt: u16, remaining: u16, current_delay_ms: u32, max_attempts: u16) -> Result<Self, DriveError> {
        if retry_decoded_state_is_legal(current_attempt, remaining, current_delay_ms, max_attempts) {
            Ok(Self::from_parts(current_attempt, remaining, current_delay_ms))
        } else {
            Err(DriveError::InvalidRetryState)
        }
    }
}

fn retry_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> bool {
    retry_zero_state_is_legal(current_attempt, remaining, current_delay_ms)
        || (current_attempt > 0 && current_delay_ms == 0 && remaining == 0)
}

fn retry_decoded_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32, max_attempts: u16) -> bool {
    retry_zero_state_is_legal(current_attempt, remaining, current_delay_ms)
        || retry_active_state_is_legal(current_attempt, remaining, max_attempts)
}

fn retry_zero_state_is_legal(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> bool {
    current_attempt == 0 && remaining == 0 && current_delay_ms == 0
}

fn retry_active_state_is_legal(current_attempt: u16, remaining: u16, max_attempts: u16) -> bool {
    let Some(total_attempts) = current_attempt.checked_add(remaining) else { return false; };
    let Some(max_live_attempts) = max_attempts.checked_add(1) else { return false; };
    max_attempts > 0 && current_attempt > 0 && current_attempt <= max_attempts && remaining <= max_attempts && total_attempts <= max_live_attempts
}

fn retry_unsigned_bits(packed: i64) -> Result<u64, DriveError> {
    u64::try_from(packed).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_delay_bits(unsigned: u64) -> Result<u32, DriveError> {
    u32::try_from((unsigned >> 32) & 4_294_967_295_u64).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_attempt_bits(unsigned: u64) -> Result<u16, DriveError> {
    u16::try_from((unsigned >> 16) & 65_535_u64).map_err(|_| DriveError::InvalidRetryState)
}

fn retry_remaining_bits(unsigned: u64) -> Result<u16, DriveError> {
    u16::try_from(unsigned & 65_535_u64).map_err(|_| DriveError::InvalidRetryState)
}

const REPEAT_STATE_SHIFT: u32 = 32;

fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> Result<i64, DriveError> {
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(DriveError::InvalidRetryState);
    }
    let high = i64::from(max_attempts)
        .checked_shl(REPEAT_STATE_SHIFT)
        .ok_or(DriveError::IntegerOverflow)?;
    high.checked_add(i64::from(current_attempt))
        .ok_or(DriveError::IntegerOverflow)
}

fn decode_repeat_state(packed: i64) -> Result<(u16, u16), DriveError> {
    let bits = u64::try_from(packed).map_err(|_| DriveError::InvalidRetryState)?;
    let max_bits = bits >> REPEAT_STATE_SHIFT;
    let low_bits = bits & 65_535_u64;
    let reserved_bits = bits & 4_294_901_760_u64;
    if reserved_bits != 0 {
        return Err(DriveError::InvalidRetryState);
    }
    let max_attempts = u16::try_from(max_bits).map_err(|_| DriveError::InvalidRetryState)?;
    let current_attempt = u16::try_from(low_bits).map_err(|_| DriveError::InvalidRetryState)?;
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(DriveError::InvalidRetryState);
    }
    Ok((max_attempts, current_attempt))
}
")
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "const MAX_EXPRESSION_STACK: usize = 64;").map_err(fmt_err)?;
    writeln!(
        out,
        "const ACCESSOR_MAX_PATH_DEPTH: u16 = {ACCESSOR_MAX_PATH_DEPTH};"
    )
    .map_err(fmt_err)?;
    out.write_str(r"struct ExprStack { values: [SlotValue; MAX_EXPRESSION_STACK], taints: [Taint; MAX_EXPRESSION_STACK], len: u8, capacity: u8 }
impl ExprStack {
    fn new(capacity: u8) -> Result<Self, DriveError> {
        if usize::from(capacity) <= MAX_EXPRESSION_STACK {
            Ok(Self { values: [SlotValue::Null; MAX_EXPRESSION_STACK], taints: [Taint::Clean; MAX_EXPRESSION_STACK], len: 0, capacity })
        } else {
            Err(DriveError::ExpressionStackOverflow { max: capacity })
        }
    }

    fn push(&mut self, value: SlotValue) -> Result<(), DriveError> {
        self.push_tainted(value, Taint::Clean)
    }

    fn push_tainted(&mut self, value: SlotValue, taint: Taint) -> Result<(), DriveError> {
        if self.len >= self.capacity {
            return Err(DriveError::ExpressionStackOverflow { max: self.capacity });
        }
        let index = usize::from(self.len);
        match (self.values.get_mut(index), self.taints.get_mut(index)) {
            (Some(value_slot), Some(taint_slot)) => {
                *value_slot = value;
                *taint_slot = taint;
            }
            (_, _) => return Err(DriveError::ExpressionStackOverflow { max: self.capacity }),
        }
        self.len = self.len.checked_add(1).ok_or(DriveError::ExpressionStackOverflow { max: self.capacity })?;
        Ok(())
    }

    fn pop(&mut self) -> Option<SlotValue> {
        self.pop_tainted().map(|entry| entry.0)
    }

    fn pop_tainted(&mut self) -> Option<(SlotValue, Taint)> {
        if self.len == 0 {
            return None;
        }
        self.len = self.len.checked_sub(1)?;
        let index = usize::from(self.len);
        match (self.values.get(index).copied(), self.taints.get(index).copied()) {
            (Some(value), Some(taint)) => Some((value, taint)),
            (_, _) => None,
        }
    }
}
")
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    writeln!(out, "{GENERATED_STORAGE_HELPERS}").map_err(fmt_err)?;
    writeln!(out, "fn read_const(index: u16) -> Result<SlotValue, DriveError> {{ CONSTANTS.get(usize::from(index)).copied().ok_or(DriveError::InvalidCompiledWorkflow {{ reason: \"constant index out of bounds\" }}) }}").map_err(fmt_err)?;
    writeln!(out, "fn expect_list_value(value: SlotValue) -> Result<u32, DriveError> {{ match value {{ SlotValue::List(handle) => Ok(handle), other => Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn expect_object_value(value: SlotValue) -> Result<u32, DriveError> {{ match value {{ SlotValue::Object(handle) => Ok(handle), other => Err(DriveError::TypeMismatch {{ expected: \"object\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn expect_bool_value(value: SlotValue) -> Result<bool, DriveError> {{ match value {{ SlotValue::Bool(value) => Ok(value), other => Err(DriveError::TypeMismatch {{ expected: \"boolean\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn expect_i64_value(value: SlotValue) -> Result<i64, DriveError> {{ match value {{ SlotValue::I64(value) => Ok(value), other => Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn read_retry_state_from_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16, max_attempts: u16) -> Result<RetryState, DriveError> {{ match read_slot(slots, slot)? {{ SlotValue::I64(raw) => RetryState::decode(raw, max_attempts), other => Err(DriveError::TypeMismatch {{ expected: \"number\", found: other.type_name() }}), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn retry_check_target(current_attempt: u16, max_attempts: u16, body: u16, exhausted: u16) -> Result<StepOutcome, DriveError> {{ if max_attempts == 0 {{ return Err(DriveError::InvalidRetryPolicy); }} if current_attempt < max_attempts {{ Ok(StepOutcome::Continue(body)) }} else {{ Ok(StepOutcome::Continue(exhausted)) }} }}").map_err(fmt_err)?;
    writeln!(out, "fn list_item_count(list_store: &ListStore, handle: u32) -> Result<u32, DriveError> {{ match list_store.len(handle)? {{ Some(len) => Ok(len), None => Err(DriveError::InvalidListHandle), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn first_list_item(list_store: &ListStore, handle: u32, count: u32) -> Result<SlotValue, DriveError> {{ if count == 0 {{ return Err(DriveError::InvalidListHandle); }} match list_store.first(handle)? {{ Some(value) => Ok(value), None => Err(DriveError::InvalidListHandle), }} }}").map_err(fmt_err)?;
    writeln!(out, "fn tail_list_handle(list_store: &mut ListStore, handle: u32) -> Result<u32, DriveError> {{ match list_store.tail(handle)? {{ Some(tail) => Ok(tail), None => Err(DriveError::InvalidListHandle), }} }}").map_err(fmt_err)?;
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
            Some(&_) => {
                return Err(CodegenError::UnsupportedIr {
                    feature: "ConstValue",
                });
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

/// Emit code to evaluate an accessor by reading the root slot and traversing handles.
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
        writeln!(out, "    stack.push_tainted(read_slot(slots, {root_slot})?, read_taint(slot_taints, {root_slot})?)?;").map_err(fmt_err)?;
    } else {
        emit_accessor_traversal(out, root_slot, &accessor.path)?;
    }
    Ok(())
}

fn emit_accessor_traversal(
    out: &mut String,
    root_slot: u16,
    path: &[vb_core::PathSegment],
) -> CodegenResult<()> {
    let depth = u16::try_from(path.len()).map_err(|_| CodegenError::SemanticMismatch {
        detail: "accessor path depth exceeds generated range".into(),
    })?;
    writeln!(out, "    {{").map_err(fmt_err)?;
    writeln!(
        out,
        "        let mut _current = read_slot(slots, {root_slot})?;"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "        let mut _taint = read_taint(slot_taints, {root_slot})?;"
    )
    .map_err(fmt_err)?;
    writeln!(out, "        if {depth}u16 > ACCESSOR_MAX_PATH_DEPTH {{ return Err(DriveError::AccessorPathTooDeep {{ depth: {depth}u16, max: ACCESSOR_MAX_PATH_DEPTH }}); }}").map_err(fmt_err)?;
    for segment in path {
        emit_accessor_segment(out, *segment)?;
    }
    writeln!(out, "        stack.push_tainted(_current, _taint)?;").map_err(fmt_err)?;
    writeln!(out, "    }}").map_err(fmt_err)
}

fn emit_accessor_segment(out: &mut String, segment: vb_core::PathSegment) -> CodegenResult<()> {
    match segment {
        vb_core::PathSegment::Field(field) => writeln!(out, "        {{ let (_value, _segment_taint) = match _current {{ SlotValue::Object(_object) => object_store.field(_object, {})?, other => return Err(DriveError::TypeMismatch {{ expected: \"object\", found: other.type_name() }}), }}; _taint = join_taint(_taint, _segment_taint); _current = _value; }}", field.get()).map_err(fmt_err),
        vb_core::PathSegment::Index(index) => writeln!(out, "        {{ let (_value, _segment_taint) = match _current {{ SlotValue::List(_list) => list_store.value_at(_list, {index})?, other => return Err(DriveError::TypeMismatch {{ expected: \"list\", found: other.type_name() }}), }}; _taint = join_taint(_taint, _segment_taint); _current = _value; }}").map_err(fmt_err),
        _ => Err(CodegenError::UnsupportedIr {
            feature: "PathSegment",
        }),
    }
}

fn fmt_err(_: std::fmt::Error) -> CodegenError {
    CodegenError::FormatBufferOverflow
}

mod proptests;
#[cfg(not(miri))]
mod tests;
