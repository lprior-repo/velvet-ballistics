#![forbid(unsafe_code)]
use crate::{AuthorEntry, AuthorValue, CompileError};
use vb_core::{CompiledInputConstraint, InputSlotKind};

pub(super) fn constraint_from_entries(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<CompiledInputConstraint, CompileError> {
    reject_pattern(entries)?;
    let optional = schema_flag(entries, "optional")?;
    let nullable = schema_flag(entries, "nullable")?;
    let secret = schema_flag(entries, "secret")?;
    let (default_value, default_cold) = super::defaults::schema_default(entries)?;
    let (min, max) = schema_range_bounds(entries, kind)?;
    let (min_length, max_length) = schema_length_bounds(entries, kind)?;
    let element = schema_element(entries, kind)?;
    let allow_extra = schema_extra(entries, kind)?;
    reject_misplaced_fields(entries, kind)?;
    Ok(CompiledInputConstraint {
        presence: optional.into(),
        nullability: nullable.into(),
        default_value,
        min,
        max,
        min_length,
        max_length,
        element,
        extra: allow_extra.into(),
        secretness: secret.into(),
        fields: None,
        default_cold,
    })
}

fn reject_pattern(entries: &[AuthorEntry<AuthorValue>]) -> Result<(), CompileError> {
    if entries.iter().any(|entry| entry.key == "pattern") {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.pattern",
            expected: "unsupported until a bounded regex engine exists",
        });
    }
    Ok(())
}

fn schema_flag(entries: &[AuthorEntry<AuthorValue>], key: &str) -> Result<bool, CompileError> {
    match find_entry(entries, key) {
        None => Ok(false),
        Some(AuthorValue::Bool(value)) => Ok(*value),
        Some(_) => Err(CompileError::InvalidInputSchema {
            field: "inputs boolean flag",
            expected: "a boolean",
        }),
    }
}

fn schema_range_bounds(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<(Option<i64>, Option<i64>), CompileError> {
    let min = schema_integer(entries, "min")?;
    let max = schema_integer(entries, "max")?;
    if (min.is_some() || max.is_some()) && !matches!(kind, InputSlotKind::I64 | InputSlotKind::List)
    {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min/max",
            expected: "present only for number or list schemas",
        });
    }
    if kind == InputSlotKind::List && [min, max].into_iter().flatten().any(|bound| bound < 0) {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min/max",
            expected: "non-negative list length bounds",
        });
    }
    if let (Some(low), Some(high)) = (min, max)
        && low > high
    {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min/max",
            expected: "min less than or equal to max",
        });
    }
    Ok((min, max))
}

fn schema_length_bounds(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<(Option<i64>, Option<i64>), CompileError> {
    let min = schema_integer(entries, "min_length")?;
    let max = schema_integer(entries, "max_length")?;
    if min.is_none() && max.is_none() {
        return Ok((None, None));
    }
    if kind != InputSlotKind::Symbol {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min_length/max_length",
            expected: "present only for text schemas",
        });
    }
    if [min, max].into_iter().flatten().any(|bound| bound < 0) {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min_length/max_length",
            expected: "non-negative text length bounds",
        });
    }
    if let (Some(low), Some(high)) = (min, max)
        && low > high
    {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.min_length/max_length",
            expected: "min less than or equal to max",
        });
    }
    Ok((min, max))
}

fn schema_element(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<Option<InputSlotKind>, CompileError> {
    match find_entry(entries, "of") {
        None => {
            if kind == InputSlotKind::List {
                return Err(CompileError::InvalidInputSchema {
                    field: "inputs.of",
                    expected: "required when is is list",
                });
            }
            Ok(None)
        }
        Some(AuthorValue::Text(name)) => {
            if kind != InputSlotKind::List {
                return Err(CompileError::InvalidInputSchema {
                    field: "inputs.of",
                    expected: "present only when is is list",
                });
            }
            match name.as_str() {
                "any" => Ok(Some(InputSlotKind::Any)),
                "text" => Ok(Some(InputSlotKind::Symbol)),
                "number" => Ok(Some(InputSlotKind::I64)),
                "boolean" => Ok(Some(InputSlotKind::Bool)),
                "object" => Ok(Some(InputSlotKind::Object)),
                _ => Err(CompileError::InvalidInputSchema {
                    field: "inputs.of",
                    expected: "one of any, text, number, boolean, object",
                }),
            }
        }
        Some(_) => Err(CompileError::InvalidInputSchema {
            field: "inputs.of",
            expected: "one of any, text, number, boolean, object",
        }),
    }
}

fn schema_extra(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<Option<bool>, CompileError> {
    match find_entry(entries, "extra") {
        None => Ok(None),
        Some(AuthorValue::Text(policy)) => {
            if kind != InputSlotKind::Object {
                return Err(CompileError::InvalidInputSchema {
                    field: "inputs.extra",
                    expected: "present only when is is object",
                });
            }
            match policy.as_str() {
                "allow" => Ok(Some(true)),
                "reject" => Ok(Some(false)),
                _ => Err(CompileError::InvalidInputSchema {
                    field: "inputs.extra",
                    expected: "allow or reject",
                }),
            }
        }
        Some(_) => Err(CompileError::InvalidInputSchema {
            field: "inputs.extra",
            expected: "allow or reject",
        }),
    }
}

fn reject_misplaced_fields(
    entries: &[AuthorEntry<AuthorValue>],
    kind: InputSlotKind,
) -> Result<(), CompileError> {
    if kind != InputSlotKind::Object && entries.iter().any(|entry| entry.key == "fields") {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.fields",
            expected: "present only when is is object",
        });
    }
    Ok(())
}

fn schema_integer(
    entries: &[AuthorEntry<AuthorValue>],
    key: &str,
) -> Result<Option<i64>, CompileError> {
    match find_entry(entries, key) {
        None => Ok(None),
        Some(AuthorValue::I64(value)) => Ok(Some(*value)),
        Some(_) => Err(CompileError::InvalidInputSchema {
            field: "inputs bound",
            expected: "an integer",
        }),
    }
}

fn find_entry<'a>(entries: &'a [AuthorEntry<AuthorValue>], key: &str) -> Option<&'a AuthorValue> {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| &entry.value)
}
