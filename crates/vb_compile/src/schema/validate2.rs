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
