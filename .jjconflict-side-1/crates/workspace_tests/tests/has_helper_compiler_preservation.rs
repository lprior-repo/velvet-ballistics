use vb_compile::CompileError;

#[path = "support/has_compiler_artifacts.rs"]
mod has_compiler_artifacts;
#[path = "support/has_compiler_precedence_cases.rs"]
mod has_precedence_cases;
include!("support/has_compiler_artifact_cases.rs");
use has_compiler_artifacts::{
    compile_with_inputs, only_error, parse_ast_with_inputs, workflow_compile,
};

#[test]
fn compile_preserves_bool_receiver_mismatch() -> Result<(), String> {
    let Err(errors) = workflow_compile("has(true, \"key\")") else {
        return Err("expected canonical compilation failure".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "set.value",
            expected: "object",
            found: "boolean"
        }
    ) {
        return Err(format!(
            "unexpected canonical error: {:?}",
            errors.0.first()
        ));
    }
    Ok(())
}

#[test]
fn compile_preserves_i64_key_mismatch() -> Result<(), String> {
    let Err(errors) = compile_with_inputs(&[("obj", "object")], "has($input.obj, 42)") else {
        return Err("expected canonical compilation failure".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "set.value",
            expected: "text",
            found: "i64"
        }
    ) {
        return Err(format!(
            "unexpected canonical error: {:?}",
            errors.0.first()
        ));
    }
    Ok(())
}

#[test]
fn compile_rejects_arity_zero() -> Result<(), String> {
    let Err(errors) = workflow_compile("has()") else {
        return Err("expected arity-zero rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::ExpressionHelperArity {
            helper: "has",
            expected: 2,
            actual: 0
        }
    ) {
        return Err(format!("unexpected arity error: {:?}", errors.0.first()));
    }
    Ok(())
}

#[test]
fn compile_rejects_arity_one() -> Result<(), String> {
    let Err(errors) = compile_with_inputs(&[("obj", "object")], "has($input.obj)") else {
        return Err("expected arity-one rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::ExpressionHelperArity {
            helper: "has",
            expected: 2,
            actual: 1
        }
    ) {
        return Err(format!("unexpected arity error: {:?}", errors.0.first()));
    }
    Ok(())
}

#[test]
fn compile_rejects_arity_three() -> Result<(), String> {
    let Err(errors) =
        compile_with_inputs(&[("obj", "object")], "has($input.obj, \"key\", \"extra\")")
    else {
        return Err("expected arity-three rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::ExpressionHelperArity {
            helper: "has",
            expected: 2,
            actual: 3
        }
    ) {
        return Err(format!("unexpected arity error: {:?}", errors.0.first()));
    }
    Ok(())
}

#[test]
fn parse_ast_rejects_bool_receiver() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[("key", "any")], "has(true, $input.key)") else {
        return Err("expected parse-AST receiver rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "object",
            found: "boolean"
        }
    ) {
        return Err(format!(
            "unexpected parse-AST error: {:?}",
            errors.0.first()
        ));
    }
    Ok(())
}

#[test]
fn parse_ast_rejects_i64_key() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[("obj", "any")], "has($input.obj, 42)") else {
        return Err("expected parse-AST key rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "text",
            found: "number"
        }
    ) {
        return Err(format!(
            "unexpected parse-AST error: {:?}",
            errors.0.first()
        ));
    }
    Ok(())
}

#[test]
fn parse_ast_preserves_has_bool_type_before_arithmetic() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(
        &[("obj", "any"), ("key", "any")],
        "has($input.obj, $input.key) + 1",
    ) else {
        return Err("expected parse-AST boolean arithmetic type rejection".to_string());
    };
    if !matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "number",
            found: "boolean"
        }
    ) {
        return Err(format!(
            "unexpected parse-AST arithmetic error: {:?}",
            errors.0.first()
        ));
    }
    Ok(())
}
