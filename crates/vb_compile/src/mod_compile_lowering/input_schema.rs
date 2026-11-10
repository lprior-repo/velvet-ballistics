#![forbid(unsafe_code)]
use crate::{AuthorValue, CompileError};
use vb_core::{CompiledInputConstraint, InputSlotKind};

#[path = "input_schema_constraints.rs"]
mod constraints;
use constraints::constraint_from_entries;
#[path = "input_schema_defaults.rs"]
pub(super) mod defaults;
#[path = "input_schema_fields.rs"]
mod fields;

/// Derive a runtime `InputSlotKind` from a declared input schema.
///
/// Parses the declared schema meaning (shorthand string or longform mapping)
/// rather than inferring from the author value shape.
pub(super) fn kind_from_input_schema(value: &AuthorValue) -> Result<InputSlotKind, CompileError> {
    match value {
        AuthorValue::Text(s) => match s.as_str() {
            "text" => Ok(InputSlotKind::Symbol),
            "number" => Ok(InputSlotKind::I64),
            "boolean" => Ok(InputSlotKind::Bool),
            "object" => Ok(InputSlotKind::Object),
            "any" => Ok(InputSlotKind::Any),
            "list<any>" | "list<text>" | "list<number>" | "list<boolean>" => {
                Ok(InputSlotKind::List)
            }
            _ => Err(CompileError::InvalidInputSchema {
                field: "inputs",
                expected: "an allowed input schema",
            }),
        },
        AuthorValue::Mapping(entries) => {
            if entries.iter().filter(|e| e.key == "is").count() > 1 {
                return Err(CompileError::InvalidInputSchema {
                    field: "inputs",
                    expected: "a single is discriminator in an allowed input schema",
                });
            }
            let is_entry = entries.iter().find(|e| e.key == "is");
            match is_entry {
                Some(entry) => match &entry.value {
                    AuthorValue::Text(s) => match s.as_str() {
                        "text" => Ok(InputSlotKind::Symbol),
                        "number" => Ok(InputSlotKind::I64),
                        "boolean" => Ok(InputSlotKind::Bool),
                        "object" => Ok(InputSlotKind::Object),
                        "list" => Ok(InputSlotKind::List),
                        "any" => Ok(InputSlotKind::Any),
                        _ => Err(CompileError::InvalidInputSchema {
                            field: "inputs",
                            expected: "an allowed input schema",
                        }),
                    },
                    _ => Err(CompileError::InvalidInputSchema {
                        field: "inputs",
                        expected: "an allowed input schema",
                    }),
                },
                None => Err(CompileError::InvalidInputSchema {
                    field: "inputs",
                    expected: "an allowed input schema",
                }),
            }
        }
        _ => Err(CompileError::InvalidInputSchema {
            field: "inputs",
            expected: "an allowed input schema",
        }),
    }
}

/// Lower the retained per-slot constraint for a declared input schema.
///
/// Membership of the declared shape is validated by reusing
/// `kind_from_input_schema`; this function additionally retains every
/// enforceable keyword (`optional`, `nullable`, `default`, `min`/`max`,
/// `min_length`/`max_length`, `of`, `extra`, `secret`) and rejects
/// unsupported combinations explicitly instead of erasing them.
/// Nested fields and portable defaults are retained and validated before IR emission.
pub(super) fn constraint_from_input_schema(
    value: &AuthorValue,
) -> Result<CompiledInputConstraint, CompileError> {
    fields::lower(value)
}

#[cfg(test)]
mod tests {
    use super::kind_from_input_schema;
    use crate::{AuthorEntry, AuthorValue, CompileError};

    #[test]
    fn kind_from_input_schema_rejects_repeated_is_discriminator() {
        let value = AuthorValue::Mapping(vec![
            AuthorEntry {
                key: "is".into(),
                value: AuthorValue::Text("text".into()),
            },
            AuthorEntry {
                key: "is".into(),
                value: AuthorValue::Text("number".into()),
            },
        ]);
        assert!(matches!(
            kind_from_input_schema(&value),
            Err(CompileError::InvalidInputSchema { .. })
        ));
    }
}
