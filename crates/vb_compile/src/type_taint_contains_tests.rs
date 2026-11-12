use crate::ast::{AstExpression, StepKindAst, WorkflowAst};
use crate::expression::{ExpressionHelper, ParsedExpression};
use crate::{CompileError, YamlCompiler};

fn source(inputs: &str, expression: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: cold_contains\nwhen: {{manual: {{}}}}\ninputs:\n{inputs}\nsteps:\n  - id: route\n    choose:\n      condition: '{expression}'\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: null\n"
    )
}

fn parse(source: &str) -> Result<WorkflowAst, String> {
    YamlCompiler::default()
        .parse_ast(source.as_bytes())
        .map_err(|errors| format!("parse_ast failed: {errors:?}"))
}

fn assert_contains_ast(ast: &WorkflowAst) -> Result<(), String> {
    let condition = match ast.steps.first().map(|step| &step.kind) {
        Some(StepKindAst::Choose { condition, .. }) => condition,
        Some(kind) => return Err(format!("first step was not Choose: {kind:?}")),
        None => return Err("missing Choose step".to_owned()),
    };
    assert_eq!(
        condition,
        &AstExpression::Parsed(Box::new(ParsedExpression::HelperCall {
            name: ExpressionHelper::Contains,
            args: vec![
                ParsedExpression::Reference("$input.receiver".into()),
                ParsedExpression::Reference("$input.needle".into()),
            ],
        })),
    );
    Ok(())
}

fn assert_type_mismatch(source: &str, expected: CompileError) -> Result<(), String> {
    match YamlCompiler::default().parse_ast(source.as_bytes()) {
        Err(errors) => {
            assert_eq!(errors.0, vec![expected]);
            Ok(())
        }
        Ok(_) => Err("expected exact type mismatch, but parse_ast accepted".to_owned()),
    }
}

#[test]
fn cold_contains_text_text() -> Result<(), String> {
    let yaml = source(
        "  receiver: text\n  needle: text",
        "contains($input.receiver, $input.needle)",
    );
    assert_contains_ast(&parse(&yaml)?)
}

#[test]
fn cold_contains_list_boolean() -> Result<(), String> {
    let yaml = source(
        "  receiver: {is: list, of: any, max: 2}\n  needle: boolean",
        "contains($input.receiver, $input.needle)",
    );
    assert_contains_ast(&parse(&yaml)?)
}

#[test]
fn cold_contains_text_any() -> Result<(), String> {
    let yaml = source(
        "  receiver: text\n  needle: any",
        "contains($input.receiver, $input.needle)",
    );
    assert_contains_ast(&parse(&yaml)?)
}

#[test]
fn cold_contains_any_number() -> Result<(), String> {
    let yaml = source(
        "  receiver: any\n  needle: number",
        "contains($input.receiver, $input.needle)",
    );
    assert_contains_ast(&parse(&yaml)?)
}

#[test]
fn cold_contains_rejects_text_number() -> Result<(), String> {
    let yaml = source(
        "  receiver: text\n  needle: number",
        "contains($input.receiver, $input.needle)",
    );
    assert_type_mismatch(
        &yaml,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "text",
            found: "number",
        },
    )
}

#[test]
fn cold_contains_rejects_boolean_receiver() -> Result<(), String> {
    let yaml = source(
        "  receiver: boolean\n  needle: text",
        "contains($input.receiver, $input.needle)",
    );
    assert_type_mismatch(
        &yaml,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "text or list",
            found: "boolean",
        },
    )
}

#[test]
fn cold_contains_result_is_boolean() -> Result<(), String> {
    let yaml = source(
        "  receiver: any\n  needle: any",
        "contains($input.receiver, $input.needle) + 1",
    );
    assert_type_mismatch(
        &yaml,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "number",
            found: "boolean",
        },
    )
}
