#![forbid(unsafe_code)]
//! Shared helper functions for primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

pub(crate) fn expect_list(value: SlotValue) -> Result<ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

pub(crate) fn empty_list() -> Box<[SlotValue]> {
    Vec::<SlotValue>::new().into_boxed_slice()
}

/// Builds a 2-element iterator state list encoding `(source_id, cursor)`.
/// This is the compact alternative to materializing the tail of the source
/// list on every iteration step. Both `source_id` and `cursor` are stored
/// as `SlotValue::I64`; the state list is immutable once inserted.
pub(crate) fn build_iterator_state(
    source_id: ListId,
    cursor: usize,
) -> Result<Box<[SlotValue]>, EngineError> {
    let source_token = i64::from(source_id.get());
    let cursor_token =
        i64::try_from(cursor).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "iterator state cursor exceeds i64 range".into(),
        })?;
    Ok(vec![SlotValue::I64(source_token), SlotValue::I64(cursor_token)].into_boxed_slice())
}

/// Decodes a 2-element iterator state list into `(source_id, cursor)`.
/// Returns an internal-invariant error on a malformed state.
pub(crate) fn decode_iterator_state(items: &[SlotValue]) -> Result<(ListId, usize), EngineError> {
    if items.len() != 2 {
        return Err(EngineError::InternalInvariantViolation {
            reason: "iterator state must be a 2-element list".into(),
        });
    }
    let source_token = match items.first().copied() {
        Some(SlotValue::I64(v)) => v,
        Some(_) => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state source token must be I64".into(),
            });
        }
        None => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state must be a 2-element list".into(),
            });
        }
    };
    let cursor_token = match items.get(1).copied() {
        Some(SlotValue::I64(v)) => v,
        Some(_) => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state cursor must be I64".into(),
            });
        }
        None => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state must be a 2-element list".into(),
            });
        }
    };
    let source_id_value =
        u32::try_from(source_token).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "iterator state source token out of u32 range".into(),
        })?;
    let cursor_value =
        usize::try_from(cursor_token).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "iterator state cursor out of usize range".into(),
        })?;
    Ok((ListId::new(source_id_value), cursor_value))
}

pub(crate) fn jump_to(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current = run.step_state(body)?;
    if current == vb_core::frame::StepState::Succeeded {
        run.mark_pending(body)?;
    }
    jump_to(run, body)
}

pub(crate) fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = require_next_step(run, next, step)?;
    jump_to(run, target)
}

pub(crate) fn require_next_step(
    run: &RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<StepIdx, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    if target.as_usize() >= usize::from(run.step_count()) {
        return Err(EngineError::InvalidProgramCounter { step: target });
    }
    Ok(target)
}

pub(crate) fn require_output(
    output: Option<SlotIdx>,
    step: StepIdx,
) -> Result<SlotIdx, EngineError> {
    output.ok_or(EngineError::MissingOutputSlot { step })
}

#[cfg(test)]
mod tests;
