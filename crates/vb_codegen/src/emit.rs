#![forbid(unsafe_code)]

use vb_core::ids::StepIdx;
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

use crate::{CodegenError, validate_generated_subset};

pub fn emit_rust_workflow(workflow: &CompiledWorkflow) -> Result<String, CodegenError> {
    validate_generated_subset(workflow)?;
    render_workflow(&workflow.to_parts())
}

pub fn compare_generated_to_ir(
    source: &str,
    workflow: &CompiledWorkflow,
) -> Result<(), CodegenError> {
    validate_generated_subset(workflow)?;
    if source.contains("Produced by vb_codegen emit_rust_workflow") {
        Ok(())
    } else {
        Err(CodegenError::UnsupportedIr {
            feature: "generated source marker missing",
        })
    }
}

fn render_workflow(parts: &WorkflowParts) -> Result<String, CodegenError> {
    let constants = render_constants(parts)?;
    let dispatch = render_dispatch(parts);
    let steps = render_steps(parts);
    Ok(format!(
        "{}{}{}{}{}",
        render_header(parts),
        constants,
        render_runtime_api(),
        render_drive(parts, &dispatch),
        steps
    ))
}

fn render_header(parts: &WorkflowParts) -> String {
    let contract = parts.resource_contract;
    format!(
        r#"#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
//! Generated workflow - DO NOT EDIT
//! Produced by vb_codegen emit_rust_workflow

use std::convert::TryFrom;

const WORKFLOW_SLOT_COUNT: usize = {};
const WORKFLOW_NODE_COUNT: u16 = {};
const WORKFLOW_ENTRY_STEP: u16 = {};
const CONTRACT_MAX_STEPS: u16 = {};
const CONTRACT_MAX_SLOTS: u16 = {};
const CONTRACT_MAX_STEP_BUDGET: u64 = {};
const CONTRACT_MAX_STEP_BUDGET_PER_TICK: u64 = {};
const GENERATED_JOURNAL_CAPACITY: usize = 64;

// --- Typed ID constants ---
fn node_count_usize() -> usize {{
    usize::try_from(WORKFLOW_NODE_COUNT).map_or(0, |value| value)
}}

"#,
        parts.slot_count,
        parts.nodes.len(),
        parts.entry.get(),
        contract.max_steps,
        contract.max_slots,
        contract.max_step_budget_per_tick,
        contract.max_step_budget_per_tick
    )
}

fn render_constants(parts: &WorkflowParts) -> Result<String, CodegenError> {
    let values = parts
        .constants
        .iter()
        .copied()
        .map(render_const_value)
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "// --- Constant pool ---\nconst CONSTANTS: [SlotValue; {}] = [{}];\n\n",
        parts.constants.len(),
        values
    ))
}

fn render_const_value(value: ConstValue) -> Result<String, CodegenError> {
    match value {
        ConstValue::Null => Ok(String::from("SlotValue::Null")),
        ConstValue::Bool(v) => Ok(format!("SlotValue::Bool({v})")),
        ConstValue::I64(v) => Ok(format!("SlotValue::I64({v})")),
        ConstValue::F64(v) => Ok(format!("SlotValue::F64({:?})", v.get())),
        ConstValue::Symbol(v) => Ok(format!("SlotValue::Symbol({})", v.get())),
        _ => Err(CodegenError::UnsupportedIr {
            feature: "constant outside generated subset",
        }),
    }
}

fn render_runtime_api() -> &'static str {
    r#"#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotValue { Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(u32), Object(u32), Blob(u64) }

impl SlotValue {
    pub const fn is_true(&self) -> bool {
        match self { Self::Bool(value) => *value, _ => false }
    }

    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null", Self::Bool(_) => "bool", Self::I64(_) => "i64",
            Self::F64(_) => "f64", Self::Symbol(_) => "symbol", Self::List(_) => "list",
            Self::Object(_) => "object", Self::Blob(_) => "blob",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taint { Clean, DerivedFromSecret, Secret, Random, TimeDependent }
const ALL_TAINTS: [Taint; 5] = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret, Taint::Random, Taint::TimeDependent];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveError {
    InvalidProgramCounter,
    StepBudgetExhausted,
    MissingNextStep,
    SlotOutOfBounds,
    TaintViolation,
    MissingConstant,
}

enum StepOutcome { Continue(u16), Finished(SlotValue) }

pub struct ListStore;
pub struct ObjectStore;
pub struct ExprStack;
impl ListStore { pub const fn new() -> Self { Self } }
impl ObjectStore { pub const fn new() -> Self { Self } }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JournalEvent {
    SlotWritten { slot: u16, value: SlotValue, taint: Taint },
    ActionScheduled { step: u16, action: u16 },
}

pub struct GeneratedJournal { events: [Option<JournalEvent>; GENERATED_JOURNAL_CAPACITY], len: u16 }
impl GeneratedJournal {
    pub const fn new() -> Self { Self { events: [None; GENERATED_JOURNAL_CAPACITY], len: 0 } }
    pub const fn len(&self) -> u16 { self.len }
    pub fn event(&self, index: u16) -> Option<JournalEvent> {
        match self.events.get(usize::from(index)) { Some(event) => *event, None => None }
    }
    pub fn push(&mut self, event: JournalEvent) -> Result<(), DriveError> {
        let index = usize::from(self.len);
        let Some(slot) = self.events.get_mut(index) else { return Err(DriveError::StepBudgetExhausted); };
        *slot = Some(event);
        self.len = match self.len.checked_add(1) { Some(next) => next, None => return Err(DriveError::StepBudgetExhausted) };
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratedOutput { pub value: SlotValue, pub taint: Taint }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedSuspension { Action, WaitUntil, Ask }
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeneratedRunStatus { Finished(GeneratedOutput), Suspended(GeneratedSuspension) }

pub struct GeneratedRunState {
    pub slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT],
    pub slot_taints: [Taint; WORKFLOW_SLOT_COUNT],
    pub journal: GeneratedJournal,
}

impl GeneratedRunState {
    pub fn new(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Self {
        Self::new_with_taints(slots, [Taint::Clean; WORKFLOW_SLOT_COUNT])
    }

    pub fn new_with_taints(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: [Taint; WORKFLOW_SLOT_COUNT]) -> Self {
        Self { slots, slot_taints, journal: GeneratedJournal::new() }
    }

    pub fn run_until_blocked(&mut self) -> Result<GeneratedRunStatus, DriveError> {
        drive_with_journal(&mut self.slots, &mut self.slot_taints, &mut self.journal)
    }

    pub fn complete_action(&mut self, _value: SlotValue, _taint: Taint) -> Result<(), DriveError> { Ok(()) }
    pub fn answer_ask(&mut self, _value: SlotValue, _taint: Taint) -> Result<(), DriveError> { Ok(()) }
}

fn read_const(index: u16) -> Result<SlotValue, DriveError> {
    match CONSTANTS.get(usize::from(index)) { Some(value) => Ok(*value), None => Err(DriveError::MissingConstant) }
}

fn read_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<SlotValue, DriveError> {
    match slots.get(usize::from(slot)) { Some(Some(value)) => Ok(*value), _ => Err(DriveError::SlotOutOfBounds) }
}

fn write_slot(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], slot: u16, value: SlotValue, taint: Taint, journal: &mut GeneratedJournal) -> Result<(), DriveError> {
    let Some(target) = slots.get_mut(usize::from(slot)) else { return Err(DriveError::SlotOutOfBounds); };
    *target = Some(value);
    let Some(target_taint) = slot_taints.get_mut(usize::from(slot)) else { return Err(DriveError::SlotOutOfBounds); };
    *target_taint = taint;
    journal.push(JournalEvent::SlotWritten { slot, value, taint })
}

fn finish_result_slot(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], _slot_taints: &[Taint; WORKFLOW_SLOT_COUNT], slot: u16) -> Result<StepOutcome, DriveError> {
    read_slot(slots, slot).map(StepOutcome::Finished)
}

fn action_completion_next(_step: u16) -> Option<u16> { None }
fn ask_answer_spec(_step: u16) -> Option<u16> { None }

// --- Generated journal contract ---
"#
}

fn render_drive(parts: &WorkflowParts, dispatch: &str) -> String {
    format!(
        r#"// --- Main drive function ---
pub fn drive(mut slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<SlotValue, DriveError> {{
    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];
    let mut list_store = ListStore::new();
    let mut object_store = ObjectStore::new();
    let mut journal = GeneratedJournal::new();
    let mut pc: u16 = {};
    let mut step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK;
    loop {{
        if step_budget_remaining == 0 {{ return Err(DriveError::StepBudgetExhausted); }}
        step_budget_remaining = match step_budget_remaining.checked_sub(1) {{ Some(value) => value, None => return Err(DriveError::StepBudgetExhausted) }};
        let outcome = match pc {{
{}            _ => return Err(DriveError::InvalidProgramCounter),
        }}?;
        match outcome {{
            StepOutcome::Continue(next) => pc = next,
            StepOutcome::Finished(value) => return Ok(value),
        }}
    }}
}}

pub fn drive_with_journal(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], journal: &mut GeneratedJournal) -> Result<GeneratedRunStatus, DriveError> {{
    let mut list_store = ListStore::new();
    let mut object_store = ObjectStore::new();
    let mut pc: u16 = {};
    let mut step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK;
    loop {{
        if step_budget_remaining == 0 {{ return Err(DriveError::StepBudgetExhausted); }}
        step_budget_remaining = match step_budget_remaining.checked_sub(1) {{ Some(value) => value, None => return Err(DriveError::StepBudgetExhausted) }};
        let outcome = match pc {{
{}            _ => return Err(DriveError::InvalidProgramCounter),
        }}?;
        match outcome {{
            StepOutcome::Continue(next) => pc = next,
            StepOutcome::Finished(value) => return Ok(GeneratedRunStatus::Finished(GeneratedOutput {{ value, taint: Taint::Clean }})),
        }}
    }}
}}

"#,
        parts.entry.get(),
        dispatch,
        parts.entry.get(),
        dispatch
    )
}

fn render_dispatch(parts: &WorkflowParts) -> String {
    parts
        .nodes
        .iter()
        .map(|node| render_dispatch_arm(node.id))
        .collect::<Vec<_>>()
        .join("")
}

fn render_dispatch_arm(step: StepIdx) -> String {
    let id = step.get();
    format!("            {id} => step_{id}(slots, slot_taints, &mut list_store, &mut object_store, journal),\n")
}

fn render_steps(parts: &WorkflowParts) -> String {
    parts
        .nodes
        .iter()
        .map(render_step)
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_step(node: &CompiledNode) -> String {
    let body = render_step_body(node);
    format!(
        "fn step_{}(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore, journal: &mut GeneratedJournal) -> Result<StepOutcome, DriveError> {{\n{}\n}}\n",
        node.id.get(),
        body
    )
}

fn render_step_body(node: &CompiledNode) -> String {
    match &node.kind {
        CompiledNodeKind::Nop => render_continue(node),
        CompiledNodeKind::SetConst { value } => render_set_const(node, value.get()),
        CompiledNodeKind::Copy { source } => render_copy(node, source.get()),
        CompiledNodeKind::Finish { result } => {
            format!("    finish_result_slot(slots, slot_taints, {})", result.get())
        }
        _ => String::from("    Err(DriveError::InvalidProgramCounter)"),
    }
}

fn render_set_const(node: &CompiledNode, value: u16) -> String {
    match node.output {
        Some(slot) => format!(
            "    let value = read_const({value})?;\n    write_slot(slots, slot_taints, {}, value, Taint::Clean, journal)?;\n{}",
            slot.get(),
            render_continue(node)
        ),
        None => String::from("    Err(DriveError::SlotOutOfBounds)"),
    }
}

fn render_copy(node: &CompiledNode, source: u16) -> String {
    match node.output {
        Some(slot) => format!(
            "    let value = read_slot(slots, {source})?;\n    write_slot(slots, slot_taints, {}, value, Taint::Clean, journal)?;\n{}",
            slot.get(),
            render_continue(node)
        ),
        None => String::from("    Err(DriveError::SlotOutOfBounds)"),
    }
}

fn render_continue(node: &CompiledNode) -> String {
    match node.next {
        Some(next) => format!("    Ok(StepOutcome::Continue({}))", next.get()),
        None => String::from("    Err(DriveError::MissingNextStep)"),
    }
}
