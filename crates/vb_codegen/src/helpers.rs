//! Helper functions for code generation.

use std::fmt::Write;
use crate::{fmt_err, CodegenResult};
use vb_core::CompiledWorkflow;

pub fn write_header(out: &mut String) -> CodegenResult<()> {
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
    writeln!(out, "    WaitSuspend {{ deadline_slot: u16 }},").map_err(fmt_err)?;
    writeln!(out, "    AskSuspend {{ prompt_slot: u16 }},").map_err(fmt_err)?;
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
    Ok(())
}

pub fn write_next_or_error(out: &mut String, next: Option<vb_core::StepIdx>) -> CodegenResult<()> {
    match next {
        Some(target) => {
            writeln!(out, "    Ok(StepOutcome::Continue({}))", target.get()).map_err(fmt_err)
        }
        None => writeln!(out, "    Err(DriveError::MissingNextStep)").map_err(fmt_err),
    }
}

pub fn emit_unsupported_step(out: &mut String, primitive: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    Err(DriveError::UnsupportedPrimitive {{ primitive: \"{primitive}\" }})"
    )
    .map_err(fmt_err)
}

pub fn emit_unsupported_expr(out: &mut String, op: &'static str) -> CodegenResult<()> {
    writeln!(
        out,
        "    return Err(DriveError::UnsupportedExpressionOp {{ op: \"{op}\" }});"
    )
    .map_err(fmt_err)
}

/// Emit code to evaluate an accessor by reading the root slot.
/// For empty-path accessors, this simply reads the root slot value.
/// For non-empty paths, this emits a typed error matching the runtime engine behavior.
pub fn emit_accessor_eval(
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
