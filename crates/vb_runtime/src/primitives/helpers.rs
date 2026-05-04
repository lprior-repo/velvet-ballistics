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

pub(crate) fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    if items.len() <= 1 {
        return Ok(empty_list());
    }
    let tail_len = items
        .len()
        .checked_sub(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "tail_items length checked nonempty",
        })?;
    let mut tail = Vec::with_capacity(tail_len);
    let mut index = 1usize;
    while index < items.len() {
        let value = *items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "tail_items index checked",
            })?;
        tail.push(value);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "tail_items index overflow",
            })?;
    }
    Ok(tail.into_boxed_slice())
}

pub(crate) fn jump_to(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

pub(crate) fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
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
        let id = store.insert_list(items).map_err(|e| format!("insert_list failed: {e:?}"))?;
        let result = expect_list(SlotValue::List(id)).map_err(|e| format!("expect_list failed: {e:?}"))?;
        ensure(result == id, "expected list id to match")
    }

    #[test]
    fn expect_list_returns_error_for_i64() -> Result<(), String> {
        let result = expect_list(SlotValue::I64(42));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(expected == "list", format!("expected 'list', got '{expected}'"))?;
                ensure(found == "number", format!("expected 'number', got '{found}'"))
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_bool() -> Result<(), String> {
        let result = expect_list(SlotValue::Bool(true));
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(expected == "list", format!("expected 'list', got '{expected}'"))?;
                ensure(found == "boolean", format!("expected 'boolean', got '{found}'"))
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    #[test]
    fn expect_list_returns_error_for_null() -> Result<(), String> {
        let result = expect_list(SlotValue::Null);
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                ensure(expected == "list", format!("expected 'list', got '{expected}'"))?;
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
                ensure(expected == "list", format!("expected 'list', got '{expected}'"))?;
                ensure(found == "symbol", format!("expected 'symbol', got '{found}'"))
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
                ensure(expected == "list", format!("expected 'list', got '{expected}'"))?;
                ensure(found == "number", format!("expected 'number', got '{found}'"))
            }
            other => Err(format!("expected TypeMismatch, got {other:?}")),
        }
    }

    // ── empty_list tests ───────────────────────────────────────────────

    #[test]
    fn empty_list_produces_zero_length_slice() -> Result<(), String> {
        let list = empty_list();
        ensure(list.len() == 0, format!("expected empty, got len {}", list.len()))
    }

    #[test]
    fn empty_list_is_boxed_slice() -> Result<(), String> {
        let list = empty_list();
        ensure(list.is_empty(), "expected is_empty")
    }

    // ── tail_items tests ───────────────────────────────────────────────

    #[test]
    fn tail_items_empty_input_returns_empty() -> Result<(), String> {
        let items: Box<[SlotValue]> = Box::new([]);
        let tail = tail_items(&items).map_err(|e| format!("tail_items failed: {e:?}"))?;
        ensure(tail.is_empty(), "expected empty tail for empty input")
    }

    #[test]
    fn tail_items_single_item_returns_empty() -> Result<(), String> {
        let items: Box<[SlotValue]> = vec![SlotValue::I64(42)].into_boxed_slice();
        let tail = tail_items(&items).map_err(|e| format!("tail_items failed: {e:?}"))?;
        ensure(tail.is_empty(), "expected empty tail for single item")
    }

    #[test]
    fn tail_items_two_items_returns_second() -> Result<(), String> {
        let items: Box<[SlotValue]> = vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice();
        let tail = tail_items(&items).map_err(|e| format!("tail_items failed: {e:?}"))?;
        ensure(tail.len() == 1, format!("expected len 1, got {}", tail.len()))?;
        ensure(
            tail.get(0) == Some(&SlotValue::I64(20)),
            format!("expected I64(20), got {:?}", tail.get(0)),
        )
    }

    #[test]
    fn tail_items_three_items_returns_last_two() -> Result<(), String> {
        let items: Box<[SlotValue]> = vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
        ]
        .into_boxed_slice();
        let tail = tail_items(&items).map_err(|e| format!("tail_items failed: {e:?}"))?;
        ensure(tail.len() == 2, format!("expected len 2, got {}", tail.len()))?;
        ensure(
            tail.get(0) == Some(&SlotValue::I64(2)),
            "first tail item mismatch",
        )?;
        ensure(
            tail.get(1) == Some(&SlotValue::I64(3)),
            "second tail item mismatch",
        )
    }

    #[test]
    fn tail_items_preserves_mixed_types() -> Result<(), String> {
        let items: Box<[SlotValue]> = vec![
            SlotValue::I64(1),
            SlotValue::Bool(true),
            SlotValue::Null,
        ]
        .into_boxed_slice();
        let tail = tail_items(&items).map_err(|e| format!("tail_items failed: {e:?}"))?;
        ensure(tail.len() == 2, format!("expected len 2, got {}", tail.len()))?;
        ensure(
            tail.get(0) == Some(&SlotValue::Bool(true)),
            "expected Bool(true) as first tail item",
        )?;
        ensure(
            tail.get(1) == Some(&SlotValue::Null),
            "expected Null as second tail item",
        )
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
        ensure(run.pc() == target, format!("expected pc={target:?}, got {:?}", run.pc()))
    }

    #[test]
    fn jump_to_increments_executed_counter() -> Result<(), String> {
        let mut run = fresh_frame();
        let before = run.executed();
        jump_to(&mut run, StepIdx::new(1)).map_err(|e| format!("jump_to failed: {e:?}"))?;
        ensure(
            run.executed() == before.saturating_add(1),
            format!("expected {}, got {}", before.saturating_add(1), run.executed()),
        )
    }

    #[test]
    fn jump_to_returns_continue_signal() -> Result<(), String> {
        let mut run = fresh_frame();
        let signal = jump_to(&mut run, StepIdx::new(1)).map_err(|e| format!("jump_to failed: {e:?}"))?;
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
        let signal =
            jump_to_next(&mut run, Some(next), StepIdx::ZERO).map_err(|e| format!("jump_to_next failed: {e:?}"))?;
        ensure(run.pc() == next, format!("expected pc={next:?}, got {:?}", run.pc()))?;
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
            Err(EngineError::MissingNextStep { step }) => {
                ensure(step == StepIdx::ZERO, format!("expected ZERO, got {step:?}"))
            }
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
            format!("expected {}, got {}", before.saturating_add(1), run.executed()),
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
}
