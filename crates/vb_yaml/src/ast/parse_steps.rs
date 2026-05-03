//! Step parsing logic.

use crate::{YamlError, YamlResult};
use super::types::*;
use super::parse::{lookup, require_str_in, require_scalar_in, opt_str, opt_u32, require_u16};

pub(super) fn parse_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };
    let mut steps = Vec::new();
    for item in seq {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_step(yaml: &saphyr::Yaml<'_>) -> YamlResult<StepAst> {
    if !yaml.is_mapping() {
        return Err(YamlError::FieldShape { field: "step", expected: "mapping" });
    }
    let id = require_str_in(yaml, "id", "step.id")?;
    let name = opt_str(yaml, "name");
    let condition = opt_str(yaml, "if");
    let primitive = parse_step_primitive(yaml)?;
    let with = opt_str(yaml, "with");
    let retry = parse_retry(yaml)?;
    let on_error = parse_error_handler(yaml)?;
    let then = opt_str(yaml, "then");
    Ok(StepAst { id, name, condition, primitive, with, retry, on_error, then })
}

fn parse_step_primitive(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    // Set
    if let Some(sub) = lookup(node, "set") && sub.is_mapping() {
        let output = require_str_in(sub, "output", "set.output")?;
        let value = require_str_in(sub, "value", "set.value")?;
        return Ok(StepPrimitive::Set { output, value });
    }
    // Save
    if let Some(sub) = lookup(node, "save") && sub.is_mapping() {
        let value = require_scalar_in(sub, "value", "save.value")?;
        return Ok(StepPrimitive::Save { value });
    }
    // Do
    if let Some(sub) = lookup(node, "do") && sub.is_mapping() {
        let action = require_str_in(sub, "action", "do.action")?;
        let input = require_str_in(sub, "input", "do.input")?;
        return Ok(StepPrimitive::Do { action, input });
    }
    // Choose
    if let Some(sub) = lookup(node, "choose") && sub.is_mapping() {
        return parse_choose(sub);
    }
    // ForEach
    if let Some(sub) = lookup(node, "foreach") && sub.is_mapping() {
        return parse_foreach(sub);
    }
    // Together
    if let Some(sub) = lookup(node, "together") && sub.is_mapping() {
        return parse_together(sub);
    }
    // Collect
    if let Some(sub) = lookup(node, "collect") && sub.is_mapping() {
        return parse_collect(sub);
    }
    // Reduce
    if let Some(sub) = lookup(node, "reduce") && sub.is_mapping() {
        return parse_reduce(sub);
    }
    // Repeat
    if let Some(sub) = lookup(node, "repeat") && sub.is_mapping() {
        return parse_repeat(sub);
    }
    // Wait
    if let Some(sub) = lookup(node, "wait") && sub.is_mapping() {
        let event = opt_str(sub, "event");
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Wait { event, timeout });
    }
    // Ask
    if let Some(sub) = lookup(node, "ask") && sub.is_mapping() {
        let prompt = require_str_in(sub, "prompt", "ask.prompt")?;
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Ask { prompt, timeout });
    }
    // Finish
    if let Some(sub) = lookup(node, "finish") && sub.is_mapping() {
        let result = require_scalar_in(sub, "result", "finish.result")?;
        return Ok(StepPrimitive::Finish { result });
    }
    Err(YamlError::MissingField {
        field: "step primitive (set/save/do/choose/foreach/together/collect/reduce/repeat/wait/ask/finish)",
    })
}

fn parse_choose(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut branches = Vec::new();
    if let Some(seq) = lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape { field: "choose.branches[]", expected: "mapping" });
            }
            let when = require_str_in(item, "when", "choose.branches[].when")?;
            let steps = parse_body_steps(item)?;
            branches.push(ChooseBranch { when, steps });
        }
    }
    let otherwise = opt_str(node, "otherwise");
    Ok(StepPrimitive::Choose { branches, otherwise })
}

fn parse_foreach(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "foreach.variable")?;
    let input = require_str_in(node, "input", "foreach.input")?;
    let at_once = opt_u32(node, "at_once");
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::ForEach { variable, input, at_once, body })
}

fn parse_together(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut branches = Vec::new();
    if let Some(seq) = lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape { field: "together.branches[]", expected: "mapping" });
            }
            let label = require_str_in(item, "label", "together.branches[].label")?;
            let steps = parse_body_steps(item)?;
            branches.push(TogetherBranch { label, steps });
        }
    }
    Ok(StepPrimitive::Together { branches })
}

fn parse_collect(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "collect.variable")?;
    let source = require_str_in(node, "source", "collect.source")?;
    let pages = opt_u32(node, "pages");
    let items = opt_u32(node, "items");
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Collect { variable, source, pages, items, body })
}

fn parse_reduce(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let variable = require_str_in(node, "variable", "reduce.variable")?;
    let input = require_str_in(node, "input", "reduce.input")?;
    let initial = require_str_in(node, "initial", "reduce.initial")?;
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Reduce { variable, input, initial, body })
}

fn parse_repeat(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let max_attempts = require_u16(node, "max_attempts")?;
    let body = parse_body_steps(node)?;
    Ok(StepPrimitive::Repeat { max_attempts, body })
}

fn parse_body_steps(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<StepAst>> {
    let Some(seq) = lookup(node, "steps").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };
    let mut steps = Vec::new();
    for item in seq {
        steps.push(parse_step(item)?);
    }
    Ok(steps)
}

fn parse_retry(node: &saphyr::Yaml<'_>) -> YamlResult<Option<RetryPolicy>> {
    let Some(sub) = lookup(node, "retry") else { return Ok(None) };
    if !sub.is_mapping() { return Ok(None); }
    let max_attempts = require_u16(sub, "max_attempts")?;
    let delay = opt_str(sub, "delay");
    Ok(Some(RetryPolicy { max_attempts, delay }))
}

fn parse_error_handler(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ErrorHandlerAst>> {
    let Some(sub) = lookup(node, "on_error") else { return Ok(None) };
    if !sub.is_mapping() { return Ok(None); }
    let handler = require_str_in(sub, "handler", "on_error.handler")?;
    Ok(Some(ErrorHandlerAst { handler }))
}
