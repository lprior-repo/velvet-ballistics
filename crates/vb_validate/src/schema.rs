//! Schema validation for workflow documents.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

use crate::{ValidationError, ValidationResult};

const CANONICAL_VERSION: &str = "velvet-ballastics/v1";

const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];

const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version",
    "name",
    "when",
    "inputs",
    "vars",
    "secrets",
    "result",
    "examples",
    "steps",
];

const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id",
    "name",
    "then",
    "save",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "retry",
];

const STEP_PRIMITIVES: &[&str] = &[
    "save",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
];

const RESERVED_IDS: &[&str] = &[
    "now",
    "random",
    "runtime",
    "null",
    "true",
    "false",
    "input",
    "vars",
    "secrets",
    "steps",
    "error",
    "attempt",
    "total",
];

/// Validates a workflow document against the v1 schema.
///
/// Checks required fields, unknown fields, version, trigger, IDs,
/// step fields, and single-primitive rules.
pub fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_required_fields(doc)?;
    validate_unknown_fields(doc)?;
    validate_version(doc)?;
    validate_trigger(doc)?;
    validate_ids(doc)?;
    validate_step_fields(doc)?;
    Ok(())
}

/// Checks that the version field matches the canonical version string.
pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(version) if version == CANONICAL_VERSION => Ok(()),
        Some(version) => Err(ValidationError::InvalidVersion {
            version: version.to_owned(),
        }),
        None => Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        }),
    }
}

/// Validates the trigger (when) block accepts manual/ipc and rejects HTTP.
pub fn validate_trigger(doc: &WorkflowDoc) -> ValidationResult<()> {
    let trigger = doc.get_mapping("when").ok_or_else(|| {
        ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        }
    })?;
    if trigger.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        });
    }
    if trigger.len() > 1 {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: "multiple triggers".to_owned(),
        });
    }
    let (kind, _body) = trigger.first().ok_or_else(|| {
        ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        }
    })?;
    match kind.as_str() {
        "manual" | "ipc" => Ok(()),
        "http" => Err(ValidationError::HttpTriggerOutOfCore),
        other => Err(ValidationError::UnsupportedTrigger {
            trigger: other.to_owned(),
        }),
    }
}

/// Validates all step and top-level IDs against grammar, reserved words, and duplicates.
pub fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
    let name = doc.get_string("name").ok_or_else(|| {
        ValidationError::MissingRequiredField {
            field: "name".to_owned(),
        }
    })?;
    validate_id("name", name)?;
    let steps = doc.get_sequence("steps").ok_or_else(|| {
        ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        }
    })?;
    if steps.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(steps.len());
    for step in steps {
        let id = step.get_string("id").ok_or_else(|| {
            ValidationError::MissingRequiredField {
                field: "step id".to_owned(),
            }
        })?;
        validate_single_id(id, &seen)?;
        seen.push(id);
    }
    Ok(())
}

/// Validates step field shapes and the single-primitive constraint.
pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc.get_sequence("steps").ok_or_else(|| {
        ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        }
    })?;
    for step in steps {
        validate_step_unknown_fields(step)?;
        validate_single_primitive(step)?;
    }
    Ok(())
}

fn validate_required_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in REQUIRED_TOP_LEVEL_FIELDS {
        if !doc.has_field(field) {
            return Err(ValidationError::MissingRequiredField {
                field: (*field).to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_unknown_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in doc.field_names() {
        if !ALLOWED_TOP_LEVEL_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownTopLevelField);
        }
    }
    Ok(())
}

fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownStepField);
        }
    }
    Ok(())
}

/// Ensures exactly one primitive field per step.
pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let count = step
        .field_names()
        .iter()
        .filter(|field| STEP_PRIMITIVES.contains(field))
        .count();
    if count == 0 {
        return Err(ValidationError::MissingStepPrimitive);
    }
    if count > 1 {
        return Err(ValidationError::MultipleStepPrimitives);
    }
    Ok(())
}

fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId {
            id: format!("{field}: {id}"),
        });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId {
            id: format!("{field}: {id}"),
        });
    }
    Ok(())
}

fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId {
            id: id.to_owned(),
        });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId {
            id: id.to_owned(),
        });
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId {
            id: id.to_owned(),
        });
    }
    Ok(())
}

fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let first = id.as_bytes().first();
    let Some(&byte) = first else {
        return false;
    };
    if !byte.is_ascii_lowercase() {
        return false;
    }
    for byte in id.as_bytes() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'_' {
            return false;
        }
    }
    true
}

fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

// ---------------------------------------------------------------------------
// Lightweight document model for schema validation
// ---------------------------------------------------------------------------

/// Read-only view of a workflow document's top-level fields.
pub struct WorkflowDoc {
    fields: Vec<(String, FieldValue)>,
}

/// Value associated with a workflow field.
#[derive(Clone)]
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
#[derive(Clone)]
pub struct StepDoc {
    fields: Vec<(String, FieldValue)>,
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
        WorkflowDoc::from_pairs(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
        StepDoc::from_pairs(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    fn valid_workflow_doc() -> WorkflowDoc {
        make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![
                make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
            ])),
        ])
    }

    #[test]
    fn accepts_valid_workflow() {
        let doc = valid_workflow_doc();
        assert!(validate_workflow_schema(&doc).is_ok());
    }

    #[test]
    fn rejects_missing_version() {
        let doc = make_workflow(vec![
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![
                make_step(vec![("id", FieldValue::String("s1".to_owned())), ("finish", FieldValue::Empty)]),
            ])),
        ]);
        assert!(matches!(
            validate_workflow_schema(&doc),
            Err(ValidationError::MissingRequiredField { .. })
        ));
    }

    #[test]
    fn rejects_wrong_version() {
        let mut doc = valid_workflow_doc();
        doc.fields[0].1 = FieldValue::String("velvet-ballistics/v1".to_owned());
        assert!(matches!(
            validate_version(&doc),
            Err(ValidationError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn rejects_http_trigger() {
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![
                make_step(vec![("id", FieldValue::String("s1".to_owned())), ("finish", FieldValue::Empty)]),
            ])),
        ]);
        assert!(matches!(
            validate_trigger(&doc),
            Err(ValidationError::HttpTriggerOutOfCore)
        ));
    }

    #[test]
    fn rejects_invalid_id() {
        let result = validate_single_id("123bad", &[]);
        assert!(matches!(result, Err(ValidationError::InvalidId { .. })));
    }

    #[test]
    fn rejects_reserved_id() {
        let result = validate_single_id("runtime", &[]);
        assert!(matches!(result, Err(ValidationError::ReservedId { .. })));
    }

    #[test]
    fn rejects_duplicate_id() {
        let result = validate_single_id("step1", &["step1"]);
        assert!(matches!(result, Err(ValidationError::DuplicateId { .. })));
    }

    #[test]
    fn rejects_step_without_primitive() {
        let step = make_step(vec![("id", FieldValue::String("s1".to_owned()))]);
        assert!(matches!(
            validate_single_primitive(&step),
            Err(ValidationError::MissingStepPrimitive)
        ));
    }

    #[test]
    fn rejects_step_with_multiple_primitives() {
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("save", FieldValue::Empty),
            ("finish", FieldValue::Empty),
        ]);
        assert!(matches!(
            validate_single_primitive(&step),
            Err(ValidationError::MultipleStepPrimitives)
        ));
    }

    #[test]
    fn accepts_valid_id() {
        assert!(is_valid_id("step_1"));
        assert!(is_valid_id("a"));
        assert!(is_valid_id("abc_def_123"));
    }

    #[test]
    fn rejects_uppercase_id() {
        assert!(!is_valid_id("StepOne"));
    }

    #[test]
    fn rejects_id_starting_with_digit() {
        assert!(!is_valid_id("1step"));
    }

    #[test]
    fn rejects_too_long_id() {
        let long_id = "a".repeat(65);
        assert!(!is_valid_id(&long_id));
    }

    #[test]
    fn accepts_max_length_id() {
        let max_id = "a".repeat(64);
        assert!(is_valid_id(&max_id));
    }
}
