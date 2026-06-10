#![forbid(unsafe_code)]
//! Step parsing logic.

use super::parse::{
    lookup, mapping, opt_str, opt_u32, reject_unknown_fields, require_scalar_in, require_str_in,
    require_u16, sequence,
};
use super::types::*;
use crate::{YamlError, YamlResult};

pub(super) fn parse_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(node) = lookup(node, "steps") else {
        return Err(YamlError::MissingField { field: "steps" });
    };
    let mut steps = Vec::new();
    for item in sequence(node, "steps")? {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_step(yaml: &saphyr::Yaml<'_>) -> YamlResult<StepAst> {
    reject_unknown_step_fields(yaml)?;
    let id = require_str_in(yaml, "id", "step.id")?;
    let name = opt_str(yaml, "name");
    let condition = opt_str(yaml, "if");
    let primitive = parse_step_primitive(yaml)?;
    let with = opt_str(yaml, "with");
    let retry = parse_retry(yaml)?;
    let on_error = parse_error_handler(yaml)?;
    let then = opt_str(yaml, "then");
    Ok(StepAst {
        id,
        name,
        condition,
        primitive,
        with,
        retry,
        on_error,
        then,
    })
}

fn parse_step_primitive(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut selected: Option<(&str, &saphyr::Yaml<'_>)> = None;
    for (key, value) in mapping(node, "step")? {
        let Some(key) = key.as_str() else {
            return Err(YamlError::FieldShape {
                field: "step key",
                expected: "string",
            });
        };
        // Intercept legacy names BEFORE is_primitive() gate to emit correct error
        if key == "parallel" {
            return Err(YamlError::LegacyPrimitive {
                primitive: "parallel",
                canonical: "together",
            });
        }
        if key == "aggregate" {
            return Err(YamlError::LegacyPrimitive {
                primitive: "aggregate",
                canonical: "reduce",
            });
        }
        if is_primitive(key) {
            if selected.is_some() {
                return Err(YamlError::FieldShape {
                    field: "step",
                    expected: "exactly one primitive",
                });
            }
            selected = Some((key, value));
        }
    }
    let Some((kind, sub)) = selected else {
        return Err(YamlError::MissingField {
            field: "step primitive",
        });
    };
    match kind {
        "set" => parse_set(sub, "set"),
        "save" => parse_set(sub, "save"),
        "do" | "run" => parse_do(sub, kind),
        "choose" => parse_choose(sub),
        "foreach" | "for_each" => parse_foreach(sub),
        "together" => parse_together(sub),
        "collect" => parse_collect(sub),
        "reduce" => parse_reduce(sub),
        "repeat" => parse_repeat(sub),
        "wait" => parse_wait(sub),
        "ask" => parse_ask(sub),
        "finish" => parse_finish(sub),
        _ => Err(YamlError::UnknownField { field: kind.into() }),
    }
}

/// Returns `true` if the given field name is a recognised step primitive.
///
/// `pub(crate)` so the `kani_is_primitive_legacy` harness under
/// `src/kani_is_primitive_legacy.rs` can verify the legacy-name
/// rejection behavior. Not part of the public API.
pub(crate) fn is_primitive(field: &str) -> bool {
    matches!(
        field,
        "set"
            | "save"
            | "do"
            | "run"
            | "choose"
            | "foreach"
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

fn reject_unknown_step_fields(node: &saphyr::Yaml<'_>) -> YamlResult<()> {
    reject_unknown_fields(
        node,
        &[
            "id",
            "name",
            "if",
            "set",
            "save",
            "do",
            "run",
            "choose",
            "foreach",
            "for_each",
            "together",
            "collect",
            "reduce",
            "repeat",
            "wait",
            "ask",
            "finish",
            "with",
            "try_again",
            "on_error",
            "then",
        ],
    )
}

fn parse_set(sub: &saphyr::Yaml<'_>, primitive: &'static str) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["output", "value"])?;
    let output = require_str_in(sub, "output", "set.output")?;
    let value = require_str_in(sub, "value", "set.value")?;
    if primitive == "save" && output.is_empty() {
        return Err(YamlError::FieldShape {
            field: "save.output",
            expected: "non-empty string",
        });
    }
    Ok(StepPrimitive::Set { output, value })
}

fn parse_do(sub: &saphyr::Yaml<'_>, primitive: &str) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["action", "input"])?;
    let action = require_str_in(sub, "action", "do.action")?;
    let input = require_str_in(sub, "input", "do.input")?;
    if primitive == "run" && action.is_empty() {
        return Err(YamlError::FieldShape {
            field: "run.action",
            expected: "non-empty string",
        });
    }
    Ok(StepPrimitive::Do { action, input })
}

fn parse_choose(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["branches", "otherwise"])?;
    let mut branches = Vec::new();
    if let Some(seq) = lookup(node, "branches") {
        for item in sequence(seq, "choose.branches")? {
            reject_unknown_fields(item, &["when", "steps"])?;
            let when = require_str_in(item, "when", "choose.branches[].when")?;
            let steps = parse_body_steps(item)?;
            branches.push(ChooseBranch { when, steps });
        }
    }
    let otherwise = opt_str(node, "otherwise");
    Ok(StepPrimitive::Choose {
        branches,
        otherwise,
    })
}

fn parse_foreach(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["variable", "input", "at_once", "steps"])?;
    let variable = require_str_in(node, "variable", "foreach.variable")?;
    let input = require_str_in(node, "input", "foreach.input")?;
    let at_once = opt_u32(node, "at_once");
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body,
    })
}

fn parse_together(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["branches"])?;
    let mut branches = Vec::new();
    if let Some(seq) = lookup(node, "branches") {
        for item in sequence(seq, "together.branches")? {
            reject_unknown_fields(item, &["label", "steps"])?;
            let label = require_str_in(item, "label", "together.branches[].label")?;
            let steps = parse_body_steps(item)?;
            branches.push(TogetherBranch { label, steps });
        }
    }
    Ok(StepPrimitive::Together { branches })
}

fn parse_collect(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["variable", "source", "pages", "items", "steps"])?;
    let variable = require_str_in(node, "variable", "collect.variable")?;
    let source = require_str_in(node, "source", "collect.source")?;
    let pages = opt_u32(node, "pages");
    let items = opt_u32(node, "items");
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Collect {
        variable,
        source,
        pages,
        items,
        body,
    })
}

fn parse_reduce(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["variable", "input", "initial", "steps"])?;
    let variable = require_str_in(node, "variable", "reduce.variable")?;
    let input = require_str_in(node, "input", "reduce.input")?;
    let initial = require_str_in(node, "initial", "reduce.initial")?;
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Reduce {
        variable,
        input,
        initial,
        body,
    })
}

fn parse_repeat(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(node, &["max_attempts", "steps"])?;
    let max_attempts = require_u16(node, "max_attempts")?;
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Repeat { max_attempts, body })
}

fn parse_body_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = lookup(node, "steps") else {
        return Ok(Vec::new());
    };
    let mut steps = Vec::new();
    for item in sequence(seq, "steps")? {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_retry(node: &saphyr::Yaml<'_>) -> YamlResult<Option<RetryPolicy>> {
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
    let delay = match lookup(sub, "delay") {
        Some(v) => Some(
            v.as_str()
                .ok_or(YamlError::FieldShape {
                    field: "try_again.delay",
                    expected: "string",
                })?
                .to_string(),
        ),
        None => None,
    };
    Ok(Some(RetryPolicy {
        max_attempts,
        delay,
    }))
}

fn parse_error_handler(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ErrorHandlerAst>> {
    let Some(sub) = lookup(node, "on_error") else {
        return Ok(None);
    };
    reject_unknown_fields(sub, &["handler"])?;
    let handler = require_str_in(sub, "handler", "on_error.handler")?;
    Ok(Some(ErrorHandlerAst { handler }))
}

fn parse_wait(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["event", "timeout"])?;
    let event = match lookup(sub, "event") {
        Some(v) => Some(
            v.as_str()
                .ok_or(YamlError::FieldShape {
                    field: "wait.event",
                    expected: "string",
                })?
                .to_string(),
        ),
        None => None,
    };
    let timeout = match lookup(sub, "timeout") {
        Some(v) => Some(
            v.as_str()
                .ok_or(YamlError::FieldShape {
                    field: "wait.timeout",
                    expected: "string",
                })?
                .to_string(),
        ),
        None => None,
    };
    Ok(StepPrimitive::Wait { event, timeout })
}

fn parse_ask(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["prompt", "timeout"])?;
    let prompt = require_str_in(sub, "prompt", "ask.prompt")?;
    let timeout = match lookup(sub, "timeout") {
        Some(v) => Some(
            v.as_str()
                .ok_or(YamlError::FieldShape {
                    field: "ask.timeout",
                    expected: "string",
                })?
                .to_string(),
        ),
        None => None,
    };
    Ok(StepPrimitive::Ask { prompt, timeout })
}

fn parse_finish(sub: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    reject_unknown_fields(sub, &["result"])?;
    let result = require_scalar_in(sub, "result", "finish.result")?;
    Ok(StepPrimitive::Finish { result })
}
