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

mod validate2;
