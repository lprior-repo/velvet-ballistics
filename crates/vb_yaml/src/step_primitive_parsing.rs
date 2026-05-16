//! Step primitive parsing.

use crate::{YamlError, YamlResult};

use super::types::{
    opt_str, opt_u32, require_scalar_in, require_str_in, require_u16,
};
use super::step::{ChooseBranch, StepPrimitive, TogetherBranch};
use super::step_parsing::parse_body_steps;

// ---------------------------------------------------------------------------
// Step primitive parsing
// ---------------------------------------------------------------------------

pub fn parse_step_primitive(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    // Set
    if let Some(sub) = super::types::lookup(node, "set")
        && sub.is_mapping()
    {
        let output = require_str_in(sub, "output", "set.output")?;
        let value = require_str_in(sub, "value", "set.value")?;
        return Ok(StepPrimitive::Set { output, value });
    }

    // Save (alias accepted by compile layer)
    if let Some(sub) = super::types::lookup(node, "save")
        && sub.is_mapping()
    {
        let value = require_scalar_in(sub, "value", "save.value")?;
        return Ok(StepPrimitive::Save { value });
    }

    // Do
    if let Some(sub) = super::types::lookup(node, "do")
        && sub.is_mapping()
    {
        let action = require_str_in(sub, "action", "do.action")?;
        let input = require_str_in(sub, "input", "do.input")?;
        return Ok(StepPrimitive::Do { action, input });
    }

    // Choose
    if let Some(sub) = super::types::lookup(node, "choose")
        && sub.is_mapping()
    {
        return parse_choose(sub);
    }

    // ForEach
    if let Some(sub) = super::types::lookup(node, "foreach")
        && sub.is_mapping()
    {
        return parse_foreach(sub);
    }

    // Together
    if let Some(sub) = super::types::lookup(node, "together")
        && sub.is_mapping()
    {
        return parse_together(sub);
    }

    // Collect
    if let Some(sub) = super::types::lookup(node, "collect")
        && sub.is_mapping()
    {
        return parse_collect(sub);
    }

    // Reduce
    if let Some(sub) = super::types::lookup(node, "reduce")
        && sub.is_mapping()
    {
        return parse_reduce(sub);
    }

    // Repeat
    if let Some(sub) = super::types::lookup(node, "repeat")
        && sub.is_mapping()
    {
        return parse_repeat(sub);
    }

    // Wait
    if let Some(sub) = super::types::lookup(node, "wait")
        && sub.is_mapping()
    {
        let event = opt_str(sub, "event");
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Wait { event, timeout });
    }

    // Ask
    if let Some(sub) = super::types::lookup(node, "ask")
        && sub.is_mapping()
    {
        let prompt = require_str_in(sub, "prompt", "ask.prompt")?;
        let timeout = opt_str(sub, "timeout");
        return Ok(StepPrimitive::Ask { prompt, timeout });
    }

    // Finish
    if let Some(sub) = super::types::lookup(node, "finish")
        && sub.is_mapping()
    {
        let result = require_scalar_in(sub, "result", "finish.result")?;
        return Ok(StepPrimitive::Finish { result });
    }

    Err(YamlError::MissingField {
        field: "step primitive (set/save/do/choose/foreach/together/collect/reduce/repeat/wait/ask/finish)",
    })
}

fn parse_choose(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
    let mut branches = Vec::new();

    if let Some(seq) = super::types::lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape {
                    field: "choose.branches[]",
                    expected: "mapping",
                });
            }
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
    let mut branches = Vec::new();

    if let Some(seq) = super::types::lookup(node, "branches").and_then(|v| v.as_vec()) {
        for item in seq {
            if !item.is_mapping() {
                return Err(YamlError::FieldShape {
                    field: "together.branches[]",
                    expected: "mapping",
                });
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

    Ok(StepPrimitive::Collect {
        variable,
        source,
        pages,
        items,
        body,
    })
}

fn parse_reduce(node: &saphyr::Yaml<'_>) -> YamlResult<StepPrimitive> {
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
    let max_attempts = require_u16(node, "max_attempts")?;
    let body = parse_body_steps(node)?;

    Ok(StepPrimitive::Repeat { max_attempts, body })
}
