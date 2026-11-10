#![forbid(unsafe_code)]
//! Tail-field and suspension primitive parsing for workflow steps.

use super::{lookup, reject_unknown_fields, require_scalar_in, require_str_in, require_u16};
use crate::yaml_ast::types::{ErrorHandlerAst, RetryPolicy, StepAst, StepPrimitive};
use crate::yaml_error::{YamlError, YamlResult};

pub(super) fn parse_retry(node: &saphyr::Yaml<'_>) -> YamlResult<Option<RetryPolicy>> {
    if lookup(node, "retry").is_some() {
        return Err(YamlError::UnknownField {
            field: "retry".into(),
        });
    }
    let Some(sub) = lookup(node, "try_again") else {
        return Ok(None);
    };
    reject_unknown_fields(sub, &["max_attempts", "delay"])?;
    let max_attempts = require_u16(sub, "max_attempts")?;
    let delay = optional_string(sub, "delay", "try_again.delay")?;
    Ok(Some(RetryPolicy {
        max_attempts,
        delay,
    }))
}

pub(super) fn parse_error_handler(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ErrorHandlerAst>> {
    let Some(sub) = lookup(node, "on_error") else {
        return Ok(None);
    };
    reject_unknown_fields(sub, &["handler"])?;
    let handler = require_str_in(sub, "handler", "on_error.handler")?;
    Ok(Some(ErrorHandlerAst { handler }))
}

pub(super) fn parse_wait(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["event", "timeout"])?;
    let event = optional_string(sub, "event", "wait.event")?;
    if event.as_ref().is_some_and(String::is_empty) {
        return Err(YamlError::FieldShape {
            field: "wait.event",
            expected: "non-empty string",
        });
    }
    let timeout = optional_string(sub, "timeout", "wait.timeout")?;
    Ok(StepPrimitive::Wait { event, timeout })
}

pub(super) fn parse_ask(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["prompt", "timeout"])?;
    let prompt = require_str_in(sub, "prompt", "ask.prompt")?;
    let timeout = optional_string(sub, "timeout", "ask.timeout")?;
    Ok(StepPrimitive::Ask { prompt, timeout })
}

pub(super) fn parse_finish(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["result"])?;
    let result = require_scalar_in(sub, "result", "finish.result")?;
    Ok(StepPrimitive::Finish { result })
}

fn optional_string(
    node: &saphyr::Yaml<'_>,
    field: &'static str,
    label: &'static str,
) -> YamlResult<Option<String>> {
    match lookup(node, field) {
        Some(value) => Ok(Some(
            value
                .as_str()
                .ok_or(YamlError::FieldShape {
                    field: label,
                    expected: "string",
                })?
                .to_string(),
        )),
        None => Ok(None),
    }
}

pub(super) fn parse_body_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = super::lookup(node, "steps") else {
        return Ok(Vec::new());
    };
    let mut steps = Vec::new();
    for item in super::sequence(seq, "steps")? {
        steps.push(super::parse_step(item)?);
    }
    Ok(steps)
}
