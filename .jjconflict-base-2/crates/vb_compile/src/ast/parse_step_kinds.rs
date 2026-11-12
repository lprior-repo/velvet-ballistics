#![forbid(unsafe_code)]
use super::parse_step_fields::{
    parse_action_idx, parse_slot_idx, parse_step_idx, parse_step_idx_sequence, parse_u16_field,
    parse_u32_field, parse_value_fields, step_field,
};
use super::parse_values::{parse_expression, parse_slot_expr, parse_value};
use super::types::{AstExpression, AstValue, StepKindAst, StepPrimitiveAst};
use crate::CompileError;
use saphyr::Yaml;
use vb_core::SlotIdx;

pub(super) fn parse_step_kind(
    mapping: &saphyr::Mapping<'_>,
    index: usize,
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
        "together" | "parallel" => {
            parse_together(body, index).map(|kind| (StepPrimitiveAst::Together, kind))
        }
        "collect" => parse_collect(body, index).map(|kind| (StepPrimitiveAst::Collect, kind)),
        "reduce" => parse_reduce(body, index).map(|kind| (StepPrimitiveAst::Reduce, kind)),
        "repeat" => parse_repeat(body, index).map(|kind| (StepPrimitiveAst::Repeat, kind)),
        "wait" => parse_wait(body, index).map(|kind| (StepPrimitiveAst::Wait, kind)),
        "ask" => parse_ask(body, index).map(|kind| (StepPrimitiveAst::Ask, kind)),
        "finish" => parse_finish(body, index).map(|kind| (StepPrimitiveAst::Finish, kind)),
        _ => Err(CompileError::UnknownStepField {
            step: index,
            field: field.into(),
        }),
    }
}

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

fn is_supported_primitive(field: &str) -> bool {
    matches!(
        field,
        "set"
            | "run"
            | "do"
            | "save"
            | "choose"
            | "for_each"
            | "together"
            | "parallel"
            | "collect"
            | "reduce"
            | "repeat"
            | "wait"
            | "ask"
            | "finish"
    )
}

fn parse_run(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Run {
        action: parse_action_idx(step_field(body, index, "action")?, index)?,
        input: parse_slot_idx(step_field(body, index, "input")?, index, "input")?,
    })
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

fn parse_for_each(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::ForEach {
        input: parse_slot_idx(step_field(body, index, "input")?, index, "input")?,
        item: parse_slot_idx(step_field(body, index, "item")?, index, "item")?,
        limit: parse_u32_field(body, index, "limit")?,
    })
}

fn parse_together(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Together {
        branches: parse_step_idx_sequence(step_field(body, index, "branches")?, index, "branches")?,
    })
}

fn parse_collect(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Collect {
        source: parse_slot_idx(step_field(body, index, "source")?, index, "source")?,
        limit: parse_u32_field(body, index, "limit")?,
        page_size: parse_u32_field(body, index, "page_size")?,
    })
}

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

fn parse_repeat(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Repeat {
        max_attempts: parse_u16_field(body, index, "max_attempts")?,
    })
}

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

fn parse_ask(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    Ok(StepKindAst::Ask {
        prompt: parse_slot_idx(step_field(body, index, "prompt")?, index, "prompt")?,
        answer: parse_slot_idx(step_field(body, index, "answer")?, index, "answer")?,
        timeout: optional_slot(body, index, "timeout")?,
    })
}

fn parse_finish(body: &Yaml<'_>, index: usize) -> Result<StepKindAst, CompileError> {
    let result = match body.as_mapping_get("result") {
        Some(node) => node,
        None => body,
    };
    Ok(StepKindAst::Finish {
        result: parse_finish_expression(result, index)?,
    })
}

fn parse_finish_expression(node: &Yaml<'_>, index: usize) -> Result<AstExpression, CompileError> {
    if let Some(value) = node.as_integer() {
        if finish_integer_is_slot(value, index) {
            return parse_slot_expr(value);
        }
        return Ok(AstExpression::Literal(AstValue::I64(value)));
    }
    parse_expression(node)
}

fn optional_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(node) => parse_slot_idx(node, step, field).map(Some),
        None => Ok(None),
    }
}

fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}
