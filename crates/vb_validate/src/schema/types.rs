//! Lightweight document model for schema validation.
//!
//! Read-only views of workflow documents, steps, and field values.

pub(super) const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish",
];

/// Read-only view of a workflow document's top-level fields.
#[derive(Clone, PartialEq, Debug)]
pub struct WorkflowDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

/// Value associated with a workflow field.
#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum FieldValue {
    /// String scalar value.
    String(String),
    /// Ordered sequence of step documents.
    Sequence(Vec<StepDoc>),
    /// Key-value mapping (for triggers, etc).
    Mapping(Vec<(String, FieldValue)>),
    /// Field present but empty/null.
    Empty,
}

/// Read-only view of a single step's fields.
#[derive(Clone, PartialEq, Debug)]
pub struct StepDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

impl WorkflowDoc {
    /// Creates a workflow document from key-value pairs.
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self {
        Self { fields }
    }

    /// Returns the string value of a field, if present and string-typed.
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(name, value)| {
            if name == field {
                match value {
                    FieldValue::String(s) => Some(s.as_str()),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// Returns the sequence value of a field, if present.
    pub fn get_sequence(&self, field: &str) -> Option<&[StepDoc]> {
        self.fields.iter().find_map(|(name, value)| {
            if name == field {
                match value {
                    FieldValue::Sequence(steps) => Some(steps.as_slice()),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// Returns the mapping value of a field, if present.
    pub fn get_mapping(&self, field: &str) -> Option<&[(String, FieldValue)]> {
        self.fields.iter().find_map(|(name, value)| {
            if name == field {
                match value {
                    FieldValue::Mapping(entries) => Some(entries.as_slice()),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// Returns whether a field is present.
    pub fn has_field(&self, field: &str) -> bool {
        self.fields.iter().any(|(name, _)| name == field)
    }

    /// Returns all field names.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|(name, _)| name.as_str()).collect()
    }
}

impl StepDoc {
    /// Creates a step document from key-value pairs.
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self {
        Self { fields }
    }

    /// Returns the string value of a field, if present and string-typed.
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(name, value)| {
            if name == field {
                match value {
                    FieldValue::String(s) => Some(s.as_str()),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// Returns all field names.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Returns the primitive field name and its value, if one is declared.
    pub(super) fn primitive_value(&self) -> Option<(&str, &FieldValue)> {
        for (field, value) in &self.fields {
            if STEP_PRIMITIVES.contains(&field.as_str()) {
                return Some((field, value));
            }
        }
        None
    }
}
