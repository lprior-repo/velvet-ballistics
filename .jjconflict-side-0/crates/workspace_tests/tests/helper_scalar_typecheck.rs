use vb_compile::CompileError;
use vb_expr::ExprError;
use vb_expr::expr_typecheck::{ExprType, TypeContext, typecheck_expr};
use vb_expr::lexer::lex_expr;
use vb_expr::parser::parse_expr;

fn infer(source: &str) -> vb_expr::ExprResult<ExprType> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

fn infer_reference(helper: &str, reference: &str, ty: ExprType) -> vb_expr::ExprResult<ExprType> {
    let expression = format!("{helper}({reference})");
    let tokens = lex_expr(&expression)?;
    let ast = parse_expr(&tokens)?;
    let mut context = TypeContext::new();
    context.add_variable(reference.into(), ty);
    typecheck_expr(&ast, &context)
}

fn workflow_source(expression: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: helper_scalar\nwhen:\n  manual: {{}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: \"{expression}\"}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

fn assert_compiler_mismatch(expression: &str, found_type: &str) {
    let source = workflow_source(expression);
    let result = vb_compile::compile_workflow(source.as_bytes());
    let mut actual = Vec::new();
    let error_count = match result {
        Ok(_) => 0,
        Err(errors) => {
            let count = errors.0.len();
            for error in &errors.0 {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = error
                {
                    actual.push((*field, *expected, *found));
                }
            }
            count
        }
    };
    assert_eq!(error_count, 1);
    assert_eq!(actual, vec![("set.value", "list", found_type)]);
}

#[test]
fn scalar_literals_return_exact_helper_type_mismatches() {
    for helper in ["length", "count", "empty"] {
        for (literal, found) in [("1", "i64"), ("true", "boolean"), ("1.0", "f64")] {
            assert_eq!(
                infer(&format!("{helper}({literal})")),
                Err(ExprError::TypeMismatch {
                    expected: "list".into(),
                    found: found.into(),
                })
            );
        }
    }
}

#[test]
fn known_scalar_references_return_exact_helper_type_mismatches() {
    for helper in ["length", "count", "empty"] {
        for (ty, found) in [
            (ExprType::I64, "i64"),
            (ExprType::Bool, "boolean"),
            (ExprType::F64, "f64"),
        ] {
            assert_eq!(
                infer_reference(helper, "$known", ty),
                Err(ExprError::TypeMismatch {
                    expected: "list".into(),
                    found: found.into(),
                })
            );
        }
    }
}

#[test]
fn deferred_and_compatible_references_keep_helper_result_types() {
    for (helper, output) in [
        ("length", ExprType::I64),
        ("count", ExprType::I64),
        ("empty", ExprType::Bool),
    ] {
        assert_eq!(infer(&format!("{helper}(null)")), Ok(output));
        for (reference, ty) in [
            ("$items", ExprType::List),
            ("$deferred", ExprType::Unknown),
            ("$text", ExprType::Text),
            ("$object", ExprType::Object),
        ] {
            assert_eq!(infer_reference(helper, reference, ty), Ok(output));
        }
    }
}

#[test]
fn public_compiler_returns_exact_errors_for_all_scalar_helper_pairs() {
    for helper in ["length", "count", "empty"] {
        for (literal, found) in [("1", "i64"), ("true", "boolean"), ("1.0", "f64")] {
            assert_compiler_mismatch(&format!("{helper}({literal})"), found);
        }
    }
}
