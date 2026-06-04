#![forbid(unsafe_code)]

use serde_json::Value;
use vb_compile::ast::{
    AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, StepPrimitiveAst, TriggerAst,
    WorkflowAst,
};
use vb_compile::expression::ParsedExpression;

pub(crate) fn trigger_label(ast: Option<&WorkflowAst>) -> &'static str {
    match ast.map(|workflow| &workflow.trigger) {
        Some(TriggerAst::Manual { .. }) => "manual",
        Some(TriggerAst::Webhook { .. }) => "webhook",
        Some(TriggerAst::Schedule { .. }) => "schedule",
        Some(TriggerAst::Event { .. }) => "event",
        Some(_) => "unknown",
        None => "unknown",
    }
}

pub(crate) fn secrets_value(ast: Option<&WorkflowAst>) -> Value {
    serde_json::json!({
        "declared": declared_secrets(ast),
        "references_by_step": secret_references_by_step(ast),
        "source_metadata_available": ast.is_some(),
    })
}

pub(crate) fn declared_secrets(ast: Option<&WorkflowAst>) -> Vec<String> {
    ast.map_or_else(Vec::new, |workflow| {
        workflow
            .secrets
            .iter()
            .map(|entry| String::from(entry.name.as_ref()))
            .collect()
    })
}

pub(crate) fn secret_references_by_step(ast: Option<&WorkflowAst>) -> Vec<Value> {
    ast.map_or_else(Vec::new, |workflow| {
        workflow
            .steps
            .iter()
            .filter_map(secret_reference_entry)
            .collect()
    })
}

fn secret_reference_entry(step: &StepAst) -> Option<Value> {
    let secrets = step_secret_references(step);
    if secrets.is_empty() {
        None
    } else {
        Some(serde_json::json!({
            "step_id": step.id.as_ref(),
            "primitive": primitive_label(step.primitive),
            "secrets": secrets,
        }))
    }
}

fn step_secret_references(step: &StepAst) -> Vec<String> {
    let mut secrets = Vec::new();
    match &step.kind {
        StepKindAst::Save { fields } => fields
            .iter()
            .for_each(|field| collect_value_secrets(&field.value, &mut secrets)),
        StepKindAst::Choose { condition, .. } => {
            collect_expression_secrets(condition, &mut secrets)
        }
        StepKindAst::Reduce { initial, .. } => collect_value_secrets(initial, &mut secrets),
        StepKindAst::Finish { result } => collect_expression_secrets(result, &mut secrets),
        _ => {}
    }
    secrets
}

fn collect_value_secrets(value: &AstValue, secrets: &mut Vec<String>) {
    match value {
        AstValue::Reference(reference) => push_secret_reference(reference, secrets),
        AstValue::Sequence(items) => items
            .iter()
            .for_each(|item| collect_value_secrets(item, secrets)),
        AstValue::Mapping(entries) => entries.iter().for_each(|entry: &AstMapEntry<AstValue>| {
            collect_value_secrets(&entry.value, secrets);
        }),
        _ => {}
    }
}

fn collect_expression_secrets(expression: &AstExpression, secrets: &mut Vec<String>) {
    match expression {
        AstExpression::Reference(reference) => push_secret_reference(reference, secrets),
        AstExpression::Parsed(parsed) => collect_parsed_expression_secrets(parsed, secrets),
        AstExpression::Literal(value) => collect_value_secrets(value, secrets),
        _ => {}
    }
}

fn collect_parsed_expression_secrets(expression: &ParsedExpression, secrets: &mut Vec<String>) {
    match expression {
        ParsedExpression::Reference(reference) => push_secret_reference(reference, secrets),
        ParsedExpression::Unary { expr, .. } => collect_parsed_expression_secrets(expr, secrets),
        ParsedExpression::Binary { left, right, .. } => {
            collect_parsed_expression_secrets(left, secrets);
            collect_parsed_expression_secrets(right, secrets);
        }
        ParsedExpression::HelperCall { args, .. } => args
            .iter()
            .for_each(|arg| collect_parsed_expression_secrets(arg, secrets)),
        _ => {}
    }
}

fn push_secret_reference(reference: &str, secrets: &mut Vec<String>) {
    let Some(rest) = reference.strip_prefix("$secrets.") else {
        return;
    };
    let Some(name) = rest.split('.').next() else {
        return;
    };
    if name.is_empty() || secrets.iter().any(|existing| existing == name) {
        return;
    }
    secrets.push(String::from(name));
}

fn primitive_label(primitive: StepPrimitiveAst) -> &'static str {
    match primitive {
        StepPrimitiveAst::Set => "set",
        StepPrimitiveAst::Run => "run",
        StepPrimitiveAst::Do => "do",
        StepPrimitiveAst::Save => "save",
        StepPrimitiveAst::Choose => "choose",
        StepPrimitiveAst::ForEach => "for_each",
        StepPrimitiveAst::Together => "together",
        StepPrimitiveAst::Collect => "collect",
        StepPrimitiveAst::Reduce => "reduce",
        StepPrimitiveAst::Repeat => "repeat",
        StepPrimitiveAst::Wait => "wait",
        StepPrimitiveAst::Ask => "ask",
        StepPrimitiveAst::Finish => "finish",
        _ => "unknown",
    }
}
