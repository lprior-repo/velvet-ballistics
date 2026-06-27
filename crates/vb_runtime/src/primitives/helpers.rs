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
            reason: "iterator state cursor exceeds i64 range",
        })?;
    Ok(vec![SlotValue::I64(source_token), SlotValue::I64(cursor_token)].into_boxed_slice())
}

/// Decodes a 2-element iterator state list into `(source_id, cursor)`.
/// Returns an internal-invariant error on a malformed state.
pub(crate) fn decode_iterator_state(items: &[SlotValue]) -> Result<(ListId, usize), EngineError> {
    if items.len() != 2 {
        return Err(EngineError::InternalInvariantViolation {
            reason: "iterator state must be a 2-element list",
        });
    }
    let source_token = match items.first().copied() {
        Some(SlotValue::I64(v)) => v,
        Some(_) => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state source token must be I64",
            });
        }
        None => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state must be a 2-element list",
            });
        }
    };
    let cursor_token = match items.get(1).copied() {
        Some(SlotValue::I64(v)) => v,
        Some(_) => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state cursor must be I64",
            });
        }
        None => {
            return Err(EngineError::InternalInvariantViolation {
                reason: "iterator state must be a 2-element list",
            });
        }
    };
    let source_id_value =
        u32::try_from(source_token).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "iterator state source token out of u32 range",
        })?;
    let cursor_value =
        usize::try_from(cursor_token).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "iterator state cursor out of usize range",
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
mod tests {
    use super::*;
    use vb_core::value_store::ValueStore;

    fn ensure(condition: bool, message: impl Into<String>) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.into())
        }
    }

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(4, 8)
    }

    // ── expect_list tests ──────────────────────────────────────────────

    #[test]
    fn expect_list_returns_list_id_for_list_value() -> Result<(), String> {
        let mut store = ValueStore::new();
        let items: Box<[SlotValue]> = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
        let id = store
            .insert_list(items)
            .map_err(|e| format!("insert_list failed: {e:?}"))?;
        let result =
            expect_list(SlotValue::List(id)).map_err(|e| format!("expect_list failed: {e:?}"))?;
        ensure(result == id, "expected list id to match")
    }

    #[test]
    fn expect_list_returns_error_for_i64() -> Result<(), String> {
        let result = expect_list(SlotValue::I64(42));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(
                    expected == "list",
                    format!("expected 'list', got '{expected}'"),
                )?;
                ensure(
                    found == "number",
                    format!("expected 'number', got '{found}'"),
                )
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_bool() -> Result<(), String> {
        let result = expect_list(SlotValue::Bool(true));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(
                    expected == "list",
                    format!("expected 'list', got '{expected}'"),
                )?;
                ensure(
                    found == "boolean",
                    format!("expected 'boolean', got '{found}'"),
                )
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_null() -> Result<(), String> {
        let result = expect_list(SlotValue::Null);
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(
                    expected == "list",
                    format!("expected 'list', got '{expected}'"),
                )?;
                ensure(found == "null", format!("expected 'null', got '{found}'"))
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_symbol() -> Result<(), String> {
        let result = expect_list(SlotValue::Symbol(vb_core::ids::SymbolId::new(1)));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(
                    expected == "list",
                    format!("expected 'list', got '{expected}'"),
                )?;
                ensure(
                    found == "symbol",
                    format!("expected 'symbol', got '{found}'"),
                )
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_f64() -> Result<(), String> {
        let f64_val = vb_core::value::FiniteF64::new(3.14).map_err(|e| format!("{e}"))?;
        let result = expect_list(SlotValue::F64(f64_val));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(
                    expected == "list",
                    format!("expected 'list', got '{expected}'"),
                )?;
                ensure(
                    found == "number",
                    format!("expected 'number', got '{found}'"),
                )
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    // ── empty_list tests ───────────────────────────────────────────────

    #[test]
    fn empty_list_produces_zero_length_slice() -> Result<(), String> {
        let list = empty_list();
        ensure(
            list.len() == 0,
            format!("expected empty, got len {}", list.len()),
        )
    }

    #[test]
    fn empty_list_is_boxed_slice() -> Result<(), String> {
        let list = empty_list();
        ensure(list.is_empty(), "expected is_empty")
    }

    // ── jump_to tests ──────────────────────────────────────────────────

    #[test]
    fn jump_to_sets_pc_to_target() -> Result<(), String> {
        let mut run = fresh_frame();
        let target = StepIdx::new(3);
        let signal = jump_to(&mut run, target).map_err(|e| format!("jump_to failed: {e:?}"))?;
        ensure(
            signal == vb_core::EngineSignal::Continue,
            "expected Continue signal",
        )?;
        ensure(
            run.pc() == target,
            format!("expected pc={target:?}, got {:?}", run.pc()),
        )
    }

    #[test]
    fn jump_to_increments_executed_counter() -> Result<(), String> {
        let mut run = fresh_frame();
        let before = run.executed();
        jump_to(&mut run, StepIdx::new(1)).map_err(|e| format!("jump_to failed: {e:?}"))?;
        ensure(
            run.executed() == before.saturating_add(1),
            format!(
                "expected {}, got {}",
                before.saturating_add(1),
                run.executed()
            ),
        )
    }

    #[test]
    fn jump_to_returns_continue_signal() -> Result<(), String> {
        let mut run = fresh_frame();
        let signal =
            jump_to(&mut run, StepIdx::new(1)).map_err(|e| format!("jump_to failed: {e:?}"))?;
        ensure(
            matches!(signal, vb_core::EngineSignal::Continue),
            "expected Continue",
        )
    }

    // ── jump_to_next tests ─────────────────────────────────────────────

    #[test]
    fn jump_to_next_with_valid_next_succeeds() -> Result<(), String> {
        let mut run = fresh_frame();
        let next = StepIdx::new(2);
        let signal = jump_to_next(&mut run, Some(next), StepIdx::ZERO)
            .map_err(|e| format!("jump_to_next failed: {e:?}"))?;
        ensure(
            run.pc() == next,
            format!("expected pc={next:?}, got {:?}", run.pc()),
        )?;
        ensure(
            signal == vb_core::EngineSignal::Continue,
            "expected Continue signal",
        )
    }

    #[test]
    fn jump_to_next_without_next_returns_error() -> Result<(), String> {
        let mut run = fresh_frame();
        let result = jump_to_next(&mut run, None, StepIdx::ZERO);
        match result {
            Err(EngineError::MissingNextStep { step }) => ensure(
                step == StepIdx::ZERO,
                format!("expected ZERO, got {step:?}"),
            ),
            other => Err(format!("expected MissingNextStep, got {other:?}")),
        }
    }

    #[test]
    fn jump_to_next_increments_executed() -> Result<(), String> {
        let mut run = fresh_frame();
        let before = run.executed();
        jump_to_next(&mut run, Some(StepIdx::new(1)), StepIdx::ZERO)
            .map_err(|e| format!("jump_to_next failed: {e:?}"))?;
        ensure(
            run.executed() == before.saturating_add(1),
            format!(
                "expected {}, got {}",
                before.saturating_add(1),
                run.executed()
            ),
        )
    }

    #[test]
    fn jump_to_next_reports_correct_step_on_missing() -> Result<(), String> {
        let step = StepIdx::new(7);
        let mut run = fresh_frame();
        let result = jump_to_next(&mut run, None, step);
        match result {
            Err(EngineError::MissingNextStep { step: s }) => {
                ensure(s == step, format!("expected step={step:?}, got {s:?}"))
            }
            other => Err(format!("expected MissingNextStep, got {other:?}")),
        }
    }

    // ── require_output tests ───────────────────────────────────────────

    #[test]
    fn require_output_returns_slot_when_some() -> Result<(), String> {
        let slot = SlotIdx::new(3);
        let result = require_output(Some(slot), StepIdx::ZERO).map_err(|e| format!("{e:?}"))?;
        ensure(result == slot, format!("expected {slot:?}, got {result:?}"))
    }

    #[test]
    fn require_output_returns_error_when_none() -> Result<(), String> {
        let step = StepIdx::new(5);
        let result = require_output(None, step);
        match result {
            Err(EngineError::MissingOutputSlot { step: s }) => {
                ensure(s == step, format!("expected step={step:?}, got {s:?}"))
            }
            other => Err(format!("expected MissingOutputSlot, got {other:?}")),
        }
    }

    #[test]
    fn require_output_with_zero_step() -> Result<(), String> {
        let slot = SlotIdx::new(0);
        let result = require_output(Some(slot), StepIdx::ZERO).map_err(|e| format!("{e:?}"))?;
        ensure(result == slot, "slot should be returned for zero step")
    }

    // ── jump_to_body tests ──────────────────────────────────────────────

    #[test]
    fn tc001_jump_to_body_succeeded_to_pending() -> Result<(), String> {
        let mut run = fresh_frame();
        let body = StepIdx::new(1);
        run.mark_succeeded(body).map_err(|e| format!("{e:?}"))?;
        let before_exec = run.executed();
        let result =
            jump_to_body(&mut run, body).map_err(|e| format!("jump_to_body failed: {e:?}"))?;
        ensure(
            result == vb_core::EngineSignal::Continue,
            "expected Continue signal",
        )?;
        ensure(
            run.pc() == body,
            format!("expected pc={body:?}, got {:?}", run.pc()),
        )?;
        ensure(
            run.executed() == before_exec.saturating_add(1),
            "executed should increment",
        )?;
        let state = run.step_state(body).map_err(|e| format!("{e:?}"))?;
        ensure(
            matches!(state, vb_core::frame::StepState::Pending),
            format!("expected Pending, got {state:?}"),
        )?;
        Ok(())
    }

    #[test]
    fn tc002_jump_to_body_pending_idempotent() -> Result<(), String> {
        let mut run = fresh_frame();
        let body = StepIdx::new(1);
        run.mark_pending(body).map_err(|e| format!("{e:?}"))?;
        let result =
            jump_to_body(&mut run, body).map_err(|e| format!("jump_to_body failed: {e:?}"))?;
        ensure(
            result == vb_core::EngineSignal::Continue,
            "expected Continue",
        )?;
        let state = run.step_state(body).map_err(|e| format!("{e:?}"))?;
        ensure(
            matches!(state, vb_core::frame::StepState::Pending),
            format!("expected Pending, got {state:?}"),
        )
    }

    #[test]
    fn tc003_jump_to_body_succeeded_also_idempotent() -> Result<(), String> {
        let mut run = fresh_frame();
        let body = StepIdx::new(1);
        run.mark_succeeded(body).map_err(|e| format!("{e:?}"))?;
        let result =
            jump_to_body(&mut run, body).map_err(|e| format!("jump_to_body failed: {e:?}"))?;
        ensure(
            result == vb_core::EngineSignal::Continue,
            "expected Continue",
        )?;
        let state = run.step_state(body).map_err(|e| format!("{e:?}"))?;
        ensure(
            matches!(state, vb_core::frame::StepState::Pending),
            format!("expected Pending (Succeeded→Pending), got {state:?}"),
        )
    }

    #[test]
    fn tc004_jump_to_body_waiting_reentry_valid() -> Result<(), String> {
        let mut run = fresh_frame();
        let body = StepIdx::new(1);
        run.mark_running(body).map_err(|e| format!("{e:?}"))?;
        run.mark_waiting(body).map_err(|e| format!("{e:?}"))?;
        let result =
            jump_to_body(&mut run, body).map_err(|e| format!("jump_to_body failed: {e:?}"))?;
        ensure(
            result == vb_core::EngineSignal::Continue,
            "expected Continue",
        )?;
        let state = run.step_state(body).map_err(|e| format!("{e:?}"))?;
        ensure(
            matches!(state, vb_core::frame::StepState::Waiting),
            format!("expected Waiting (unchanged), got {state:?}"),
        )
    }

    #[test]
    fn tc005_jump_to_body_asking_reentry_valid() -> Result<(), String> {
        let mut run = fresh_frame();
        let body = StepIdx::new(1);
        run.mark_running(body).map_err(|e| format!("{e:?}"))?;
        run.mark_asking(body).map_err(|e| format!("{e:?}"))?;
        let result =
            jump_to_body(&mut run, body).map_err(|e| format!("jump_to_body failed: {e:?}"))?;
        ensure(
            result == vb_core::EngineSignal::Continue,
            "expected Continue",
        )?;
        let state = run.step_state(body).map_err(|e| format!("{e:?}"))?;
        ensure(
            matches!(state, vb_core::frame::StepState::Asking),
            format!("expected Asking (unchanged), got {state:?}"),
        )
    }
}
