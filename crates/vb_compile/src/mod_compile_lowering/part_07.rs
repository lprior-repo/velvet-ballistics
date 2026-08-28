#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, ConstValue, ExprIdx, ExprProgram, InputSlotKind, ResourceContract, SlotBranch,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

/// Lowers a `repeat` primitive into retry IR nodes.
pub fn lower_repeat(
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let attempt_slot = slot_idx_for_step(
        id.as_usize()
            .checked_add(1)
            .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?,
    )?;
    builder.record_slot(attempt_slot);
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: Some(attempt_slot),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: attempt_slot,
            },
        },
    ])
}

/// Type-safe discriminator for the two legal `wait` shapes.
///
/// Replaces the previous `is_event: bool` parameter, which allowed invalid
/// combinations such as passing `is_event = false` with a `timeout_slot`,
/// which would be silently discarded.
#[non_exhaustive]
pub enum WaitKind {
    /// `wait.until` — waits until a deadline slot is reached; no timeout.
    Until { deadline: SlotIdx },
    /// `wait.event` — waits for an event slot, with an optional timeout.
    Event {
        event: SlotIdx,
        timeout: Option<SlotIdx>,
    },
}

/// Lowers a `wait` primitive into `WaitUntil` or `WaitEvent` IR nodes.
pub fn lower_wait(id: StepIdx, kind: WaitKind, builder: &mut SlotCompiler) -> CompiledNode {
    let compiled_kind = match kind {
        WaitKind::Until { deadline } => {
            builder.record_slot(deadline);
            CompiledNodeKind::WaitUntil {
                deadline_slot: deadline,
            }
        }
        WaitKind::Event { event, timeout } => {
            builder.record_slot(event);
            if let Some(slot) = timeout {
                builder.record_slot(slot);
            }
            CompiledNodeKind::WaitEvent {
                event,
                timeout_slot: timeout,
            }
        }
    };
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: compiled_kind,
    }
}

/// Lowers an `ask` primitive into `Ask` and `AskResume` IR nodes.
pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(prompt);
    builder.record_slot(answer);
    let resume = id
        .checked_add(1)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "ask",
            field: "resume_step",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })?;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            },
        },
        CompiledNode {
            id: resume,
            output: Some(answer),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::AskResume { answer },
        },
    ])
}

/// Lowers a `finish` primitive into a terminal `Finish` node.
pub fn lower_finish(id: StepIdx, result: SlotIdx, builder: &mut SlotCompiler) -> CompiledNode {
    builder.record_slot(result);
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Finish { result },
    }
}

/// Validates compiled workflow IR against structural and resource invariants.
///
/// Runs the shared validation pipeline (gates 7-15) via
/// [`vb_validate::shared::validate`], then delegates to
/// [`CompiledWorkflow::try_from_parts`] for core structural and budget checks.
///
/// Returns the specific validation error so callers can distinguish gate
/// failures from structural errors.
pub fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, CompileErrors> {
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

/// Mutable slot compiler state for building node arrays.
///
/// Tracks slot allocation, constant pool, expression programs, accessor
/// programs, and input slot metadata during step lowering.
#[derive(Debug, Default)]
pub struct SlotCompiler {
    pub(super) nodes: Vec<CompiledNode>,
    pub(super) constants: Vec<ConstValue>,
    pub(super) expressions: Vec<ExprProgram>,
    pub(super) accessors: Vec<AccessorProgram>,
    pub(super) max_slot: Option<usize>,
    pub(super) input_slots: Vec<CompiledInputSlot>,
}
