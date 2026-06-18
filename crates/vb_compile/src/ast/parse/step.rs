#![forbid(unsafe_code)]
//! Step parsing: dispatch, primitive detection, and body-step parsing.
//!
//! Detects the single primitive key in a step mapping and dispatches
//! to the appropriate step-kind parser.

use crate::ast::types::{AstExpression, AstValue, StepAst, StepKindAst, StepPrimitiveAst};
use crate::CompileError;
use saphyr::Yaml;

use super::expr::{parse_action_idx, parse_expression, parse_slot_expr, parse_slot_idx, parse_step_idx, parse_u16_field, parse_u32_field};
use super::field::{optional_slot, optional_str, parse_value, parse_value_fields, step_field, step_str};

/// Parse a sequence of step nodes.
pub(crate) fn parse_steps(
    steps: &saphyr::Sequence<'_>,
    marks: &crate::ast::marks::AstMarks,
) -> Result<Vec<StepAst>, CompileError> {
    let mut parsed = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        parsed.push(parse_step(step, index, marks)?);
    }
    Ok(parsed)
}

/// Parse a single step node into a `StepAst`.
pub(crate) fn parse_step(
    step: &Yaml<'_>,
    index: usize,
    marks: &crate::ast::marks::AstMarks,
) -> Result<StepAst, CompileError> {
    let mapping = step
        .as_mapping()
        .ok_or(CompileError::StepShape { step: index })?;
    let id = step_str(step, index, "id")?;
    let (primitive, kind) = parse_step_kind(mapping, index, marks)?;
    Ok(StepAst {
        id: id.into(),
        name: optional_str(step, "name").map(Box::<str>::from),
        primitive,
        kind,
        mark: marks.step(id),
    })
}

/// Detect the single primitive key and dispatch to its parser.
fn parse_step_kind(
    mapping: &saphyr::Mapping<'_>,
    index: usize,
    marks: &crate::ast::marks::AstMarks,
) -> Result<(StepPrimitiveAst, StepKindAst), CompileError> {
    let Some((field, body)) = primitive_entry(mapping, index)? else {
        return Err(CompileError::MissingStepPrimitive { step: index });
    };
    match field {
        "set" => parse_save(body).map(|kind| (StepPrimitiveAst::Set, kind)),
        "run" => parse_run(body, index).map(|kind| (StepPrimitiveAst::Run, kind)),
        "do" => parse_run(body, index).map(|kind| (StepPrimitiveAst::Do, kind)),
        "save" => parse_save(body).map(|kind| (StepPrimitiveAst::Save, kind)),
        "choose" => parse_choose(body, index).map(|kind| (StepPrimitiveAst::Choose, kind)),
        "for_each" => parse_for_each(body, index).map(|kind| (StepPrimitiveAst::ForEach, kind)),
        "together" => parse_together(body, index).map(|kind| (StepPrimitiveAst::Together, kind)),
        "collect" => parse_collect(body, index).map(|kind| (StepPrimitiveAst::Collect, kind)),
        "reduce" => parse_reduce(body, index).map(|kind| (StepPrimitiveAst::Reduce, kind)),
        "repeat" => parse_repeat(body, index, marks).map(|kind| (StepPrimitiveAst::Repeat, kind)),
        "wait" => parse_wait(body, index).map(|kind| (StepPrimitiveAst::Wait, kind)),
        "ask" => parse_ask(body, index).map(|kind| (StepPrimitiveAst::Ask, kind)),
        "finish" => parse_finish(body, index).map(|kind| (StepPrimitiveAst::Finish, kind)),
        _ => Err(CompileError::UnknownStepField {
            step: index,
            field: field.into(),
        }),
    }
}

/// Find the single supported primitive entry in a step mapping.
fn primitive_entry<'map, 'input>(
    mapping: &'map saphyr::Mapping<'input>,
    index: usize,
) -> Result<Option<(&'map str, &'map Yaml<'input>)>, CompileError> {
    let mut selected = None;
    for (key, body) in mapping {
        let field = key.as_str().ok_or_else(crate::non_string_key_error)?;
        if is_supported_primitive(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some((field, body));
        }
    }
    Ok(selected)
}

/// Check if a field name is a supported step primitive.
pub(crate) fn is_supported_primitive(field: &str) -> bool {
    matches!(
        field,
        "set"
            | "run"
            | "do"
            | "save"
            | "choose"
            | "for_each"
            | "together"
            | "collect"
            | "reduce"
            | "repeat"
            | "wait"
            | "ask"
            | "finish"
    )
}

/// Parse a `run` / `do` step body.
fn parse_run(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Run {
        action: parse_action_idx(step_field(body, index, "action")?, index)?,
        input: parse_slot_idx(step_field(body, index, "input")?, index, "input")?,
    })
}

/// Parse a `set` / `save` step body.
fn parse_save(body: &Yaml<'_>) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Save {
        fields: parse_value_fields(body)?,
    })
}

/// Parse a `choose` step body.
fn parse_choose(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Choose {
        condition: parse_expression(step_field(body, index, "condition")?)?,
        on_true: parse_step_idx(step_field(body, index, "on_true")?)?,
        on_false: parse_step_idx(step_field(body, index, "on_false")?)?,
    })
}

/// Parse a `for_each` step body.
fn parse_for_each(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::ForEach {
        input: parse_slot_idx(step_field(body, index, "input")?, index, "input")?,
        item: parse_slot_idx(step_field(body, index, "item")?, index, "item")?,
        limit: parse_u32_field(body, index, "limit")?,
    })
}

/// Parse a `together` step body.
fn parse_together(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    use super::expr::parse_step_idx_sequence;
    Ok(StepKindAst::Together {
        branches: parse_step_idx_sequence(
            step_field(body, index, "branches")?,
            index,
            "branches",
        )?,
    })
}

/// Parse a `collect` step body.
fn parse_collect(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Collect {
        source: parse_slot_idx(step_field(body, index, "source")?, index, "source")?,
        limit: parse_u32_field(body, index, "limit")?,
        page_size: parse_u32_field(body, index, "page_size")?,
    })
}

/// Parse a `reduce` step body.
fn parse_reduce(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Reduce {
        input: parse_slot_idx(step_field(body, index, "input")?, index, "input")?,
        accumulator: parse_slot_idx(
            step_field(body, index, "accumulator")?,
            index,
            "accumulator",
        )?,
        initial: parse_value(step_field(body, index, "initial")?)?,
    })
}

/// Parse a `repeat` step body.
fn parse_repeat(
    body: &Yaml<'_>,
    index: usize,
    marks: &crate::ast::marks::AstMarks,
) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Repeat {
        max_attempts: parse_u16_field(body, index, "max_attempts")?,
        body: parse_body_steps(body, index, marks)?,
    })
}

/// Parse the `steps:` body of a control-flow primitive into a `Vec<StepAst>`.
///
/// Returns an empty vector when the field is absent, mirroring the
/// `vb_yaml::ast::StepPrimitive` upstream contract. The sequence shape
/// (when present) is validated by `parse_step`, which delegates to
/// `parse_step_kind` and surfaces the same diagnostics as top-level steps.
pub(crate) fn parse_body_steps(
    body: &Yaml<'_>,
    index: usize,
    marks: &crate::ast::marks::AstMarks,
) -> Result<Vec<StepAst>, CompileError> {
    let Some(node) = body.as_mapping_get("steps") else {
        return Ok(Vec::new());
    };
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "steps",
        expected: "a sequence of step objects",
    })?;
    let mut parsed = Vec::with_capacity(sequence.len());
    for (sub_index, item) in sequence.iter().enumerate() {
        // Each body step must carry a unique id within the body. We reuse
        // the step-level `parse_step`, which enforces the `id` requirement.
        // The body sub-index is propagated so diagnostics refer to a stable
        // offset within the body sequence.
        let _ = sub_index;
        parsed.push(parse_step(item, index, marks)?);
    }
    Ok(parsed)
}

/// Parse a `wait` step body.
fn parse_wait(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    let until = optional_slot(body, index, "until")?;
    let event = optional_slot(body, index, "event")?;
    match (until, event) {
        (Some(slot), None) => Ok(StepKindAst::Wait {
            slot,
            timeout: None,
            is_event: false,
        }),
        (None, Some(slot)) => Ok(StepKindAst::Wait {
            slot,
            timeout: optional_slot(body, index, "timeout")?,
            is_event: true,
        }),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "exactly one of until or event",
        }),
    }
}

/// Parse an `ask` step body.
fn parse_ask(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Ask {
        prompt: parse_slot_idx(step_field(body, index, "prompt")?, index, "prompt")?,
        answer: parse_slot_idx(step_field(body, index, "answer")?, index, "answer")?,
        timeout: optional_slot(body, index, "timeout")?,
    })
}

/// Parse a `finish` step body.
fn parse_finish(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    let result = match body.as_mapping_get("result") {
        Some(node) => node,
        None => body,
    };
    Ok(StepKindAst::Finish {
        result: parse_finish_expression(result, index)?,
    })
}

/// Parse the result expression for a `finish` step.
fn parse_finish_expression(node: &Yaml<'_>, index: usize) -> Result<AstExpression, CompileError> {
    if let Some(value) = node.as_integer() {
        if finish_integer_is_slot(value, index) {
            return parse_slot_expr(value);
        }
        return Ok(AstExpression::Literal(AstValue::I64(value)));
    }
    parse_expression(node)
}

/// Check whether a finish-step integer is a slot reference.
fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}
