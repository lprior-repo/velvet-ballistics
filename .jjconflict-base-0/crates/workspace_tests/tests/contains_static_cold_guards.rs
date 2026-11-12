use vb_compile::ast::{AstExpression, AstValue, StepKindAst, WorkflowAst};
use vb_compile::expression::{ExpressionHelper, ExpressionLiteral, ParsedExpression};
use vb_compile::{CompileError, CompileErrors, YamlCompiler};
use vb_core::FiniteF64;

fn parse_ast(inputs: &str, expression: &str) -> Result<WorkflowAst, CompileErrors> {
    let input_section = if inputs.is_empty() {
        "inputs: {}".to_string()
    } else {
        format!("inputs:\n{inputs}")
    };
    let source = format!(
        "version: velvet-ballistics/v1\nname: contains_static_guard\nwhen:\n  manual: {{}}\n{input_section}\nsteps:\n  - id: route\n    choose:\n      condition: '{expression}'\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish: {{ result: null }}\n"
    );
    YamlCompiler::default().parse_ast(source.as_bytes())
}

fn assert_input_schema(ast: &WorkflowAst, inputs: &[(&str, &str)]) -> Result<(), String> {
    assert_eq!(ast.inputs.len(), inputs.len());
    for (i, (name, schema)) in inputs.iter().enumerate() {
        let Some(actual) = ast.inputs.get(i) else {
            return Err(format!(
                "declared input at index {i} was not retained in the AST"
            ));
        };
        assert_eq!(actual.name.as_ref(), *name);
        assert_eq!(actual.value, AstValue::Text((*schema).into()));
    }
    Ok(())
}

fn assert_contains_ast(
    ast: &WorkflowAst,
    inputs: &[(&str, &str)],
    expected_args: &[ParsedExpression],
) -> Result<(), String> {
    assert_input_schema(ast, inputs)?;
    let Some(step) = ast.steps.first() else {
        return Err("choose step was not retained in the AST".to_string());
    };
    let StepKindAst::Choose { condition, .. } = &step.kind else {
        return Err("first AST step was not choose".to_string());
    };
    let AstExpression::Parsed(parsed) = condition else {
        return Err("choose condition was not retained as parsed expression".to_string());
    };
    let ParsedExpression::HelperCall { name, args } = parsed.as_ref() else {
        return Err("choose condition was not retained as helper call".to_string());
    };
    assert_eq!(*name, ExpressionHelper::Contains);
    assert_eq!(args.as_ref(), expected_args);
    Ok(())
}

#[test]
fn parse_ast_rejects_bool_contains_receiver_with_exact_cold_type_error() -> Result<(), String> {
    for expression in [
        "contains(true)",
        "contains(true, 42)",
        "contains(true, 42, 7)",
    ] {
        let errors = match parse_ast("", expression) {
            Ok(ast) => return Err(format!("invalid receiver was accepted: {ast:?}")),
            Err(errors) => errors,
        };
        assert_eq!(
            errors.0,
            vec![CompileError::TypeMismatch {
                field: "choose.condition",
                expected: "list",
                found: "boolean",
            }]
        );
    }
    Ok(())
}

#[test]
fn parse_ast_accepts_list_receiver_f64_needle_and_retains_schema_and_arguments()
-> Result<(), String> {
    let ast = parse_ast("  items: list<any>", "contains($input.items, 3.5)")
        .map_err(|errors| format!("List receiver should parse: {errors:?}"))?;
    let float = FiniteF64::new(3.5).map_err(|error| error.to_string())?;
    assert_contains_ast(
        &ast,
        &[("items", "list<any>")],
        &[
            ParsedExpression::Reference("$input.items".into()),
            ParsedExpression::Literal(ExpressionLiteral::F64(float)),
        ],
    )
}

#[test]
fn parse_ast_accepts_any_receiver_and_retains_schema_and_arguments() -> Result<(), String> {
    let ast = parse_ast("  value: any", "contains($input.value, null)")
        .map_err(|errors| format!("Any receiver should parse: {errors:?}"))?;
    assert_contains_ast(
        &ast,
        &[("value", "any")],
        &[
            ParsedExpression::Reference("$input.value".into()),
            ParsedExpression::Literal(ExpressionLiteral::Null),
        ],
    )
}

#[test]
fn parse_ast_retains_zero_one_and_three_argument_contains_without_cold_arity_checks()
-> Result<(), String> {
    let reference = ParsedExpression::Reference("$input.items".into());
    let cases = [
        ("contains()", Vec::new()),
        ("contains($input.items)", vec![reference.clone()]),
        (
            "contains($input.items, true, 7)",
            vec![
                reference,
                ParsedExpression::Literal(ExpressionLiteral::Bool(true)),
                ParsedExpression::Literal(ExpressionLiteral::I64(7)),
            ],
        ),
    ];
    for (expression, expected_args) in cases {
        let ast = parse_ast("  items: list<any>", expression)
            .map_err(|errors| format!("cold path added arity validation: {errors:?}"))?;
        assert_contains_ast(&ast, &[("items", "list<any>")], &expected_args)?;
    }
    Ok(())
}

#[test]
fn parse_ast_accepts_text_list_object_needle_with_list_receiver_and_retains_both_schemas()
-> Result<(), String> {
    for schema in ["text", "list<any>", "object"] {
        let inputs = format!("  items: list<any>\n  needle: {schema}");
        let ast = parse_ast(&inputs, "contains($input.items, $input.needle)")
            .map_err(|errors| format!("needles of {schema} should parse: {errors:?}"))?;
        assert_contains_ast(
            &ast,
            &[("items", "list<any>"), ("needle", schema)],
            &[
                ParsedExpression::Reference("$input.items".into()),
                ParsedExpression::Reference("$input.needle".into()),
            ],
        )?;
    }
    Ok(())
}
