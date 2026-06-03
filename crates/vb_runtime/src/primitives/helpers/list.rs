#![forbid(unsafe_code)]
//! List helper functions.

use vb_core::errors::EngineError;
use vb_core::ids::ListId;
use vb_core::value::SlotValue;

/// Extracts a [`ListId`] from a [`SlotValue::List`], returning a type error otherwise.
pub(crate) fn expect_list(value: SlotValue) -> Result<ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

/// Constructs an empty boxed slice for use as a tail or empty list.
pub(crate) fn empty_list() -> Box<[SlotValue]> {
    Vec::<SlotValue>::new().into_boxed_slice()
}

/// Returns all items after the first element.
///
/// # Errors
/// Returns [`EngineError::InternalInvariantViolation`] if the input is empty
/// (invariant: caller should check `items.len() > 0` before calling).
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