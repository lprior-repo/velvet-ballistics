#![forbid(unsafe_code)]
use crate::{AuthorValue, CompileError};
use vb_core::InputSlotKind;

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
