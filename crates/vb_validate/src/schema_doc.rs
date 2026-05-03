//! Lightweight document model for schema validation.

pub struct WorkflowDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

#[derive(Clone)]
pub enum FieldValue {
    String(String),
    Sequence(Vec<StepDoc>),
    Mapping(Vec<(String, FieldValue)>),
    Empty,
}

#[derive(Clone)]
pub struct StepDoc {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

impl WorkflowDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self { Self { fields } }
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::String(s) = v { Some(s.as_str()) } else { None } } else { None })
    }
    pub fn get_sequence(&self, field: &str) -> Option<&[StepDoc]> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::Sequence(s) = v { Some(s.as_slice()) } else { None } } else { None })
    }
    pub fn get_mapping(&self, field: &str) -> Option<&[(String, FieldValue)]> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::Mapping(e) = v { Some(e.as_slice()) } else { None } } else { None })
    }
    pub fn has_field(&self, field: &str) -> bool { self.fields.iter().any(|(n, _)| n == field) }
    pub fn field_names(&self) -> Vec<&str> { self.fields.iter().map(|(n, _)| n.as_str()).collect() }
}

impl StepDoc {
    #[must_use]
    pub fn from_pairs(fields: Vec<(String, FieldValue)>) -> Self { Self { fields } }
    pub fn get_string(&self, field: &str) -> Option<&str> {
        self.fields.iter().find_map(|(n, v)| if n == field { if let FieldValue::String(s) = v { Some(s.as_str()) } else { None } } else { None })
    }
    pub fn field_names(&self) -> Vec<&str> { self.fields.iter().map(|(n, _)| n.as_str()).collect() }
}
