#![forbid(unsafe_code)]
//! Canonical object-key membership with unchanged replay state on refusal.

use crate::replay::{ReplayError, ReplayExprStack};
use crate::value::SlotValue;
use crate::value_store::ValueStore;

fn unsupported() -> ReplayError {
    ReplayError::Internal {
        reason: "unsupported expression op for replay",
    }
}

pub(super) fn eval_has(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    if stack.len > stack.capacity || usize::from(stack.capacity) > stack.values.len() {
        return Err(unsupported());
    }
    let next_len = stack.len.checked_sub(1).ok_or_else(unsupported)?;
    let left_index = usize::from(next_len.checked_sub(1).ok_or_else(unsupported)?);
    let right_index = usize::from(next_len);
    let left = stack
        .values
        .get(left_index)
        .copied()
        .ok_or_else(unsupported)?;
    let right = stack
        .values
        .get(right_index)
        .copied()
        .ok_or_else(unsupported)?;
    let (SlotValue::Object(object), SlotValue::Symbol(key)) = (left, right) else {
        return Err(unsupported());
    };
    let found = store
        .object(object)
        .map_err(|_| unsupported())?
        .iter()
        .any(|field| field.key == key);

    // Finish every fallible check before changing either stack contents or length.
    let destination = stack.values.get_mut(left_index).ok_or_else(unsupported)?;
    *destination = SlotValue::Bool(found);
    stack.len = next_len;
    Ok(())
}
