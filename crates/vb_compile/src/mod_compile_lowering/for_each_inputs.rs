#![forbid(unsafe_code)]

use std::collections::HashMap;

use crate::{AuthorEntry, AuthorValue, CompileError};
use vb_core::SlotIdx;

/// Cold readonly declared input origins for for_each lowering.
///
/// Records each declared input schema nonfallibly and resolves
/// selected slots into validated u32 bounds.
pub(in crate::mod_compile_lowering) struct ForEachInputs {
    inputs: HashMap<SlotIdx, (String, AuthorValue)>,
}

impl ForEachInputs {
    pub(in crate::mod_compile_lowering) fn new() -> Self {
        Self {
            inputs: HashMap::new(),
        }
    }

    /// Record a declared input schema. Never errors.
    pub(in crate::mod_compile_lowering) fn record(
        &mut self,
        slot: SlotIdx,
        name: &str,
        schema: &AuthorValue,
    ) {
        self.inputs.insert(slot, (name.to_string(), schema.clone()));
    }

    /// Resolve a selected slot into a validated u32 bound.
    pub(in crate::mod_compile_lowering) fn resolve(
        &self,
        slot: SlotIdx,
        name: &str,
        index: usize,
    ) -> Result<u32, CompileError> {
        let entry = self.inputs.get(&slot);
        let (recorded_name, schema) = match entry {
            Some(e) => e,
            None => {
                return Err(shape_error(index));
            }
        };

        if recorded_name != name {
            return Err(shape_error(index));
        }

        parse_list_any_max(schema, index)
    }
}

fn shape_error(index: usize) -> CompileError {
    CompileError::StepFieldShape {
        step: index,
        field: "for_each.input",
        expected: "direct declared list<any> input with finite schema max",
    }
}

fn parse_list_any_max(schema: &AuthorValue, index: usize) -> Result<u32, CompileError> {
    let entries = match schema {
        AuthorValue::Mapping(entries) => entries,
        _ => {
            return Err(shape_error(index));
        }
    };

    if !is_list_schema(entries) {
        return Err(shape_error(index));
    }

    if !is_of_any(entries) {
        return Err(shape_error(index));
    }

    let max = match get_max_value(entries) {
        Some(n) => n,
        None => {
            return Err(shape_error(index));
        }
    };

    if max < 0 {
        return Err(shape_error(index));
    }

    u32::try_from(max).map_err(|_| CompileError::InvalidInputSchema {
        field: "inputs.max",
        expected: "list maximum representable as u32",
    })
}

fn is_list_schema(entries: &[AuthorEntry<AuthorValue>]) -> bool {
    matches!(find_entry_value(entries, "is"), Some(AuthorValue::Text(s)) if s == "list")
}

fn is_of_any(entries: &[AuthorEntry<AuthorValue>]) -> bool {
    matches!(find_entry_value(entries, "of"), Some(AuthorValue::Text(s)) if s == "any")
}

fn get_max_value(entries: &[AuthorEntry<AuthorValue>]) -> Option<i64> {
    match find_entry_value(entries, "max") {
        Some(AuthorValue::I64(n)) => Some(*n),
        _ => None,
    }
}

fn find_entry_value<'a>(
    entries: &'a [AuthorEntry<AuthorValue>],
    key: &str,
) -> Option<&'a AuthorValue> {
    for entry in entries {
        if entry.key == key {
            return Some(&entry.value);
        }
    }
    None
}
