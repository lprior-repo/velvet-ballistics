//! Schema validation for workflow documents.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

use crate::{ValidationError, ValidationResult};

const CANONICAL_VERSION: &str = "velvet-ballastics/v1";

const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];

const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version", "name", "when", "inputs", "vars", "secrets", "result", "examples", "steps",
];

const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id", "name", "then", "save", "choose", "for_each", "together", "collect", "reduce", "repeat",
    "wait", "ask", "finish", "do", "on_error", "retry",
];

const STEP_PRIMITIVES: &[&str] = &[
    "save", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish", "do",
];

const RESERVED_IDS: &[&str] = &[
    "now", "random", "runtime", "null", "true", "false", "input", "vars", "secrets", "steps",
    "error", "attempt", "total",
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
    let trigger = doc
        .get_mapping("when")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
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
    let (kind, _body) = trigger
        .first()
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
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
    let name = doc
        .get_string("name")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "name".to_owned(),
        })?;
    validate_id("name", name)?;
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        })?;
    if steps.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(steps.len());
    for step in steps {
        let id = step
            .get_string("id")
            .ok_or_else(|| ValidationError::MissingRequiredField {
                field: "step id".to_owned(),
            })?;
        validate_single_id(id, &seen)?;
        seen.push(id);
    }
    Ok(())
}

/// Validates step field shapes and the single-primitive constraint.
pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
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
        return Err(ValidationError::InvalidId { id: id.to_owned() });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId { id: id.to_owned() });
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId { id: id.to_owned() });
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
        WorkflowDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn make_step(fields: Vec<(&str, FieldValue)>) -> StepDoc {
        StepDoc::from_pairs(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn valid_workflow_doc() -> WorkflowDoc {
        make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("step1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
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
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        assert!(matches!(
            validate_workflow_schema(&doc),
            Err(ValidationError::MissingRequiredField { .. })
        ));
    }

    #[test]
    fn rejects_wrong_version() {
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("other-language/v1".to_owned()),
            ),
            ("name", FieldValue::String("test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        assert!(matches!(
            validate_version(&doc),
            Err(ValidationError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn rejects_http_trigger() {
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
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

    // ---------------------------------------------------------------------------
    // BDD exact-assertion tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_workflow_schema_returns_unknown_top_level_field_for_invalid_field() {
        // Given a workflow doc with a field not in ALLOWED_TOP_LEVEL_FIELDS
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
            ("bogus_field", FieldValue::Empty),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns UnknownTopLevelField
        assert_eq!(result, Err(ValidationError::UnknownTopLevelField));
    }

    #[test]
    fn validate_workflow_schema_returns_unknown_step_field_for_invalid_step_field() {
        // Given a workflow doc where a step has a field not in ALLOWED_STEP_FIELDS
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
                ("nonsense", FieldValue::Empty),
            ])])),
        ]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns UnknownStepField
        assert_eq!(result, Err(ValidationError::UnknownStepField));
    }

    #[test]
    fn validate_workflow_schema_returns_missing_required_field_for_absent_name() {
        // Given a workflow doc without "name"
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns MissingRequiredField with field "name"
        assert_eq!(result, Err(ValidationError::MissingRequiredField {
            field: "name".to_owned(),
        }));
    }

    #[test]
    fn validate_workflow_schema_returns_missing_required_field_for_absent_steps() {
        // Given a workflow doc without "steps"
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns MissingRequiredField with field "steps"
        assert_eq!(result, Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        }));
    }

    #[test]
    fn validate_version_returns_invalid_version_for_bad_version() {
        // Given a workflow doc with a wrong version string
        let doc = make_workflow(vec![
            ("version", FieldValue::String("2.0".to_owned())),
        ]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with the exact version string
        assert_eq!(result, Err(ValidationError::InvalidVersion {
            version: "2.0".to_owned(),
        }));
    }

    #[test]
    fn validate_version_accepts_current_version() {
        // Given a workflow doc with the canonical version
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
        ]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_version_rejects_empty_version() {
        // Given a workflow doc with an empty version string
        let doc = make_workflow(vec![
            ("version", FieldValue::String(String::new())),
        ]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with empty string
        assert_eq!(result, Err(ValidationError::InvalidVersion {
            version: String::new(),
        }));
    }

    #[test]
    fn validate_version_rejects_unknown_version() {
        // Given a workflow doc with an unknown version string
        let doc = make_workflow(vec![
            ("version", FieldValue::String("other-language/v2".to_owned())),
        ]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with the exact version
        assert_eq!(result, Err(ValidationError::InvalidVersion {
            version: "other-language/v2".to_owned(),
        }));
    }

    #[test]
    fn validate_version_returns_missing_required_field_when_absent() {
        // Given a workflow doc with no version field
        let doc = make_workflow(vec![
            ("name", FieldValue::String("test".to_owned())),
        ]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns MissingRequiredField for "version"
        assert_eq!(result, Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        }));
    }

    #[test]
    fn validate_ids_returns_invalid_id_for_malformed_id() {
        // Given a workflow doc with a step whose id starts with a digit
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("1bad".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with the exact id
        assert_eq!(result, Err(ValidationError::InvalidId {
            id: "1bad".to_owned(),
        }));
    }

    #[test]
    fn validate_ids_returns_reserved_id_for_system_id() {
        // Given a workflow doc with a step using a reserved id
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("runtime".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId with the exact id
        assert_eq!(result, Err(ValidationError::ReservedId {
            id: "runtime".to_owned(),
        }));
    }

    #[test]
    fn validate_ids_returns_duplicate_id_for_same_step_id() {
        // Given a single id check with "step1" already seen
        let seen = vec!["step1"];
        // When validate_single_id is called with "step1" again
        let result = validate_single_id("step1", &seen);
        // Then it returns DuplicateId with exact id
        assert_eq!(result, Err(ValidationError::DuplicateId {
            id: "step1".to_owned(),
        }));
    }

    #[test]
    fn validate_single_primitive_returns_multiple_step_primitives_for_two_primitives() {
        // Given a step with both "save" and "finish"
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("save", FieldValue::Empty),
            ("finish", FieldValue::Empty),
        ]);
        // When validate_single_primitive is called
        let result = validate_single_primitive(&step);
        // Then it returns MultipleStepPrimitives
        assert_eq!(result, Err(ValidationError::MultipleStepPrimitives));
    }

    #[test]
    fn validate_ids_accepts_valid_step_ids() {
        // Given a workflow doc with valid IDs
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("my_workflow".to_owned())),
            ("when", FieldValue::Mapping(vec![("ipc".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![
                make_step(vec![
                    ("id", FieldValue::String("step_one".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
                make_step(vec![
                    ("id", FieldValue::String("step_two".to_owned())),
                    ("finish", FieldValue::Empty),
                ]),
            ])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_ids_rejects_step_id_with_spaces() {
        // Given a workflow doc with a step id containing spaces
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("has space".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId
        assert_eq!(result, Err(ValidationError::InvalidId {
            id: "has space".to_owned(),
        }));
    }

    #[test]
    fn validate_ids_rejects_step_id_starting_with_digit() {
        // Given a workflow doc with a step id starting with a digit
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("9lead".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with exact id
        assert_eq!(result, Err(ValidationError::InvalidId {
            id: "9lead".to_owned(),
        }));
    }

    #[test]
    fn validate_ids_rejects_step_id_with_special_chars() {
        // Given a workflow doc with a step id containing a dash
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("bad-id".to_owned())),
                ("finish", FieldValue::Empty),
            ])])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with exact id
        assert_eq!(result, Err(ValidationError::InvalidId {
            id: "bad-id".to_owned(),
        }));
    }

    #[test]
    fn validate_trigger_accepts_ipc_trigger() {
        // Given a workflow doc with an ipc trigger
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![("ipc".to_owned(), FieldValue::Empty)])),
        ]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_accepts_manual_trigger() {
        // Given a workflow doc with a manual trigger
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
        ]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_rejects_unsupported_trigger() {
        // Given a workflow doc with a cron trigger
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::Empty)])),
        ]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger with the exact trigger name
        assert_eq!(result, Err(ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
        }));
    }

    #[test]
    fn validate_trigger_rejects_empty_when_mapping() {
        // Given a workflow doc with an empty when mapping
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![])),
        ]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns MissingRequiredField for "when"
        assert_eq!(result, Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        }));
    }

    #[test]
    fn validate_step_fields_accepts_valid_do_step() {
        // Given a workflow doc with a step that has a "do" primitive
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("do", FieldValue::Empty),
            ])])),
        ]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_accepts_valid_set_step() {
        // Given a workflow doc with a step that has a "save" primitive (set)
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("save", FieldValue::Empty),
            ])])),
        ]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_accepts_valid_branch_step() {
        // Given a workflow doc with a step that has a "choose" primitive
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("choose", FieldValue::Empty),
            ])])),
        ]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_rejects_step_without_kind() {
        // Given a workflow doc with a step that has no primitive field
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
            ])])),
        ]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns MissingStepPrimitive
        assert_eq!(result, Err(ValidationError::MissingStepPrimitive));
    }

    #[test]
    fn validate_workflow_schema_accepts_minimal_valid_workflow() {
        // Given a minimal valid workflow document
        let doc = valid_workflow_doc();
        // When validate_workflow_schema is called end-to-end
        let result = validate_workflow_schema(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_workflow_schema_rejects_empty_workflow() {
        // Given a workflow doc with no fields at all
        let doc = make_workflow(vec![]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns MissingRequiredField for the first required field ("version")
        assert_eq!(result, Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        }));
    }

    // ---------------------------------------------------------------------------
    // Accessor and query tests
    // ---------------------------------------------------------------------------

    #[test]
    fn get_string_returns_some_for_existing_string_field() {
        // Given a workflow doc with a string field "name"
        let doc = make_workflow(vec![
            ("name", FieldValue::String("hello".to_owned())),
        ]);
        // When get_string is called
        let result = doc.get_string("name");
        // Then it returns Some with the exact value
        assert_eq!(result, Some("hello"));
    }

    #[test]
    fn get_string_returns_none_for_missing_field() {
        // Given a workflow doc without "name"
        let doc = make_workflow(vec![]);
        // When get_string is called for "name"
        let result = doc.get_string("name");
        // Then it returns None
        assert_eq!(result, None);
    }

    #[test]
    fn get_string_returns_none_for_non_string_field() {
        // Given a workflow doc where "name" is a mapping, not a string
        let doc = make_workflow(vec![
            ("name", FieldValue::Mapping(vec![])),
        ]);
        // When get_string is called for "name"
        let result = doc.get_string("name");
        // Then it returns None
        assert_eq!(result, None);
    }

    #[test]
    fn get_mapping_returns_some_for_existing_mapping() {
        // Given a workflow doc with a mapping field "when"
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])),
        ]);
        // When get_mapping is called
        let result = doc.get_mapping("when");
        // Then it returns Some with the mapping entries
        assert!(result.is_some());
        let Some(mapping) = result else {
            return;
        };
        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping[0].0, "manual");
    }

    #[test]
    fn get_mapping_returns_none_for_missing_field() {
        // Given a workflow doc without "when"
        let doc = make_workflow(vec![]);
        // When get_mapping is called for "when"
        let result = doc.get_mapping("when");
        // Then it returns None
        assert!(result.is_none());
    }

    #[test]
    fn get_sequence_returns_some_for_existing_sequence() {
        // Given a workflow doc with a sequence field "steps"
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
            ])])),
        ]);
        // When get_sequence is called
        let result = doc.get_sequence("steps");
        // Then it returns Some with the steps
        assert!(result.is_some());
        let Some(seq) = result else {
            return;
        };
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn get_sequence_returns_none_for_missing_field() {
        // Given a workflow doc without "steps"
        let doc = make_workflow(vec![]);
        // When get_sequence is called for "steps"
        let result = doc.get_sequence("steps");
        // Then it returns None
        assert!(result.is_none());
    }

    #[test]
    fn has_field_returns_true_for_existing_field() {
        // Given a workflow doc with a "name" field
        let doc = make_workflow(vec![
            ("name", FieldValue::String("test".to_owned())),
        ]);
        // When has_field is called for "name"
        let result = doc.has_field("name");
        // Then it returns true
        assert!(result);
    }

    #[test]
    fn has_field_returns_false_for_missing_field() {
        // Given a workflow doc without "name"
        let doc = make_workflow(vec![]);
        // When has_field is called for "name"
        let result = doc.has_field("name");
        // Then it returns false
        assert!(!result);
    }

    #[test]
    fn get_string_with_multiple_fields_returns_correct_one() {
        // Given a workflow doc with multiple string fields
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("multi_test".to_owned())),
        ]);
        // When get_string is called for each
        let version = doc.get_string("version");
        let name = doc.get_string("name");
        // Then each returns its exact value
        assert_eq!(version, Some("velvet-ballastics/v1"));
        assert_eq!(name, Some("multi_test"));
    }

    #[test]
    fn get_mapping_with_nested_data_returns_correct_mapping() {
        // Given a workflow doc with a mapping containing nested data
        let doc = make_workflow(vec![
            ("when", FieldValue::Mapping(vec![
                ("manual".to_owned(), FieldValue::String("test".to_owned())),
            ])),
        ]);
        // When get_mapping is called
        let result = doc.get_mapping("when");
        // Then it returns the mapping with the nested value
        assert!(result.is_some());
        let Some(mapping) = result else {
            return;
        };
        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping[0].0, "manual");
        let FieldValue::String(ref s) = mapping[0].1 else {
            return;
        };
        assert_eq!(s, "test");
    }

    #[test]
    fn get_sequence_with_multiple_entries_returns_correct_one() {
        // Given a workflow doc with multiple steps in sequence
        let doc = make_workflow(vec![
            ("steps", FieldValue::Sequence(vec![
                make_step(vec![("id", FieldValue::String("s1".to_owned()))]),
                make_step(vec![("id", FieldValue::String("s2".to_owned()))]),
            ])),
        ]);
        // When get_sequence is called
        let result = doc.get_sequence("steps");
        // Then it returns both steps in order
        assert!(result.is_some());
        let Some(seq) = result else {
            return;
        };
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].get_string("id"), Some("s1"));
        assert_eq!(seq[1].get_string("id"), Some("s2"));
    }

    #[test]
    fn field_names_returns_correct_fields_for_workflow() {
        // Given a workflow doc with known fields
        let doc = make_workflow(vec![
            ("version", FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name", FieldValue::String("test".to_owned())),
        ]);
        // When field_names is called
        let names = doc.field_names();
        // Then it returns exactly the field names
        assert_eq!(names, vec!["version", "name"]);
    }

    #[test]
    fn step_doc_get_string_returns_value_for_existing_field() {
        // Given a step with an "id" string field
        let step = make_step(vec![
            ("id", FieldValue::String("my_step".to_owned())),
        ]);
        // When get_string is called
        let result = step.get_string("id");
        // Then it returns Some with exact value
        assert_eq!(result, Some("my_step"));
    }

    #[test]
    fn step_doc_get_string_returns_none_for_missing() {
        // Given a step with no "id" field
        let step = make_step(vec![
            ("finish", FieldValue::Empty),
        ]);
        // When get_string is called for "id"
        let result = step.get_string("id");
        // Then it returns None
        assert_eq!(result, None);
    }

    #[test]
    fn step_doc_field_names_returns_all_names() {
        // Given a step with multiple fields
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("finish", FieldValue::Empty),
        ]);
        // When field_names is called
        let names = step.field_names();
        // Then it returns both names
        assert_eq!(names, vec!["id", "finish"]);
    }

    #[test]
    fn from_pairs_creates_workflow_with_given_pairs() {
        // Given a set of field pairs
        let pairs = vec![
            ("version".to_owned(), FieldValue::String("velvet-ballastics/v1".to_owned())),
            ("name".to_owned(), FieldValue::String("roundtrip".to_owned())),
        ];
        // When WorkflowDoc::from_pairs is called
        let doc = WorkflowDoc::from_pairs(pairs);
        // Then the fields are accessible
        assert_eq!(doc.get_string("version"), Some("velvet-ballastics/v1"));
        assert_eq!(doc.get_string("name"), Some("roundtrip"));
    }

    #[test]
    fn from_pairs_creates_step_with_given_pairs() {
        // Given a set of field pairs for a step
        let pairs = vec![
            ("id".to_owned(), FieldValue::String("s1".to_owned())),
            ("do".to_owned(), FieldValue::Empty),
        ];
        // When StepDoc::from_pairs is called
        let step = StepDoc::from_pairs(pairs);
        // Then the fields are accessible
        assert_eq!(step.get_string("id"), Some("s1"));
        assert_eq!(step.field_names(), vec!["id", "do"]);
    }
}
