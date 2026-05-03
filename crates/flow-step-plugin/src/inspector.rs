/// Inspector field definition.
#[derive(Debug, Clone)]
pub struct InspectorField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Text,
    Number,
    Boolean,
    Select(Vec<String>),
    Expression,
    Duration,
    SecretRef,
}

/// Get inspector fields for a given state kind.
pub fn fields_for_kind(kind: &str) -> Vec<InspectorField> {
    let _ = kind;
    Vec::new()
}
