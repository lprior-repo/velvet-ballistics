// ── CompileErrors first() method ─────────────────────────────────────────────

#[test]
fn compile_errors_first_returns_first_error() {
    let source = LAST_STEP_SAVE_SOURCE;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let first = errors.first();
    assert!(first.is_some());
}

// ── CompileErrors len() and is_empty() ─────────────────────────────────────

#[test]
fn compile_errors_len_and_is_empty() {
    let source = LAST_STEP_SAVE_SOURCE;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    assert!(!errors.is_empty());
    assert!(errors.len() >= 1);
}

// ── CompileErrors iter() ─────────────────────────────────────────────────────

#[test]
fn compile_errors_iter_works() {
    let source = LAST_STEP_SAVE_SOURCE;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let count = errors.iter().count();
    assert!(count >= 1);
}

// ── CompileErrors as_slice() ─────────────────────────────────────────────────

#[test]
fn compile_errors_as_slice_returns_all_errors() {
    let source = LAST_STEP_SAVE_SOURCE;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let slice = errors.as_slice();
    assert!(!slice.is_empty());
}

// ── YamlCompiler Default ─────────────────────────────────────────────────────

#[test]
fn yaml_compiler_default_constructs() {
    let _compiler = YamlCompiler::default();
}

// ── YamlCompiler parse_ast returns Err for invalid source ────────────────────

#[test]
fn yaml_compiler_rejects_invalid_source() {
    let source = b"not yaml at all";
    let result = YamlCompiler::default().parse_ast(source);
    assert!(result.is_err());
}

// ── YamlCompiler compile returns Err for invalid source ──────────────────────

#[test]
fn yaml_compiler_rejects_invalid_source_via_compile() {
    let source = b"not yaml at all";
    let result = YamlCompiler::default().compile(source);
    assert!(result.is_err());
}

// ── Parse expressions tests ─────────────────────────────────────────────────

#[test]
fn parse_expression_accepts_integer_literals() {
    use crate::expression::{ExpressionLiteral, ParsedExpression, parse_expression};
    let result = parse_expression("42");
    assert!(result.is_ok());
    if let Ok(ParsedExpression::Literal(ExpressionLiteral::I64(n))) = result {
        assert_eq!(n, 42);
    } else {
        panic!("expected integer literal");
    }
}

#[test]
fn parse_expression_accepts_boolean_literals() {
    use crate::expression::{ExpressionLiteral, ParsedExpression, parse_expression};
    let result = parse_expression("true");
    assert!(result.is_ok());
    if let Ok(ParsedExpression::Literal(ExpressionLiteral::Bool(b))) = result {
        assert!(b);
    } else {
        panic!("expected boolean literal");
    }
}

#[test]
fn parse_expression_rejects_invalid_syntax() {
    use crate::expression::parse_expression;
    let result = parse_expression("= 1 + 2");
    assert!(result.is_err());
}

// ── Strict YAML profile tests ────────────────────────────────────────────────

#[test]
fn strict_yaml_rejects_alias() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("&anchor value");
    assert!(result.is_err());
}

#[test]
fn strict_yaml_accepts_single_document() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("key: value");
    assert!(result.is_ok());
}

#[test]
fn strict_yaml_rejects_multiple_documents() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("key1: value1\n---\nkey2: value2");
    assert!(result.is_err());
}

