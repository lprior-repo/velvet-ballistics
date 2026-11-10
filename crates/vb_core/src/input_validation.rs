//! Shared checks for declared numeric input contracts at admission.

use crate::value::SlotValue;
use crate::value_store::ValueStoreCheckpointV1;
use crate::workflow::{CompiledInputSlot, InputSlotKind};

/// A supplied value does not satisfy its retained input declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputValueIssue {
    /// The runtime value kind disagrees with the declaration.
    Kind,
    /// A numeric value or payload length exceeds its declared bounds.
    Bounds,
    /// An arena reference has no corresponding payload.
    Payload,
    /// A list member has a different kind than its element declaration.
    Element,
    /// An object contains fields forbidden by its closed schema.
    ExtraField,
}

/// Classifies a handle or scalar without resolving an arena reference.
#[must_use]
pub const fn input_value_kind(value: SlotValue) -> InputSlotKind {
    match value {
        SlotValue::Null => InputSlotKind::Null,
        SlotValue::Bool(_) => InputSlotKind::Bool,
        SlotValue::I64(_) => InputSlotKind::I64,
        SlotValue::F64(_) => InputSlotKind::F64,
        SlotValue::Symbol(_) => InputSlotKind::Symbol,
        SlotValue::List(_) => InputSlotKind::List,
        SlotValue::Object(_) => InputSlotKind::Object,
        SlotValue::Blob(_) => InputSlotKind::Blob,
    }
}

/// Checks a supplied value before acceptance or action scheduling.
///
/// Payload constraints require an owned checkpoint. Positional scalar wires
/// may retain a compiled symbol ID only when no text payload checks are needed.
/// Text lengths use UTF-8 bytes, matching expression length and arena bounds.
pub fn validate_input_value(
    declaration: &CompiledInputSlot,
    value: SlotValue,
    symbols_count: u32,
    checkpoint: Option<&ValueStoreCheckpointV1>,
) -> Result<(), InputValueIssue> {
    let constraint = &declaration.constraint;
    if value == SlotValue::Null && constraint.nullable {
        return Ok(());
    }
    if declaration.kind != InputSlotKind::Any && declaration.kind != input_value_kind(value) {
        return Err(InputValueIssue::Kind);
    }
    match value {
        SlotValue::I64(number) => check_bounds(number, constraint.min, constraint.max),
        SlotValue::Symbol(id) => {
            if let Some(checkpoint) = checkpoint {
                let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
                let text = checkpoint
                    .symbols
                    .get(index)
                    .ok_or(InputValueIssue::Payload)?;
                check_length(text.len(), constraint.min_length, constraint.max_length)
            } else if id.get() >= symbols_count
                || constraint.min_length.is_some()
                || constraint.max_length.is_some()
            {
                Err(InputValueIssue::Payload)
            } else {
                Ok(())
            }
        }
        SlotValue::List(id) => {
            let checkpoint = checkpoint.ok_or(InputValueIssue::Payload)?;
            let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
            let items = checkpoint
                .lists
                .get(index)
                .ok_or(InputValueIssue::Payload)?;
            check_length(items.len(), constraint.min, constraint.max)?;
            if let Some(expected) = constraint.element
                && expected != InputSlotKind::Any
                && items.iter().any(|item| input_value_kind(*item) != expected)
            {
                return Err(InputValueIssue::Element);
            }
            Ok(())
        }
        SlotValue::Object(id) => {
            let checkpoint = checkpoint.ok_or(InputValueIssue::Payload)?;
            let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
            let fields = checkpoint
                .objects
                .get(index)
                .ok_or(InputValueIssue::Payload)?;
            if constraint.allow_extra == Some(false) && !fields.is_empty() {
                return Err(InputValueIssue::ExtraField);
            }
            Ok(())
        }
        SlotValue::Blob(id) => {
            let checkpoint = checkpoint.ok_or(InputValueIssue::Payload)?;
            let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
            checkpoint
                .blobs
                .get(index)
                .ok_or(InputValueIssue::Payload)
                .map(|_| ())
        }
        _ => Ok(()),
    }
}

fn check_length(length: usize, min: Option<i64>, max: Option<i64>) -> Result<(), InputValueIssue> {
    let length = i64::try_from(length).map_err(|_| InputValueIssue::Bounds)?;
    check_bounds(length, min, max)
}

fn check_bounds(value: i64, min: Option<i64>, max: Option<i64>) -> Result<(), InputValueIssue> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        Err(InputValueIssue::Bounds)
    } else {
        Ok(())
    }
}
