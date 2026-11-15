//! Detects a canonical repeat's lexical attempt-number reads in cold source.

use super::part_14::MAX_BODY_NESTING_DEPTH;
use crate::expression::{ParsedExpression, parse_expression};
use crate::{AuthorValue, ScalarValue, StepAst, StepPrimitive};

pub(in crate::mod_compile_lowering) fn repeat_body_uses_attempt_number(steps: &[StepAst]) -> bool {
    body_uses_attempt_number(steps, 0)
}

fn body_uses_attempt_number(steps: &[StepAst], depth: u16) -> bool {
    depth <= MAX_BODY_NESTING_DEPTH
        && steps
            .iter()
            .any(|step| step_uses_attempt_number(step, depth))
}

fn nested_body_uses_attempt_number(steps: &[StepAst], depth: u16) -> bool {
    depth
        .checked_add(1)
        .is_some_and(|next| body_uses_attempt_number(steps, next))
}

fn step_uses_attempt_number(step: &StepAst, depth: u16) -> bool {
    step.condition
        .as_deref()
        .is_some_and(expression_uses_attempt_number)
        || step.retry.as_ref().is_some_and(|retry| {
            retry
                .delay
                .as_deref()
                .is_some_and(expression_uses_attempt_number)
        })
        || step.with.as_ref().is_some_and(value_uses_attempt_number)
        || primitive_uses_attempt_number(&step.primitive, depth)
}

fn primitive_uses_attempt_number(primitive: &StepPrimitive, depth: u16) -> bool {
    match primitive {
        StepPrimitive::Set { value, .. } | StepPrimitive::Do { input: value, .. } => {
            expression_uses_attempt_number(value)
        }
        StepPrimitive::Save { value } | StepPrimitive::Finish { result: value } => {
            scalar_uses_attempt_number(value)
        }
        StepPrimitive::Choose { branches, .. } => choose_uses_attempt_number(branches, depth),
        StepPrimitive::ForEach { input, body, .. }
        | StepPrimitive::Collect {
            source: input,
            body,
            ..
        }
        | StepPrimitive::Aggregate { input, body, .. } => {
            input_and_body_use_attempt_number(input, body, depth)
        }
        StepPrimitive::Together { branches } => together_uses_attempt_number(branches, depth),
        StepPrimitive::Repeat { .. } => false,
        StepPrimitive::Wait { event, timeout } => wait_uses_attempt_number(event, timeout),
        StepPrimitive::Ask { prompt, timeout } => ask_uses_attempt_number(prompt, timeout),
        _ => false,
    }
}

fn choose_uses_attempt_number(branches: &[crate::ChooseBranch], depth: u16) -> bool {
    branches.iter().any(|branch| {
        expression_uses_attempt_number(&branch.when)
            || nested_body_uses_attempt_number(&branch.steps, depth)
    })
}

fn input_and_body_use_attempt_number(input: &str, body: &[StepAst], depth: u16) -> bool {
    expression_uses_attempt_number(input) || nested_body_uses_attempt_number(body, depth)
}

fn together_uses_attempt_number(branches: &[crate::TogetherBranch], depth: u16) -> bool {
    branches
        .iter()
        .any(|branch| nested_body_uses_attempt_number(&branch.steps, depth))
}

fn wait_uses_attempt_number(event: &Option<String>, timeout: &Option<String>) -> bool {
    optional_expression_uses_attempt_number(event)
        || optional_expression_uses_attempt_number(timeout)
}

fn ask_uses_attempt_number(prompt: &str, timeout: &Option<String>) -> bool {
    expression_uses_attempt_number(prompt) || optional_expression_uses_attempt_number(timeout)
}

fn scalar_uses_attempt_number(value: &ScalarValue) -> bool {
    matches!(value, ScalarValue::String(text) if text.starts_with('$') && expression_uses_attempt_number(text))
}

fn optional_expression_uses_attempt_number(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(expression_uses_attempt_number)
}

fn value_uses_attempt_number(value: &AuthorValue) -> bool {
    let mut pending = vec![value];
    std::iter::from_fn(move || {
        pending.pop().map(|value| match value {
            AuthorValue::Text(text) => Some(text.as_str()),
            AuthorValue::Sequence(values) => {
                pending.extend(values.iter());
                None
            }
            AuthorValue::Mapping(entries) => {
                pending.extend(entries.iter().map(|entry| &entry.value));
                None
            }
            AuthorValue::Null | AuthorValue::Bool(_) | AuthorValue::I64(_) => None,
            _ => None,
        })
    })
    .flatten()
    .any(|text| text.starts_with('$') && expression_uses_attempt_number(text))
}

fn expression_uses_attempt_number(expression: &str) -> bool {
    match parse_expression(expression) {
        Ok(parsed) => parsed_uses_attempt_number(&parsed),
        Err(_) => false,
    }
}

fn parsed_uses_attempt_number(expression: &ParsedExpression) -> bool {
    match expression {
        ParsedExpression::Reference(reference) => reference.as_ref() == "$attempt.number",
        ParsedExpression::Unary { expr, .. } => parsed_uses_attempt_number(expr),
        ParsedExpression::Binary { left, right, .. } => {
            parsed_uses_attempt_number(left) || parsed_uses_attempt_number(right)
        }
        ParsedExpression::HelperCall { args, .. } => args.iter().any(parsed_uses_attempt_number),
        ParsedExpression::Literal(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{expression_uses_attempt_number, repeat_body_uses_attempt_number};
    use crate::{ChooseBranch, StepAst, StepPrimitive};

    #[test]
    fn parser_distinguishes_reference_from_literal_text() {
        assert!(expression_uses_attempt_number("$attempt.number"));
        assert!(!expression_uses_attempt_number("\"$attempt.number\""));
        assert!(!expression_uses_attempt_number("$attempt.number.bad"));
    }

    #[test]
    fn usage_scan_stops_at_the_existing_body_depth_limit() {
        let leaf = StepAst {
            id: String::from("leaf"),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set {
                output: String::from("answer"),
                value: String::from("$attempt.number"),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        };
        let nested = (0..=64).fold(vec![leaf], |steps, index| {
            vec![StepAst {
                id: format!("branch-{index}"),
                name: None,
                condition: None,
                primitive: StepPrimitive::Choose {
                    branches: vec![ChooseBranch {
                        when: String::from("true"),
                        steps,
                    }],
                    otherwise: None,
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }]
        });
        assert!(!repeat_body_uses_attempt_number(&nested));
    }
}
