#![forbid(unsafe_code)]
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
    "id",
    "name",
    "if",
    "with",
    "then",
    "set",
    "choose",
    "for_each",
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "try_again",
];

const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "parallel", "collect", "aggregate", "repeat", "wait", "ask",
    "finish",
];

const RESERVED_IDS: &[&str] = &[
    "now",
    "random",
    "runtime",
    "null",
    "true",
    "false",
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "error",
    "attempt",
    "total",
    "result",
    "when",
    "item",
    "do",
    "set",
    "choose",
    "for_each",
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

/// Validates a workflow document against the v1 schema.
///
/// Checks required fields, unknown fields, version, trigger, IDs,
/// step fields, and single-primitive rules.
pub fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_duplicate_fields(doc)?;
    validate_required_fields(doc)?;
    validate_unknown_fields(doc)?;
    validate_version(doc)?;
    validate_trigger(doc)?;
    validate_ids(doc)?;
    validate_step_fields(doc)?;
    Ok(())
}

fn validate_duplicate_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_no_duplicate_names(&doc.fields)?;
    let Some(steps) = doc.get_sequence("steps") else {
        return Ok(());
    };
    for step in steps {
        validate_no_duplicate_names(&step.fields)?;
    }
    Ok(())
}

fn validate_no_duplicate_names(fields: &[(String, FieldValue)]) -> ValidationResult<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (name, _) in fields {
        if seen.contains(&name.as_str()) {
            return Err(ValidationError::DuplicateKey);
        }
        seen.push(name.as_str());
    }
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

/// Validates the trigger (when) block accepts canonical v1 triggers and rejects HTTP.
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
    let (kind, body) = trigger
        .first()
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        })?;
    match kind.as_str() {
        "manual" | "webhook" => validate_empty_trigger(kind, body),
        "schedule" => validate_named_string_trigger(kind, body, "cron"),
        "event" => validate_named_string_trigger(kind, body, "name"),
        "http" => Err(ValidationError::HttpTriggerOutOfCore),
        other => Err(ValidationError::UnsupportedTrigger {
            trigger: other.to_owned(),
        }),
    }
}

fn validate_empty_trigger(kind: &str, body: &FieldValue) -> ValidationResult<()> {
    match body {
        FieldValue::Empty => Ok(()),
        FieldValue::Mapping(entries) if entries.is_empty() => Ok(()),
        _ => Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        }),
    }
}

fn validate_named_string_trigger(
    kind: &str,
    body: &FieldValue,
    required_field: &str,
) -> ValidationResult<()> {
    let FieldValue::Mapping(entries) = body else {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        });
    };
    let valid = entries.iter().any(|(field, value)| match value {
        FieldValue::String(text) => field == required_field && !text.is_empty(),
        _ => false,
    });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        })
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
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) {
            count = count.saturating_add(1);
        }
    }
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
        assert_eq!(validate_workflow_schema(&doc), Ok(()));
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
            ("set", FieldValue::Empty),
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
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
            ("bogus_field", FieldValue::Empty),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns UnknownTopLevelField
        assert_eq!(result, Err(ValidationError::UnknownTopLevelField));
    }

    #[test]
    fn validate_workflow_schema_returns_duplicate_key_for_duplicate_top_level_field() {
        // Given a workflow doc with duplicate top-level "name" keys
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("first".to_owned())),
            ("name", FieldValue::String("second".to_owned())),
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
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns DuplicateKey exactly.
        assert_eq!(result, Err(ValidationError::DuplicateKey));
    }

    #[test]
    fn validate_workflow_schema_returns_duplicate_key_for_duplicate_step_field() {
        // Given a step with duplicate primitive keys
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("s1".to_owned())),
                    ("set", FieldValue::Empty),
                    ("set", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns DuplicateKey before primitive counting.
        assert_eq!(result, Err(ValidationError::DuplicateKey));
    }

    #[test]
    fn validate_workflow_schema_returns_unknown_step_field_for_invalid_step_field() {
        // Given a workflow doc where a step has a field not in ALLOWED_STEP_FIELDS
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                    ("nonsense", FieldValue::Empty),
                ])]),
            ),
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
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
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
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns MissingRequiredField with field "name"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "name".to_owned(),
            })
        );
    }

    #[test]
    fn validate_workflow_schema_returns_missing_required_field_for_absent_steps() {
        // Given a workflow doc without "steps"
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns MissingRequiredField with field "steps"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "steps".to_owned(),
            })
        );
    }

    #[test]
    fn validate_version_returns_invalid_version_for_bad_version() {
        // Given a workflow doc with a wrong version string
        let doc = make_workflow(vec![("version", FieldValue::String("2.0".to_owned()))]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with the exact version string
        assert_eq!(
            result,
            Err(ValidationError::InvalidVersion {
                version: "2.0".to_owned(),
            })
        );
    }

    #[test]
    fn validate_version_accepts_current_version() {
        // Given a workflow doc with the canonical version
        let doc = make_workflow(vec![(
            "version",
            FieldValue::String("velvet-ballastics/v1".to_owned()),
        )]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_version_rejects_empty_version() {
        // Given a workflow doc with an empty version string
        let doc = make_workflow(vec![("version", FieldValue::String(String::new()))]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with empty string
        assert_eq!(
            result,
            Err(ValidationError::InvalidVersion {
                version: String::new(),
            })
        );
    }

    #[test]
    fn validate_version_rejects_unknown_version() {
        // Given a workflow doc with an unknown version string
        let doc = make_workflow(vec![(
            "version",
            FieldValue::String("other-language/v2".to_owned()),
        )]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion with the exact version
        assert_eq!(
            result,
            Err(ValidationError::InvalidVersion {
                version: "other-language/v2".to_owned(),
            })
        );
    }

    #[test]
    fn validate_version_returns_missing_required_field_when_absent() {
        // Given a workflow doc with no version field
        let doc = make_workflow(vec![("name", FieldValue::String("test".to_owned()))]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns MissingRequiredField for "version"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "version".to_owned(),
            })
        );
    }

    #[test]
    fn validate_ids_returns_invalid_id_for_malformed_id() {
        // Given a workflow doc with a step whose id starts with a digit
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("1bad".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with the exact id
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "1bad".to_owned(),
            })
        );
    }

    #[test]
    fn validate_ids_returns_reserved_id_for_system_id() {
        // Given a workflow doc with a step using a reserved id
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("runtime".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId with the exact id
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "runtime".to_owned(),
            })
        );
    }

    #[test]
    fn validate_ids_returns_duplicate_id_for_same_step_id() {
        // Given a single id check with "step1" already seen
        let seen = vec!["step1"];
        // When validate_single_id is called with "step1" again
        let result = validate_single_id("step1", &seen);
        // Then it returns DuplicateId with exact id
        assert_eq!(
            result,
            Err(ValidationError::DuplicateId {
                id: "step1".to_owned(),
            })
        );
    }

    #[test]
    fn validate_single_primitive_returns_multiple_step_primitives_for_two_primitives() {
        // Given a step with both "set" and "finish"
        let step = make_step(vec![
            ("id", FieldValue::String("s1".to_owned())),
            ("set", FieldValue::Empty),
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
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("my_workflow".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![
                    make_step(vec![
                        ("id", FieldValue::String("step_one".to_owned())),
                        ("finish", FieldValue::Empty),
                    ]),
                    make_step(vec![
                        ("id", FieldValue::String("step_two".to_owned())),
                        ("finish", FieldValue::Empty),
                    ]),
                ]),
            ),
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
                    ("id", FieldValue::String("has space".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "has space".to_owned(),
            })
        );
    }

    #[test]
    fn validate_ids_rejects_step_id_starting_with_digit() {
        // Given a workflow doc with a step id starting with a digit
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("9lead".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with exact id
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "9lead".to_owned(),
            })
        );
    }

    #[test]
    fn validate_ids_rejects_step_id_with_special_chars() {
        // Given a workflow doc with a step id containing a dash
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("bad-id".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns InvalidId with exact id
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "bad-id".to_owned(),
            })
        );
    }

    #[test]
    fn validate_trigger_rejects_ipc_trigger() {
        // Given a workflow doc with a legacy ipc trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("ipc".to_owned(), FieldValue::Empty)]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger with the exact trigger name
        assert_eq!(
            result,
            Err(ValidationError::UnsupportedTrigger {
                trigger: "ipc".to_owned(),
            })
        );
    }

    #[test]
    fn validate_trigger_accepts_schedule_trigger() {
        // Given a workflow doc with a schedule trigger carrying cron
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "schedule".to_owned(),
                FieldValue::Mapping(vec![(
                    "cron".to_owned(),
                    FieldValue::String("0 0 * * *".to_owned()),
                )]),
            )]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_accepts_event_trigger() {
        // Given a workflow doc with an event trigger carrying name
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "event".to_owned(),
                FieldValue::Mapping(vec![(
                    "name".to_owned(),
                    FieldValue::String("job.created".to_owned()),
                )]),
            )]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_accepts_webhook_trigger() {
        // Given a workflow doc with a webhook trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("webhook".to_owned(), FieldValue::Mapping(vec![]))]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_rejects_event_without_name() {
        // Given a workflow doc with an event trigger missing name
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("event".to_owned(), FieldValue::Mapping(vec![]))]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger for event
        assert_eq!(
            result,
            Err(ValidationError::UnsupportedTrigger {
                trigger: "event".to_owned(),
            })
        );
    }

    #[test]
    fn validate_trigger_accepts_manual_trigger() {
        // Given a workflow doc with a manual trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_trigger_rejects_unsupported_trigger() {
        // Given a workflow doc with a cron trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("cron".to_owned(), FieldValue::Empty)]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger with the exact trigger name
        assert_eq!(
            result,
            Err(ValidationError::UnsupportedTrigger {
                trigger: "cron".to_owned(),
            })
        );
    }

    #[test]
    fn validate_trigger_rejects_empty_when_mapping() {
        // Given a workflow doc with an empty when mapping
        let doc = make_workflow(vec![("when", FieldValue::Mapping(vec![]))]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns MissingRequiredField for "when"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "when".to_owned(),
            })
        );
    }

    #[test]
    fn validate_step_fields_accepts_valid_do_step() {
        // Given a workflow doc with a step that has a "do" primitive
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("do", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_accepts_valid_set_step() {
        // Given a workflow doc with a step that has a "set" primitive
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("set", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_accepts_master_metadata_fields() {
        // Given a step using the master v1 metadata fields around one primitive
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("name", FieldValue::String("Step One".to_owned())),
                ("if", FieldValue::String("$input.enabled".to_owned())),
                ("with", FieldValue::Mapping(vec![])),
                ("try_again", FieldValue::Mapping(vec![])),
                ("on_error", FieldValue::String("fail".to_owned())),
                ("then", FieldValue::String("done".to_owned())),
                ("set", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then the metadata fields are accepted and not counted as primitives.
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_rejects_legacy_save_field() {
        // Given a step using the obsolete "save" field instead of master v1 "set"
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("save", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns UnknownStepField exactly.
        assert_eq!(result, Err(ValidationError::UnknownStepField));
    }

    #[test]
    fn validate_step_fields_accepts_valid_branch_step() {
        // Given a workflow doc with a step that has a "choose" primitive
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("choose", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_step_fields_rejects_step_without_kind() {
        // Given a workflow doc with a step that has no primitive field
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![(
                "id",
                FieldValue::String("s1".to_owned()),
            )])]),
        )]);
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
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "version".to_owned(),
            })
        );
    }

    // ---------------------------------------------------------------------------
    // Accessor and query tests
    // ---------------------------------------------------------------------------

    #[test]
    fn get_string_returns_some_for_existing_string_field() {
        // Given a workflow doc with a string field "name"
        let doc = make_workflow(vec![("name", FieldValue::String("hello".to_owned()))]);
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
        let doc = make_workflow(vec![("name", FieldValue::Mapping(vec![]))]);
        // When get_string is called for "name"
        let result = doc.get_string("name");
        // Then it returns None
        assert_eq!(result, None);
    }

    #[test]
    fn get_mapping_returns_some_for_existing_mapping() {
        // Given a workflow doc with a mapping field "when"
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
        )]);
        // When get_mapping is called
        let result = doc.get_mapping("when");
        // Then it returns Some with the mapping entries
        assert!(result.is_some());
        let Some(mapping) = result else {
            return;
        };
        assert_eq!(mapping.len(), 1);
        let Some((field, _)) = mapping.first() else {
            return;
        };
        assert_eq!(field, "manual");
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
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![(
                "id",
                FieldValue::String("s1".to_owned()),
            )])]),
        )]);
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
        let doc = make_workflow(vec![("name", FieldValue::String("test".to_owned()))]);
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
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
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
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![(
                "manual".to_owned(),
                FieldValue::String("test".to_owned()),
            )]),
        )]);
        // When get_mapping is called
        let result = doc.get_mapping("when");
        // Then it returns the mapping with the nested value
        assert!(result.is_some());
        let Some(mapping) = result else {
            return;
        };
        assert_eq!(mapping.len(), 1);
        let Some((field, value)) = mapping.first() else {
            return;
        };
        assert_eq!(field, "manual");
        let FieldValue::String(s) = value else {
            return;
        };
        assert_eq!(s, "test");
    }

    #[test]
    fn get_sequence_with_multiple_entries_returns_correct_one() {
        // Given a workflow doc with multiple steps in sequence
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![
                make_step(vec![("id", FieldValue::String("s1".to_owned()))]),
                make_step(vec![("id", FieldValue::String("s2".to_owned()))]),
            ]),
        )]);
        // When get_sequence is called
        let result = doc.get_sequence("steps");
        // Then it returns both steps in order
        assert!(result.is_some());
        let Some(seq) = result else {
            return;
        };
        assert_eq!(seq.len(), 2);
        let Some(first) = seq.first() else {
            return;
        };
        let Some(second) = seq.get(1) else {
            return;
        };
        assert_eq!(first.get_string("id"), Some("s1"));
        assert_eq!(second.get_string("id"), Some("s2"));
    }

    #[test]
    fn field_names_returns_correct_fields_for_workflow() {
        // Given a workflow doc with known fields
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
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
        let step = make_step(vec![("id", FieldValue::String("my_step".to_owned()))]);
        // When get_string is called
        let result = step.get_string("id");
        // Then it returns Some with exact value
        assert_eq!(result, Some("my_step"));
    }

    #[test]
    fn step_doc_get_string_returns_none_for_missing() {
        // Given a step with no "id" field
        let step = make_step(vec![("finish", FieldValue::Empty)]);
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
            (
                "version".to_owned(),
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            (
                "name".to_owned(),
                FieldValue::String("roundtrip".to_owned()),
            ),
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

    // ---------------------------------------------------------------------------
    // Adversarial BDD tests: validation bypass attacks
    // ---------------------------------------------------------------------------

    #[test]
    fn adversarial_version_v2_is_rejected_as_invalid_version() {
        // Given a workflow doc with version "velvet-ballistics/v2"
        let doc = make_workflow(vec![(
            "version",
            FieldValue::String("velvet-ballistics/v2".to_owned()),
        )]);
        // When validate_version is called
        let result = validate_version(&doc);
        // Then it returns InvalidVersion (E0106) -- the name is wrong too
        assert_eq!(
            result,
            Err(ValidationError::InvalidVersion {
                version: "velvet-ballistics/v2".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_input_is_rejected_as_reserved() {
        // Given a workflow with a step using id "input"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("input".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108) for "input"
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "input".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_vars_is_rejected_as_reserved() {
        // Given a workflow with a step using id "vars"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("vars".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "vars".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_secrets_is_rejected_as_reserved() {
        // Given a workflow with a step using id "secrets"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("secrets".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "secrets".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_steps_is_rejected_as_reserved() {
        // Given a workflow with a step using id "steps"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("steps".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "steps".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_error_is_rejected_as_reserved() {
        // Given a workflow with a step using id "error"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("error".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "error".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_attempt_is_rejected_as_reserved() {
        // Given a workflow with a step using id "attempt"
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("attempt".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "attempt".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_step_with_set_and_do_primitives_is_rejected() {
        // Given a step with BOTH "set" and "do" primitives
        let step = make_step(vec![
            ("id", FieldValue::String("sneaky".to_owned())),
            ("set", FieldValue::Empty),
            ("do", FieldValue::Empty),
        ]);
        // When validate_single_primitive is called
        let result = validate_single_primitive(&step);
        // Then it returns MultipleStepPrimitives (E010A)
        assert_eq!(result, Err(ValidationError::MultipleStepPrimitives));
    }

    #[test]
    fn adversarial_step_with_choose_and_finish_primitives_is_rejected() {
        // Given a step with BOTH "choose" and "finish"
        let step = make_step(vec![
            ("id", FieldValue::String("dual_action".to_owned())),
            ("choose", FieldValue::Empty),
            ("finish", FieldValue::Empty),
        ]);
        // When validate_single_primitive is called
        let result = validate_single_primitive(&step);
        // Then it returns MultipleStepPrimitives (E010A)
        assert_eq!(result, Err(ValidationError::MultipleStepPrimitives));
    }

    #[test]
    fn adversarial_step_with_all_primitives_is_rejected() {
        // Given a step with ALL primitives at once
        let step = make_step(vec![
            ("id", FieldValue::String("greedy".to_owned())),
            ("set", FieldValue::Empty),
            ("choose", FieldValue::Empty),
            ("for_each", FieldValue::Empty),
            ("parallel", FieldValue::Empty),
            ("collect", FieldValue::Empty),
            ("aggregate", FieldValue::Empty),
            ("repeat", FieldValue::Empty),
            ("wait", FieldValue::Empty),
            ("ask", FieldValue::Empty),
            ("finish", FieldValue::Empty),
            ("do", FieldValue::Empty),
        ]);
        // When validate_single_primitive is called
        let result = validate_single_primitive(&step);
        // Then it returns MultipleStepPrimitives (E010A)
        assert_eq!(result, Err(ValidationError::MultipleStepPrimitives));
    }

    #[test]
    fn adversarial_step_with_only_non_primitive_fields_is_rejected() {
        // Given a step with only id, name, then, on_error, retry -- no primitive
        let step = make_step(vec![
            ("id", FieldValue::String("no_op".to_owned())),
            ("name", FieldValue::String("No Operation".to_owned())),
            ("then", FieldValue::String("next_step".to_owned())),
            ("on_error", FieldValue::Empty),
            ("retry", FieldValue::Empty),
        ]);
        // When validate_single_primitive is called
        let result = validate_single_primitive(&step);
        // Then it returns MissingStepPrimitive (E010B)
        assert_eq!(result, Err(ValidationError::MissingStepPrimitive));
    }

    #[test]
    fn adversarial_http_trigger_is_rejected_as_out_of_core() {
        // Given a workflow with an HTTP trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns HttpTriggerOutOfCore (E040C)
        assert_eq!(result, Err(ValidationError::HttpTriggerOutOfCore));
    }

    #[test]
    fn adversarial_duplicate_step_ids_in_full_workflow_is_rejected() {
        // Given a workflow with two steps sharing the same id
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("dup_test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![
                    make_step(vec![
                        ("id", FieldValue::String("clone".to_owned())),
                        ("set", FieldValue::Empty),
                    ]),
                    make_step(vec![
                        ("id", FieldValue::String("clone".to_owned())),
                        ("finish", FieldValue::Empty),
                    ]),
                ]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns DuplicateId (E0109)
        assert_eq!(
            result,
            Err(ValidationError::DuplicateId {
                id: "clone".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_uppercase_step_id_is_rejected() {
        // Given a workflow with an uppercase step id
        let result = validate_single_id("MyStep", &[]);
        // Then it returns InvalidId (E0107)
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "MyStep".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_hyphenated_step_id_is_rejected() {
        // Given a step id with hyphens
        let result = validate_single_id("my-step", &[]);
        // Then it returns InvalidId (E0107)
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "my-step".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_step_id_starting_with_digit_is_rejected() {
        // Given a step id starting with a digit
        let result = validate_single_id("0step", &[]);
        // Then it returns InvalidId (E0107)
        assert_eq!(
            result,
            Err(ValidationError::InvalidId {
                id: "0step".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_empty_step_id_is_rejected() {
        // Given an empty step id
        let result = validate_single_id("", &[]);
        // Then it returns InvalidId (E0107)
        assert_eq!(
            result,
            Err(ValidationError::InvalidId { id: String::new() })
        );
    }

    #[test]
    fn adversarial_multiple_triggers_are_rejected() {
        // Given a workflow with both manual and schedule triggers at once
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![
                ("manual".to_owned(), FieldValue::Empty),
                ("schedule".to_owned(), FieldValue::Mapping(vec![])),
            ]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger (E040B) for "multiple triggers"
        assert_eq!(
            result,
            Err(ValidationError::UnsupportedTrigger {
                trigger: "multiple triggers".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_unknown_trigger_kind_is_rejected() {
        // Given a workflow with an unknown timer trigger
        let doc = make_workflow(vec![(
            "when",
            FieldValue::Mapping(vec![("timer".to_owned(), FieldValue::Empty)]),
        )]);
        // When validate_trigger is called
        let result = validate_trigger(&doc);
        // Then it returns UnsupportedTrigger (E040B)
        assert_eq!(
            result,
            Err(ValidationError::UnsupportedTrigger {
                trigger: "timer".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_empty_steps_sequence_is_rejected() {
        // Given a workflow with an empty steps array
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("empty_steps".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            ("steps", FieldValue::Sequence(vec![])),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns MissingRequiredField for "steps"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "steps".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_unknown_top_level_field_webhook_is_rejected() {
        // Given a workflow with a top-level "webhook" field
        let doc = make_workflow(vec![
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
                    ("id", FieldValue::String("s1".to_owned())),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
            ("webhook", FieldValue::Empty),
        ]);
        // When validate_workflow_schema is called
        let result = validate_workflow_schema(&doc);
        // Then it returns UnknownTopLevelField (E0103)
        assert_eq!(result, Err(ValidationError::UnknownTopLevelField));
    }

    #[test]
    fn adversarial_unknown_step_field_payload_is_rejected() {
        // Given a step with an unknown field "payload"
        let doc = make_workflow(vec![(
            "steps",
            FieldValue::Sequence(vec![make_step(vec![
                ("id", FieldValue::String("s1".to_owned())),
                ("finish", FieldValue::Empty),
                ("payload", FieldValue::Empty),
            ])]),
        )]);
        // When validate_step_fields is called
        let result = validate_step_fields(&doc);
        // Then it returns UnknownStepField (E0104)
        assert_eq!(result, Err(ValidationError::UnknownStepField));
    }

    #[test]
    fn adversarial_reserved_id_result_is_rejected_in_step() {
        // Given a step using id "result" -- now reserved per the master doc
        let result = validate_single_id("result", &[]);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "result".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_when_is_rejected_in_step() {
        // Given a step using id "when" -- now reserved per the master doc
        let result = validate_single_id("when", &[]);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "when".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_reserved_id_item_is_rejected_in_step() {
        // Given a step using id "item" -- now reserved per the master doc
        let result = validate_single_id("item", &[]);
        // Then it returns ReservedId (E0108)
        assert_eq!(
            result,
            Err(ValidationError::ReservedId {
                id: "item".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_step_without_id_field_is_rejected() {
        // Given a workflow step with no "id" field at all
        let doc = make_workflow(vec![
            (
                "version",
                FieldValue::String("velvet-ballastics/v1".to_owned()),
            ),
            ("name", FieldValue::String("no_id_test".to_owned())),
            (
                "when",
                FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)]),
            ),
            (
                "steps",
                FieldValue::Sequence(vec![make_step(vec![("finish", FieldValue::Empty)])]),
            ),
        ]);
        // When validate_ids is called
        let result = validate_ids(&doc);
        // Then it returns MissingRequiredField for "step id"
        assert_eq!(
            result,
            Err(ValidationError::MissingRequiredField {
                field: "step id".to_owned(),
            })
        );
    }
}
