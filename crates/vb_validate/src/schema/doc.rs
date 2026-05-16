#![forbid(unsafe_code)]
//! Document model types for schema validation.

/// Read-only view of a workflow document's top-level fields.
pub struct WorkflowDoc {
    fields: Vec<(String, FieldValue)>,
}

/// Value associated with a workflow field.
#[derive(Clone)]
pub enum FieldValue {
    String(String),
    Sequence(Vec<StepDoc>),
    Mapping(Vec<(String, FieldValue)>),
    Empty,
}

/// Read-only view of a single step's fields.
#[derive(Clone)]
pub struct StepDoc {
    fields: Vec<(String, FieldValue)>,
}

impl WorkflowDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self {
        Self { fields }
    }

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

    pub fn has_field(&self, field: &str) -> bool {
        self.fields.iter().any(|(name, _)| name == field)
    }

    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|(name, _)| name.as_str()).collect()
    }
}

impl StepDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self {
        Self { fields }
    }

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

    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|(name, _)| name.as_str()).collect()
    }
}
