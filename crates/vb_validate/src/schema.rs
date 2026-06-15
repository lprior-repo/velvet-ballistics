#![forbid(unsafe_code)]
//! Schema validation for workflow documents.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

use crate::{ValidationError, ValidationResult};

const CANONICAL_VERSION: &str = "velvet-ballistics/v1";

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
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "try_again",
];

const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
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
    "collect",
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
        "event" => validate_named_string_trigger(kind, body, "type"),
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
        validate_step_body(step)?;
    }
    Ok(())
}

/// Validates that a step's primitive body is structurally sound.
///
/// Each primitive field should have a non-empty body that matches the
/// expected structure. This emits type-specific [`ValidationError`]
/// variants (e.g., `InvalidChoose`, `InvalidForEach`) rather than
/// generic schema errors, enabling precise diagnostics.
///
/// An `Empty` body is accepted as a valid marker (the YAML parser may
/// produce `Empty` for null/unspecified primitives); structural checks
/// only apply to non-empty bodies.
fn validate_step_body(step: &StepDoc) -> ValidationResult<()> {
    let Some((primitive, value)) = step.primitive_value() else {
        return Ok(());
    };
    if *value == FieldValue::Empty {
        return Ok(());
    }
    match primitive {
        "choose" => {
            // choose body must be a Mapping with `branches:` or `default:`
            if let FieldValue::Mapping(entries) = value {
                let has_branches = entries.iter().any(|(k, _)| k == "branches");
                let has_default = entries.iter().any(|(k, _)| k == "default");
                if !has_branches && !has_default {
                    return Err(ValidationError::InvalidChoose);
                }
            }
        }
        "for_each" => {
            // for_each body must be a Mapping with `for_each:` and body steps
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "for_each" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidForEach);
                }
            }
        }
        "together" => {
            // together body must be a Sequence of steps
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "together" && matches!(v, FieldValue::Sequence(_)));
                if !has_body {
                    return Err(ValidationError::InvalidTogether);
                }
            }
        }
        "collect" => {
            // collect body must have `of:` and `reduce:`
            if let FieldValue::Mapping(entries) = value {
                let has_of = entries.iter().any(|(k, _)| k == "of");
                let has_reduce = entries.iter().any(|(k, _)| k == "reduce");
                if !has_of || !has_reduce {
                    return Err(ValidationError::InvalidCollect);
                }
            }
        }
        "reduce" => {
            // reduce body must have `reduce:` and body steps
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "reduce" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidReduce);
                }
            }
        }
        "repeat" => {
            // repeat body must be a Mapping with `repeat:` and body steps
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "repeat" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidRepeat);
                }
            }
        }
        "wait" => {
            // wait body must be a Mapping with timeout or event
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidWait);
            }
        }
        "ask" => {
            // ask body must be a Mapping with prompt fields
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidAsk);
            }
        }
        "finish" => {
            // finish body must be a Mapping with `result:`
            if let FieldValue::Mapping(entries) = value {
                let has_result = entries.iter().any(|(k, _)| k == "result");
                if !has_result {
                    return Err(ValidationError::InvalidFinish);
                }
            }
        }
        "retry" | "try_again" => {
            // retry body must be non-empty Mapping
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidRetry);
            }
        }
        "on_error" => {
            // on_error body must have body steps
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "on_error" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidOnError);
                }
            }
        }
        _ => {}
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
#[derive(Clone, PartialEq, Debug)]
pub struct WorkflowDoc {
    fields: Vec<(String, FieldValue)>,
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

    /// Returns the primitive field name and its value, if one is declared.
    ///
    /// A step primitive is one of the `STEP_PRIMITIVES` list (set, do, choose,
    /// for_each, together, collect, reduce, repeat, wait, ask, finish).
    /// Returns `None` if no primitive is present (handled by
    /// `validate_single_primitive` separately).
    fn primitive_value(&self) -> Option<(&str, &FieldValue)> {
        for (field, value) in &self.fields {
            if STEP_PRIMITIVES.contains(&field.as_str()) {
                return Some((field, value));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "schema/tests.rs"]
mod tests;
