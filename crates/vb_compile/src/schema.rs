#![forbid(unsafe_code)]
use crate::{CompileError, CompileErrors, non_string_key_error, validate_public_name};
use saphyr::Yaml;

pub(crate) fn validate_input_schemas(doc: &Yaml<'_>) -> Result<(), CompileErrors> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let Some(mapping) = node.as_mapping() else {
        return Err(CompileErrors(vec![CompileError::FieldShape {
            field: "inputs",
            expected: "a mapping",
        }]));
    };
    let mut errors = Vec::new();
    for (_, schema) in mapping {
        errors.append(&mut validate_input_schema(schema, SchemaScope::Input));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaScope {
    Input,
    ObjectField,
}

impl SchemaScope {
    const fn allows_from(self) -> bool {
        matches!(self, Self::Input)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaKind {
    Text,
    Number,
    Boolean,
    Object,
    List,
    Any,
}

impl SchemaKind {
    fn from_long_form(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            "list" => Some(Self::List),
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    fn from_list_element(value: &str) -> Option<Self> {
        match value {
            "any" => Some(Self::Any),
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "object" => Some(Self::Object),
            _ => None,
        }
    }
}

fn validate_input_schema(schema: &Yaml<'_>, scope: SchemaScope) -> Vec<CompileError> {
    if let Some(value) = schema.as_str() {
        validate_schema_shorthand(value)
    } else if let Some(mapping) = schema.as_mapping() {
        validate_schema_mapping(mapping, scope)
    } else {
        vec![CompileError::FieldShape {
            field: "inputs",
            expected: "a mapping of input names to schema strings or schema mappings",
        }]
    }
}

fn validate_schema_shorthand(value: &str) -> Vec<CompileError> {
    if is_schema_shorthand(value) {
        Vec::new()
    } else {
        vec![CompileError::InvalidInputSchema {
            field: "inputs",
            expected: "an allowed schema shorthand",
        }]
    }
}

fn is_schema_shorthand(value: &str) -> bool {
    matches!(
        value,
        "text"
            | "number"
            | "boolean"
            | "object"
            | "any"
            | "list<any>"
            | "list<text>"
            | "list<number>"
            | "list<boolean>"
    )
}

fn validate_schema_mapping(mapping: &saphyr::Mapping<'_>, scope: SchemaScope) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut reject_unknown_schema_fields(mapping, scope));
    errors.append(&mut reject_schema_pattern(mapping));
    errors.append(&mut validate_schema_from(mapping, scope));
    let kind = match schema_kind(mapping) {
        Ok(k) => k,
        Err(e) => {
            errors.push(e);
            return errors;
        }
    };
    errors.append(&mut validate_schema_children(mapping, kind));
    errors.append(&mut validate_schema_flags(mapping));
    errors.append(&mut validate_schema_default(mapping, kind));
    errors.append(&mut validate_schema_bounds(mapping, kind));
    errors
}

fn reject_unknown_schema_fields(
    mapping: &saphyr::Mapping<'_>,
    scope: SchemaScope,
) -> Vec<CompileError> {
    let mut errors = Vec::new();
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            errors.push(non_string_key_error());
            continue;
        };
        errors.append(&mut reject_unknown_schema_field(field, scope));
    }
    errors
}

fn reject_unknown_schema_field(field: &str, scope: SchemaScope) -> Vec<CompileError> {
    if is_allowed_schema_field(field, scope) {
        Vec::new()
    } else {
        vec![CompileError::UnknownInputSchemaField {
            field: Box::<str>::from(field),
        }]
    }
}

fn is_allowed_schema_field(field: &str, scope: SchemaScope) -> bool {
    const FIELDS: &[&str] = &[
        "is",
        "of",
        "fields",
        "extra",
        "optional",
        "nullable",
        "default",
        "min",
        "max",
        "min_length",
        "max_length",
        "pattern",
        "secret",
    ];
    FIELDS.contains(&field) || (field == "from" && scope.allows_from())
}

fn reject_schema_pattern(mapping: &saphyr::Mapping<'_>) -> Vec<CompileError> {
    if mapping_get(mapping, "pattern").is_some() {
        vec![CompileError::InvalidInputSchema {
            field: "inputs.pattern",
            expected: "unsupported until a bounded regex engine exists",
        }]
    } else {
        Vec::new()
    }
}

fn validate_schema_from(mapping: &saphyr::Mapping<'_>, scope: SchemaScope) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "from") else {
        return Vec::new();
    };
    if !scope.allows_from() {
        return vec![invalid_schema(
            "inputs.from",
            "top-level input schemas only",
        )];
    }
    match value.as_str() {
        Some(text) if !text.is_empty() => Vec::new(),
        _ => vec![invalid_schema("inputs.from", "a non-empty string")],
    }
}

fn schema_kind(mapping: &saphyr::Mapping<'_>) -> Result<SchemaKind, CompileError> {
    let Some(value) = mapping_get(mapping, "is") else {
        return Err(invalid_schema(
            "inputs.is",
            "one of text, number, boolean, object, list, any",
        ));
    };
    match value.as_str().and_then(SchemaKind::from_long_form) {
        Some(kind) => Ok(kind),
        None => Err(invalid_schema(
            "inputs.is",
            "one of text, number, boolean, object, list, any",
        )),
    }
}

fn validate_schema_children(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut validate_schema_of(mapping, kind));
    errors.append(&mut validate_schema_fields(mapping, kind));
    errors.append(&mut validate_schema_extra(mapping, kind));
    errors
}

fn validate_schema_of(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "of") else {
        return require_list_element_schema(kind);
    };
    if kind != SchemaKind::List {
        return vec![invalid_schema("inputs.of", "present only when is is list")];
    }
    match value.as_str().and_then(SchemaKind::from_list_element) {
        Some(_) => Vec::new(),
        None => vec![invalid_schema(
            "inputs.of",
            "one of any, text, number, boolean, object",
        )],
    }
}

fn require_list_element_schema(kind: SchemaKind) -> Vec<CompileError> {
    if kind == SchemaKind::List {
        vec![invalid_schema("inputs.of", "required when is is list")]
    } else {
        Vec::new()
    }
}

fn validate_schema_fields(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "fields") else {
        return Vec::new();
    };
    if kind != SchemaKind::Object {
        return vec![invalid_schema(
            "inputs.fields",
            "present only when is is object",
        )];
    }
    validate_object_schema_fields(value)
}

fn validate_object_schema_fields(value: &Yaml<'_>) -> Vec<CompileError> {
    let mut errors = Vec::new();
    let Some(fields) = value.as_mapping() else {
        return vec![invalid_schema(
            "inputs.fields",
            "a mapping of field names to schemas",
        )];
    };
    for (key, field_schema) in fields {
        let Some(field) = key.as_str() else {
            errors.push(non_string_key_error());
            continue;
        };
        if let Err(e) = validate_public_name("inputs.fields", field) {
            errors.push(e);
        }
        errors.append(&mut validate_input_schema(
            field_schema,
            SchemaScope::ObjectField,
        ));
    }
    errors
}

fn validate_schema_extra(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "extra") else {
        return Vec::new();
    };
    if kind != SchemaKind::Object {
        return vec![invalid_schema(
            "inputs.extra",
            "present only when is is object",
        )];
    }
    match value.as_str() {
        Some("allow" | "reject") => Vec::new(),
        _ => vec![invalid_schema("inputs.extra", "allow or reject")],
    }
}

fn validate_schema_flags(mapping: &saphyr::Mapping<'_>) -> Vec<CompileError> {
    let mut errors = Vec::new();
    for field in ["optional", "nullable", "secret"] {
        errors.append(&mut validate_schema_bool_field(mapping, field));
    }
    errors
}

fn validate_schema_bool_field(
    mapping: &saphyr::Mapping<'_>,
    field: &'static str,
) -> Vec<CompileError> {
    match mapping_get(mapping, field) {
        Some(value) if yaml_bool(value).is_none() => {
            vec![invalid_schema("inputs boolean flag", "a boolean")]
        }
        _ => Vec::new(),
    }
}

fn validate_schema_default(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let Some(value) = mapping_get(mapping, "default") else {
        return Vec::new();
    };
    if matches!(value, Yaml::Value(saphyr::Scalar::Null)) {
        let nullable = match schema_bool(mapping, "nullable") {
            Ok(b) => b,
            Err(e) => return vec![e],
        };
        return validate_null_default(kind, nullable);
    }
    if default_matches_kind(value, kind) {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.default",
            "a value matching the declared schema type",
        )]
    }
}

fn validate_null_default(kind: SchemaKind, nullable: bool) -> Vec<CompileError> {
    if nullable || kind == SchemaKind::Any {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.default",
            "null only when nullable is true or is is any",
        )]
    }
}

fn default_matches_kind(value: &Yaml<'_>, kind: SchemaKind) -> bool {
    match kind {
        SchemaKind::Text => value.as_str().is_some(),
        SchemaKind::Number => value.as_integer().is_some(),
        SchemaKind::Boolean => yaml_bool(value).is_some(),
        SchemaKind::Object => value.is_mapping(),
        SchemaKind::List => value.as_sequence().is_some(),
        SchemaKind::Any => true,
    }
}

fn validate_schema_bounds(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let mut errors = Vec::new();
    errors.append(&mut validate_min_max_bounds(mapping, kind));
    errors.append(&mut validate_text_length_bounds(mapping, kind));
    errors
}

fn validate_min_max_bounds(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let min = match optional_integer_schema_field(mapping, "min") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let max = match optional_integer_schema_field(mapping, "max") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    if min.is_none() && max.is_none() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    errors.append(&mut validate_min_max_kind(kind));
    errors.append(&mut validate_list_bounds(kind, min, max));
    errors.append(&mut validate_ordered_bounds(min, max, "inputs.min/max"));
    errors
}

fn validate_min_max_kind(kind: SchemaKind) -> Vec<CompileError> {
    if matches!(kind, SchemaKind::Number | SchemaKind::List) {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.min/max",
            "present only for number or list schemas",
        )]
    }
}

fn validate_list_bounds(kind: SchemaKind, min: Option<i64>, max: Option<i64>) -> Vec<CompileError> {
    if kind == SchemaKind::List && [min, max].into_iter().flatten().any(|value| value < 0) {
        vec![invalid_schema(
            "inputs.min/max",
            "non-negative list length bounds",
        )]
    } else {
        Vec::new()
    }
}

fn validate_text_length_bounds(
    mapping: &saphyr::Mapping<'_>,
    kind: SchemaKind,
) -> Vec<CompileError> {
    let min = match optional_integer_schema_field(mapping, "min_length") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let max = match optional_integer_schema_field(mapping, "max_length") {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    if min.is_none() && max.is_none() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    errors.append(&mut validate_text_bounds_kind(kind));
    errors.append(&mut validate_text_bounds_values(min, max));
    errors.append(&mut validate_ordered_bounds(
        min,
        max,
        "inputs.min_length/max_length",
    ));
    errors
}

fn validate_text_bounds_kind(kind: SchemaKind) -> Vec<CompileError> {
    if kind == SchemaKind::Text {
        Vec::new()
    } else {
        vec![invalid_schema(
            "inputs.min_length/max_length",
            "present only for text schemas",
        )]
    }
}

fn validate_text_bounds_values(min: Option<i64>, max: Option<i64>) -> Vec<CompileError> {
    if [min, max].into_iter().flatten().any(|value| value < 0) {
        vec![invalid_schema(
            "inputs.min_length/max_length",
            "non-negative text length bounds",
        )]
    } else {
        Vec::new()
    }
}

fn validate_ordered_bounds(
    min: Option<i64>,
    max: Option<i64>,
    field: &'static str,
) -> Vec<CompileError> {
    match (min, max) {
        (Some(min_val), Some(max_val)) if min_val > max_val => {
            vec![invalid_schema(field, "min less than or equal to max")]
        }
        _ => Vec::new(),
    }
}

fn optional_integer_schema_field(
    mapping: &saphyr::Mapping<'_>,
    field: &'static str,
) -> Result<Option<i64>, CompileError> {
    match mapping_get(mapping, field) {
        Some(value) => value
            .as_integer()
            .map(Some)
            .ok_or(invalid_schema(field, "an integer")),
        None => Ok(None),
    }
}

fn schema_bool(mapping: &saphyr::Mapping<'_>, field: &str) -> Result<bool, CompileError> {
    match mapping_get(mapping, field) {
        Some(value) => yaml_bool(value).ok_or(CompileError::InvalidInputSchema {
            field: "inputs boolean flag",
            expected: "a boolean",
        }),
        None => Ok(false),
    }
}

fn yaml_bool(node: &Yaml<'_>) -> Option<bool> {
    match node {
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn mapping_get<'a>(mapping: &'a saphyr::Mapping<'a>, field: &str) -> Option<&'a Yaml<'a>> {
    mapping.iter().find_map(|(key, value)| match key.as_str() {
        Some(name) if name == field => Some(value),
        _ => None,
    })
}

fn invalid_schema(field: &'static str, expected: &'static str) -> CompileError {
    CompileError::InvalidInputSchema { field, expected }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::YamlCompiler;
    use saphyr::LoadableYamlNode;

    fn validate_inputs(inputs: &str) -> Result<(), CompileError> {
        let source = format!("version: velvet-ballastics/v1\ninputs:\n{inputs}\n");
        let docs = Yaml::load_from_str(&source)?;
        let Some(doc) = docs.first() else {
            return Err(CompileError::EmptySource);
        };
        match validate_input_schemas(doc) {
            Ok(()) => Ok(()),
            Err(errors) => match errors.first() {
                Some(error) => Err(error.clone()),
                None => Err(CompileError::EmptySource),
            },
        }
    }

    #[test]
    fn input_schema_rejects_unknown_fields() {
        let result = validate_inputs("  value:\n    is: text\n    kind: text\n");

        assert!(matches!(
            result,
            Err(CompileError::UnknownInputSchemaField { .. })
        ));
    }

    #[test]
    fn input_schema_rejects_invalid_bounds() {
        let result =
            validate_inputs("  value:\n    is: text\n    min_length: 9\n    max_length: 1\n");

        assert!(matches!(
            result,
            Err(CompileError::InvalidInputSchema { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // vb-yd5x RED PHASE: Shared IR parity tests
    // ---------------------------------------------------------------------------

    /// Minimal valid workflow for testing
    const VB_YD5X_MINIMAL_VALID_WORKFLOW: &[u8] = br#"
version: velvet-ballastics/v1
name: minimal_valid
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    /// Workflow with out-of-range slot reference (Gate 9)
    /// This uses a slot index that is out of bounds for the compiled workflow.
    /// The issue is the result slot 99 doesn't exist.
    const VB_YD5X_MALFORMED_SLOT_REF: &[u8] = br#"
version: velvet-ballastics/v1
name: bad_slot_ref
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: use_missing_slot
    for_each:
      input: 99
      item: 1
      limit: 10
  - id: done
    finish:
      result: 0
"#;

    /// Workflow with loop body step out of range (Gate 11)
    /// The together branches point to step 2 (join) but join is at node 1, not step 2.
    const VB_YD5X_MALFORMED_LOOP_BODY: &[u8] = br#"
version: velvet-ballastics/v1
name: bad_loop_body
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches: [2]
  - id: join
    finish:
      result: 0
"#;

    /// Workflow with duplicate step ID
    const VB_YD5X_MALFORMED_DUPLICATE_ID: &[u8] = br#"
version: velvet-ballastics/v1
name: duplicate_ids
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: build
    finish:
      result: 0
"#;

    /// Workflow with unknown reference
    const VB_YD5X_MALFORMED_UNKNOWN_REF: &[u8] = br#"
version: velvet-ballastics/v1
name: unknown_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $input.missing == true
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;

    /// Helper: validate via compile then shared pipeline
    fn vb_yd5x_validate_via_compile(source: &[u8]) -> Result<(), CompileErrors> {
        let compiled = YamlCompiler::default().compile(source)?;
        let parts = compiled.to_parts();
        vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))
    }

    #[test]
    fn vb_yd5x_valid_workflow_passes_both_paths() {
        let source = VB_YD5X_MINIMAL_VALID_WORKFLOW;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_ok(),
            "valid workflow must compile: {compile_result:?}"
        );
        assert!(
            validate_result.is_ok(),
            "valid workflow must pass shared validation: {validate_result:?}"
        );
    }

    #[test]
    fn vb_yd5x_malformed_slot_ref_fails_consistently() {
        let source = VB_YD5X_MALFORMED_SLOT_REF;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        // Both must fail
        assert!(
            compile_result.is_err(),
            "compile should fail for bad slot ref"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for bad slot ref"
        );
        // Both should produce the same error code
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("TYPE_MISMATCH"),
            "expected TYPE_MISMATCH"
        );
    }

    #[test]
    fn vb_yd5x_malformed_loop_body_fails_consistently() {
        let source = VB_YD5X_MALFORMED_LOOP_BODY;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for bad loop body"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for bad loop body"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("INVALID_THEN_TARGET"),
            "expected INVALID_THEN_TARGET"
        );
    }

    #[test]
    fn vb_yd5x_malformed_duplicate_id_fails_consistently() {
        let source = VB_YD5X_MALFORMED_DUPLICATE_ID;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for duplicate id"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for duplicate id"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(compile_code, Some("DUPLICATE_ID"), "expected DUPLICATE_ID");
    }

    #[test]
    fn vb_yd5x_malformed_unknown_ref_fails_consistently() {
        let source = VB_YD5X_MALFORMED_UNKNOWN_REF;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for unknown ref"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for unknown ref"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("UNKNOWN_REFERENCE"),
            "expected UNKNOWN_REFERENCE"
        );
    }

    #[test]
    fn vb_yd5x_diagnostic_codes_remain_stable() {
        // Test that error codes are stable across paths
        let test_cases = [
            (VB_YD5X_MALFORMED_SLOT_REF, "TYPE_MISMATCH"),
            (VB_YD5X_MALFORMED_LOOP_BODY, "INVALID_THEN_TARGET"),
            (VB_YD5X_MALFORMED_DUPLICATE_ID, "DUPLICATE_ID"),
            (VB_YD5X_MALFORMED_UNKNOWN_REF, "UNKNOWN_REFERENCE"),
        ];
        for (source, expected_code) in test_cases {
            let compile_result = YamlCompiler::default().compile(source);
            let validate_result = vb_yd5x_validate_via_compile(source);
            let compile_code = compile_result
                .as_ref()
                .err()
                .and_then(|e| e.first())
                .map(|e| e.code());
            let validate_code = validate_result
                .as_ref()
                .err()
                .and_then(|e| e.first())
                .map(|e| e.code());
            assert_eq!(
                compile_code, validate_code,
                "codes should match for {expected_code}"
            );
            assert_eq!(
                compile_code,
                Some(expected_code),
                "expected {expected_code}"
            );
        }
    }
}
