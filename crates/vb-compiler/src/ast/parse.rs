use super::marks::AstMarks;
use super::types::{
    AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, TriggerAst, WorkflowAst,
};
use crate::{CompileError, SourceMark};
use saphyr::Yaml;
use vb_core::{SlotIdx, StepIdx};

/// Parses a semantically validated YAML tree into the cold typed AST.
pub(crate) fn parse_workflow_ast(text: &str, doc: &Yaml<'_>) -> Result<WorkflowAst, CompileError> {
    let marks = AstMarks::new(text)?;
    Ok(WorkflowAst {
        version: required_str(doc, "version")?.into(),
        name: required_str(doc, "name")?.into(),
        trigger: parse_trigger(required_mapping(doc, "when")?, &marks)?,
        inputs: parse_value_map(doc, "inputs", &marks)?,
        vars: parse_value_map(doc, "vars", &marks)?,
        secrets: parse_secret_map(doc, &marks)?,
        result: parse_expression_map(doc, "result", &marks)?,
        examples: parse_examples(doc)?,
        steps: parse_steps(required_sequence(doc, "steps")?, &marks)?,
        mark: marks.document(),
    })
}

fn required_str<'a>(doc: &'a Yaml<'a>, field: &'static str) -> Result<&'a str, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_str()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a string",
        })
}

fn required_mapping<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_mapping()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a mapping",
        })
}

fn required_sequence<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_sequence()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a sequence",
        })
}

fn parse_trigger(
    mapping: &saphyr::Mapping<'_>,
    marks: &AstMarks,
) -> Result<TriggerAst, CompileError> {
    let Some((key, value)) = mapping.iter().next() else {
        return Err(CompileError::InvalidTriggerCount { count: 0 });
    };
    let kind = key.as_str().ok_or_else(crate::non_string_key_error)?;
    let mark = marks.trigger(kind);
    match kind {
        "manual" => Ok(TriggerAst::Manual { mark }),
        "webhook" => parse_webhook_trigger(value, mark),
        "schedule" => parse_schedule_trigger(value, mark),
        "event" => parse_event_trigger(value, mark),
        other => Err(CompileError::UnknownTriggerKind {
            trigger: other.into(),
        }),
    }
}

fn parse_webhook_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Webhook {
        path: trigger_str(value, "webhook", "path")?.into(),
        method: trigger_str(value, "webhook", "method")?.into(),
        unique: optional_str(value, "unique").map(Box::<str>::from),
        mark,
    })
}

fn parse_schedule_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Schedule {
        cron: trigger_str(value, "schedule", "cron")?.into(),
        timezone: optional_str(value, "timezone").map(Box::<str>::from),
        mark,
    })
}

fn parse_event_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Event {
        name: trigger_str(value, "event", "name")?.into(),
        mark,
    })
}

fn trigger_str<'a>(
    value: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    value
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?
        .as_str()
        .ok_or(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        })
}

fn optional_str<'a>(value: &'a Yaml<'a>, field: &str) -> Option<&'a str> {
    value.as_mapping_get(field).and_then(Yaml::as_str)
}

fn parse_value_map(
    doc: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<AstValue>>, CompileError> {
    let Some(node) = doc.as_mapping_get(field) else {
        return Ok(Vec::new());
    };
    parse_map(node, field, marks, parse_value)
}

fn parse_expression_map(
    doc: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<AstExpression>>, CompileError> {
    let Some(node) = doc.as_mapping_get(field) else {
        return Ok(Vec::new());
    };
    parse_map(node, field, marks, parse_expression)
}

fn parse_map<T, F>(
    node: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
    parse: F,
) -> Result<Vec<AstMapEntry<T>>, CompileError>
where
    F: Fn(&Yaml<'_>) -> Result<T, CompileError>,
{
    node.as_mapping()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a mapping",
        })?
        .iter()
        .map(|(key, value)| parse_entry(key, value, field, marks, &parse))
        .collect()
}

fn parse_entry<T, F>(
    key: &Yaml<'_>,
    value: &Yaml<'_>,
    field: &'static str,
    marks: &AstMarks,
    parse: &F,
) -> Result<AstMapEntry<T>, CompileError>
where
    F: Fn(&Yaml<'_>) -> Result<T, CompileError>,
{
    let name = key.as_str().ok_or_else(crate::non_string_key_error)?;
    Ok(AstMapEntry {
        name: name.into(),
        value: parse(value)?,
        mark: marks.nested_key(field, name),
    })
}

fn parse_secret_map(
    doc: &Yaml<'_>,
    marks: &AstMarks,
) -> Result<Vec<AstMapEntry<Box<str>>>, CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(Vec::new());
    };
    parse_map(node, "secrets", marks, |value| {
        value
            .as_str()
            .map(Box::<str>::from)
            .ok_or(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            })
    })
}

fn parse_examples(doc: &Yaml<'_>) -> Result<Vec<AstValue>, CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(Vec::new());
    };
    node.as_sequence()
        .ok_or(CompileError::FieldShape {
            field: "examples",
            expected: "a sequence",
        })?
        .iter()
        .map(parse_value)
        .collect()
}

fn parse_steps(
    steps: &saphyr::Sequence<'_>,
    marks: &AstMarks,
) -> Result<Vec<StepAst>, CompileError> {
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| parse_step(step, index, marks))
        .collect()
}

fn parse_step(step: &Yaml<'_>, index: usize, marks: &AstMarks) -> Result<StepAst, CompileError> {
    let mapping = step
        .as_mapping()
        .ok_or(CompileError::StepShape { step: index })?;
    let id = step_str(step, index, "id")?;
    Ok(StepAst {
        id: id.into(),
        name: optional_str(step, "name").map(Box::<str>::from),
        kind: parse_step_kind(mapping, index)?,
        mark: marks.step(id),
    })
}

fn parse_step_kind(
    mapping: &saphyr::Mapping<'_>,
    index: usize,
) -> Result<StepKindAst, CompileError> {
    let Some((field, body)) = primitive_entry(mapping)? else {
        return Err(CompileError::MissingStepPrimitive { step: index });
    };
    match field {
        "save" => parse_save(body),
        "choose" => parse_choose(body, index),
        "finish" => parse_finish(body),
        _ => Err(CompileError::UnknownStepField {
            step: index,
            field: field.into(),
        }),
    }
}

fn primitive_entry<'a>(
    mapping: &'a saphyr::Mapping<'a>,
) -> Result<Option<(&'a str, &'a Yaml<'a>)>, CompileError> {
    mapping.iter().try_fold(None, |selected, (key, body)| {
        let field = key.as_str().ok_or_else(crate::non_string_key_error)?;
        if is_supported_primitive(field) {
            selected.map_or(Ok(Some((field, body))), |_| {
                Err(CompileError::MultipleStepPrimitives { step: 0 })
            })
        } else {
            Ok(selected)
        }
    })
}

fn is_supported_primitive(field: &str) -> bool {
    matches!(field, "save" | "choose" | "finish")
}

fn parse_save(body: &Yaml<'_>) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Save {
        fields: parse_value_fields(body)?,
    })
}

fn parse_choose(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Choose {
        condition: parse_expression(step_field(body, index, "condition")?)?,
        on_true: parse_step_idx(step_field(body, index, "on_true")?)?,
        on_false: parse_step_idx(step_field(body, index, "on_false")?)?,
    })
}

fn parse_finish(body: &Yaml<'_>) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Finish {
        result: parse_expression(body.as_mapping_get("result").unwrap_or(body))?,
    })
}

fn parse_value_fields(body: &Yaml<'_>) -> Result<Vec<AstMapEntry<AstValue>>, CompileError> {
    body.as_mapping()
        .ok_or(CompileError::StepFieldShape {
            step: 0,
            field: "save",
            expected: "an object",
        })?
        .iter()
        .map(|(key, value)| value_field(key, value))
        .collect()
}

fn value_field(key: &Yaml<'_>, value: &Yaml<'_>) -> Result<AstMapEntry<AstValue>, CompileError> {
    let name = key.as_str().ok_or_else(crate::non_string_key_error)?;
    Ok(AstMapEntry {
        name: name.into(),
        value: parse_value(value)?,
        mark: None,
    })
}

fn step_str<'a>(
    step: &'a Yaml<'a>,
    index: usize,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    step.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step: index, field })?
        .as_str()
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a string",
        })
}

fn step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

fn parse_step_idx(node: &Yaml<'_>) -> Result<StepIdx, CompileError> {
    let value = node
        .as_integer()
        .ok_or(CompileError::BranchTargetOutOfRange { value: -1 })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(raw))
}

fn parse_expression(node: &Yaml<'_>) -> Result<AstExpression, CompileError> {
    if let Some(value) = node.as_integer() {
        return parse_slot_expr(value);
    }
    Ok(match node.as_str() {
        Some(value) if value.starts_with('$') => AstExpression::Reference(value.into()),
        _ => AstExpression::Literal(parse_value(node)?),
    })
}

fn parse_slot_expr(value: i64) -> Result<AstExpression, CompileError> {
    let raw = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(AstExpression::Slot(SlotIdx::new(raw)))
}

fn parse_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
    if node.is_null() {
        Ok(AstValue::Null)
    } else if let Some(value) = node.as_bool() {
        Ok(AstValue::Bool(value))
    } else if let Some(value) = node.as_integer() {
        Ok(AstValue::I64(value))
    } else {
        parse_non_scalar_value(node)
    }
}

fn parse_non_scalar_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
    if let Some(value) = node.as_str() {
        Ok(text_or_ref(value))
    } else if let Some(sequence) = node.as_sequence() {
        sequence
            .iter()
            .map(parse_value)
            .collect::<Result<_, _>>()
            .map(AstValue::Sequence)
    } else if let Some(mapping) = node.as_mapping() {
        mapping
            .iter()
            .map(|(k, v)| value_field(k, v))
            .collect::<Result<_, _>>()
            .map(AstValue::Mapping)
    } else {
        Err(CompileError::BadValue)
    }
}

fn text_or_ref(value: &str) -> AstValue {
    if value.starts_with('$') {
        AstValue::Reference(value.into())
    } else {
        AstValue::Text(value.into())
    }
}
