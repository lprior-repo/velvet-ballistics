use crate::{AuthorEntry, AuthorValue, CompileError};
use vb_core::input_schema::{MAX_INPUT_SCHEMA_DEPTH, apply_input_schema_defaults, default_value};
use vb_core::limits::MAX_VALUES_PER_RUN;
use vb_core::{
    CompiledInputConstraint, CompiledInputSlot, InputDefaultValue, InputSlotKind, SlotIdx,
    SlotValue, Taint, ValueStore,
};

pub(super) fn schema_default(
    entries: &[AuthorEntry<AuthorValue>],
) -> Result<(Option<SlotValue>, Option<InputDefaultValue>), CompileError> {
    let Some(value) = entries
        .iter()
        .find(|entry| entry.key == "default")
        .map(|entry| &entry.value)
    else {
        return Ok((None, None));
    };
    match value {
        AuthorValue::Null => Ok((Some(SlotValue::Null), None)),
        AuthorValue::Bool(value) => Ok((Some(SlotValue::Bool(*value)), None)),
        AuthorValue::I64(value) => Ok((Some(SlotValue::I64(*value)), None)),
        _ => owned(value).map(|value| (None, Some(value))),
    }
}

pub(super) fn validate(
    kind: InputSlotKind,
    constraint: &CompiledInputConstraint,
) -> Result<(), CompileError> {
    let mut checkpoint = ValueStore::new().checkpoint_v1();
    checkpoint.max_arena_entries = u64::try_from(MAX_VALUES_PER_RUN).map_err(|_| invalid())?;
    let Some(value) =
        default_value(constraint, &mut checkpoint, Taint::Clean).map_err(|_| invalid())?
    else {
        return Ok(());
    };
    apply_input_schema_defaults(kind, constraint, value, Taint::Clean, &mut checkpoint)
        .map_err(|_| invalid())?;
    let declaration = CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind,
        constraint: constraint.clone(),
    };
    vb_core::input_validation::validate_input_value(&declaration, value, 0, Some(&checkpoint))
        .map_err(|_| invalid())
}

enum Work<'a> {
    Value(&'a AuthorValue, usize),
    List(usize),
    Object(&'a [AuthorEntry<AuthorValue>]),
}

pub(crate) fn owned(value: &AuthorValue) -> Result<InputDefaultValue, CompileError> {
    let mut work = vec![Work::Value(value, 0)];
    let mut results = Vec::new();
    let mut visited = 0usize;
    while let Some(task) = work.pop() {
        let result = match task {
            Work::Value(value, depth) => {
                visited = visited.checked_add(1).ok_or_else(invalid)?;
                if depth > MAX_INPUT_SCHEMA_DEPTH || visited > MAX_VALUES_PER_RUN {
                    return Err(invalid());
                }
                match value {
                    AuthorValue::Null => InputDefaultValue::Null,
                    AuthorValue::Bool(value) => InputDefaultValue::Bool(*value),
                    AuthorValue::I64(value) => InputDefaultValue::I64(*value),
                    AuthorValue::Text(text) => {
                        InputDefaultValue::Text(text.clone().into_boxed_str())
                    }
                    AuthorValue::Sequence(items) => {
                        reserve(&mut work, items.len())?;
                        work.push(Work::List(items.len()));
                        for item in items.iter().rev() {
                            work.push(Work::Value(item, depth.checked_add(1).ok_or_else(invalid)?));
                        }
                        continue;
                    }
                    AuthorValue::Mapping(fields) => {
                        for (index, entry) in fields.iter().enumerate() {
                            if fields
                                .iter()
                                .take(index)
                                .any(|prior| prior.key == entry.key)
                            {
                                return Err(invalid());
                            }
                        }
                        reserve(&mut work, fields.len())?;
                        work.push(Work::Object(fields));
                        for field in fields.iter().rev() {
                            work.push(Work::Value(
                                &field.value,
                                depth.checked_add(1).ok_or_else(invalid)?,
                            ));
                        }
                        continue;
                    }
                    _ => return Err(invalid()),
                }
            }
            Work::List(count) => {
                let start = results.len().checked_sub(count).ok_or_else(invalid)?;
                InputDefaultValue::List(results.drain(start..).collect())
            }
            Work::Object(fields) => {
                let start = results
                    .len()
                    .checked_sub(fields.len())
                    .ok_or_else(invalid)?;
                InputDefaultValue::Object(
                    fields
                        .iter()
                        .zip(results.drain(start..))
                        .map(|(field, value)| (field.key.clone().into_boxed_str(), value))
                        .collect(),
                )
            }
        };
        results.try_reserve(1).map_err(|_| invalid())?;
        results.push(result);
    }
    results.pop().ok_or_else(invalid)
}

fn reserve(work: &mut Vec<Work<'_>>, count: usize) -> Result<(), CompileError> {
    let extra = count.checked_add(1).ok_or_else(invalid)?;
    if work.len().checked_add(extra).ok_or_else(invalid)? > MAX_VALUES_PER_RUN {
        return Err(invalid());
    }
    work.try_reserve(extra).map_err(|_| invalid())
}

fn invalid() -> CompileError {
    CompileError::InvalidInputSchema {
        field: "inputs.default",
        expected: "a bounded default satisfying the complete declared schema",
    }
}
