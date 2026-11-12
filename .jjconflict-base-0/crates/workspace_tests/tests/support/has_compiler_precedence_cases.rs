use vb_compile::CompileError;

use super::{only_error, parse_ast_with_inputs, workflow_compile};

#[test]
fn workflow_typecheck_precedence_bool_receiver() -> Result<(), String> {
    let Err(errors) = workflow_compile("has(true,42)") else {
        return Err("expected type failure".to_owned());
    };
    assert!(matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "set.value",
            expected: "object",
            found: "boolean"
        }
    ));
    Ok(())
}

#[test]
fn parse_ast_typecheck_precedence_bool_receiver() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[], "has(true,42)") else {
        return Err("expected type failure".to_owned());
    };
    assert!(matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "object",
            found: "boolean"
        }
    ));
    Ok(())
}

#[test]
fn parse_ast_rejects_bool_receiver_with_one_argument() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[], "has(true)") else {
        return Err("expected type failure".to_owned());
    };
    assert!(matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "object",
            found: "boolean"
        }
    ));
    Ok(())
}

#[test]
fn parse_ast_rejects_numeric_key_with_three_arguments() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[("obj", "any")], "has($input.obj, 42, 1)") else {
        return Err("expected type failure".to_owned());
    };
    assert!(matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "text",
            found: "number"
        }
    ));
    Ok(())
}

#[test]
fn parse_ast_three_arguments_report_receiver_first() -> Result<(), String> {
    let Err(errors) = parse_ast_with_inputs(&[], "has(true, 42, 1)") else {
        return Err("expected type failure".to_owned());
    };
    assert!(matches!(
        only_error(&errors)?,
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "object",
            found: "boolean"
        }
    ));
    Ok(())
}
