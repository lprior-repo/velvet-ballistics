use super::{constraint_from_entries, defaults, kind_from_input_schema};
use crate::{AuthorEntry, AuthorValue, CompileError};
use vb_core::input_schema::MAX_INPUT_SCHEMA_DEPTH;
use vb_core::limits::{MAX_OBJECT_FIELDS_PER_VALUE, MAX_VALUES_PER_RUN};
use vb_core::{CompiledInputConstraint, CompiledInputField, InputSlotKind};

enum Work<'a> {
    Schema(&'a AuthorValue, usize),
    Object(
        InputSlotKind,
        CompiledInputConstraint,
        &'a [AuthorEntry<AuthorValue>],
    ),
}

pub(super) fn lower(value: &AuthorValue) -> Result<CompiledInputConstraint, CompileError> {
    let mut work = vec![Work::Schema(value, 0)];
    let mut results = Vec::new();
    let mut visited = 0usize;
    while let Some(task) = work.pop() {
        match task {
            Work::Schema(value, depth) => {
                visited = visited.checked_add(1).ok_or_else(invalid)?;
                if depth > MAX_INPUT_SCHEMA_DEPTH || visited > MAX_VALUES_PER_RUN {
                    return Err(invalid());
                }
                let kind = kind_from_input_schema(value)?;
                let mut constraint = match value {
                    AuthorValue::Text(text) => shorthand(text),
                    AuthorValue::Mapping(entries) => {
                        check_entries(entries, depth)?;
                        constraint_from_entries(entries, kind)?
                    }
                    _ => return Err(invalid()),
                };
                if let AuthorValue::Mapping(entries) = value
                    && let Some(entry) = entries.iter().find(|entry| entry.key == "fields")
                {
                    let AuthorValue::Mapping(fields) = &entry.value else {
                        return Err(invalid());
                    };
                    if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
                        return Err(invalid());
                    }
                    let additional = fields.len().checked_add(1).ok_or_else(invalid)?;
                    if work.len().checked_add(additional).ok_or_else(invalid)? > MAX_VALUES_PER_RUN
                    {
                        return Err(invalid());
                    }
                    work.try_reserve(additional).map_err(|_| invalid())?;
                    for (index, field) in fields.iter().enumerate() {
                        crate::validate_public_name("inputs.fields", &field.key)?;
                        if fields
                            .iter()
                            .take(index)
                            .any(|prior| prior.key == field.key)
                        {
                            return Err(invalid());
                        }
                    }
                    constraint.fields = Some(Box::default());
                    work.push(Work::Object(kind, constraint, fields));
                    for field in fields.iter().rev() {
                        work.push(Work::Schema(
                            &field.value,
                            depth.checked_add(1).ok_or_else(invalid)?,
                        ));
                    }
                } else {
                    defaults::validate(kind, &constraint)?;
                    results.push((kind, constraint));
                }
            }
            Work::Object(kind, mut constraint, fields) => {
                let start = results
                    .len()
                    .checked_sub(fields.len())
                    .ok_or_else(invalid)?;
                let mut compiled = Vec::new();
                compiled
                    .try_reserve_exact(fields.len())
                    .map_err(|_| invalid())?;
                for (field, (kind, constraint)) in fields.iter().zip(results.drain(start..)) {
                    compiled.push(CompiledInputField {
                        name: field.key.clone().into_boxed_str(),
                        kind,
                        constraint,
                    });
                }
                constraint.fields = Some(compiled.into_boxed_slice());
                defaults::validate(kind, &constraint)?;
                results.push((kind, constraint));
            }
        }
    }
    results
        .pop()
        .map(|(_, constraint)| constraint)
        .ok_or_else(invalid)
}

fn shorthand(text: &str) -> CompiledInputConstraint {
    let mut constraint = CompiledInputConstraint::required();
    constraint.element = match text {
        "list<any>" => Some(InputSlotKind::Any),
        "list<text>" => Some(InputSlotKind::Symbol),
        "list<number>" => Some(InputSlotKind::I64),
        "list<boolean>" => Some(InputSlotKind::Bool),
        _ => None,
    };
    constraint
}

fn check_entries(entries: &[AuthorEntry<AuthorValue>], depth: usize) -> Result<(), CompileError> {
    const ALLOWED: &[&str] = &[
        "is",
        "of",
        "fields",
        "extra",
        "optional",
        "nullable",
        "default",
        "min",
        "max",
        "min_length",
        "max_length",
        "pattern",
        "secret",
        "from",
    ];
    for (index, entry) in entries.iter().enumerate() {
        if !ALLOWED.contains(&entry.key.as_str())
            || (entry.key == "from" && depth != 0)
            || entries
                .iter()
                .take(index)
                .any(|prior| prior.key == entry.key)
        {
            return Err(invalid());
        }
        if entry.key == "from"
            && !matches!(&entry.value, AuthorValue::Text(text) if !text.is_empty())
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> CompileError {
    CompileError::InvalidInputSchema {
        field: "inputs.fields",
        expected: "a bounded mapping of unique field names to complete schemas",
    }
}
