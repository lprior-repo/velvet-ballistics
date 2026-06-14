use super::*;

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn parse(source: &str) -> Result<ParsedExpression, String> {
    parse_expression(source).map_err(|error| format!("expression parse failed: {error:?}"))
}

fn parse_err(source: &str) -> Result<CompileError, String> {
    match parse_expression(source) {
        Ok(expr) => Err(format!("expression parse unexpectedly succeeded: {expr:?}")),
        Err(error) => Ok(error),
    }
}

fn binary(
    expr: &ParsedExpression,
) -> Result<(BinaryOp, &ParsedExpression, &ParsedExpression), String> {
    match expr {
        ParsedExpression::Binary { op, left, right } => Ok((*op, left, right)),
        other => Err(format!("expected binary expression, got {other:?}")),
    }
}

fn unary(expr: &ParsedExpression) -> Result<(UnaryOp, &ParsedExpression), String> {
    match expr {
        ParsedExpression::Unary { op, expr } => Ok((*op, expr)),
        other => Err(format!("expected unary expression, got {other:?}")),
    }
}

fn ensure_ref(expr: &ParsedExpression, source: &'static str) -> Result<(), String> {
    match expr {
        ParsedExpression::Reference(reference) if reference.as_ref() == source => Ok(()),
        other => Err(format!("expected reference {source}, got {other:?}")),
    }
}

fn ensure_unexpected_char(
    error: CompileError,
    _source: &'static str,
    index: usize,
    found: char,
) -> Result<(), String> {
    match error {
        CompileError::ExpressionUnexpectedChar {
            index: actual,
            found: ch,
            ..
        } if actual == index && ch == found => Ok(()),
        other => Err(format!("unexpected char diagnostic mismatch: {other:?}")),
    }
}

fn ensure_limit(error: CompileError, limit_name: &'static str) -> Result<(), String> {
    match error {
        CompileError::ExpressionLimitExceeded { limit, .. } if limit == limit_name => Ok(()),
        other => Err(format!(
            "expected {limit_name} limit diagnostic, got {other:?}"
        )),
    }
}

fn helper(expr: &ParsedExpression) -> Result<(ExpressionHelper, &[ParsedExpression]), String> {
    match expr {
        ParsedExpression::HelperCall { name, args } => Ok((*name, args)),
        other => Err(format!("expected helper call, got {other:?}")),
    }
}

#[test]
fn parser_honors_multiplication_before_addition() -> Result<(), String> {
    let expr = parse("1 + 2 * 3")?;
    let (op, _, right) = binary(&expr)?;
    let (right_op, _, _) = binary(right)?;

    ensure(op == BinaryOp::Add, "root operator was not addition")?;
    ensure(
        right_op == BinaryOp::Mul,
        "multiplication did not bind tighter",
    )
}

#[test]
fn parser_keeps_subtraction_left_associative() -> Result<(), String> {
    let expr = parse("1 - 2 - 3")?;
    let (op, left, _) = binary(&expr)?;
    let (left_op, _, _) = binary(left)?;

    ensure(op == BinaryOp::Sub, "root operator was not subtraction")?;
    ensure(
        left_op == BinaryOp::Sub,
        "subtraction was not left associative",
    )
}

#[test]
fn parser_honors_textual_not_before_and_before_or() -> Result<(), String> {
    let expr = parse("not $input.a and $input.b or $input.c")?;
    let (op, left, right) = binary(&expr)?;
    let (left_op, not_expr, _) = binary(left)?;
    let (not_op, not_ref) = unary(not_expr)?;

    ensure(op == BinaryOp::Or, "or was not the root operator")?;
    ensure(left_op == BinaryOp::And, "and did not bind tighter than or")?;
    ensure(not_op == UnaryOp::Not, "not did not parse as unary")?;
    ensure_ref(not_ref, "$input.a")?;
    ensure_ref(right, "$input.c")
}

#[test]
fn parser_keeps_textual_and_left_associative() -> Result<(), String> {
    let expr = parse("$input.a and $input.b and $input.c")?;
    let (op, left, right) = binary(&expr)?;
    let (left_op, _, _) = binary(left)?;

    ensure(op == BinaryOp::And, "root operator was not and")?;
    ensure(left_op == BinaryOp::And, "and was not left associative")?;
    ensure_ref(right, "$input.c")
}

#[test]
fn parser_accepts_valid_rooted_references() -> Result<(), String> {
    ensure_ref(&parse("$input.x")?, "$input.x")?;
    ensure_ref(&parse("$vars.x")?, "$vars.x")?;
    ensure_ref(&parse("$secrets.x")?, "$secrets.x")
}

#[test]
fn lexer_rejects_symbolic_boolean_and_remainder_ops() -> Result<(), String> {
    ensure_unexpected_char(
        parse_err("$input.a && $input.b")?,
        "$input.a && $input.b",
        9,
        '&',
    )?;
    ensure_unexpected_char(
        parse_err("$input.a || $input.b")?,
        "$input.a || $input.b",
        9,
        '|',
    )?;
    ensure_unexpected_char(parse_err("!$input.a")?, "!$input.a", 0, '!')?;
    ensure_unexpected_char(parse_err("$input.a % 2")?, "$input.a % 2", 9, '%')
}

#[test]
fn parser_accepts_required_helper_call_surface() -> Result<(), String> {
    let expr = parse("contains($input.tags, \"urgent\")")?;
    let (name, args) = helper(&expr)?;

    ensure(
        name == ExpressionHelper::Contains,
        "helper name was not retained",
    )?;
    ensure(args.len() == 2, "helper args were not retained")
}

#[test]
fn parser_accepts_coalesce_helper_call() -> Result<(), String> {
    let expr = parse("coalesce($input.value, \"fallback\")")?;
    let (name, args) = helper(&expr)?;

    ensure(
        name == ExpressionHelper::Coalesce,
        "coalesce helper missing",
    )?;
    ensure(args.len() == 2, "coalesce arity was not retained")
}

#[test]
fn lexer_rejects_expression_token_limit() -> Result<(), String> {
    let source = "1 + ".repeat(MAX_EXPRESSION_TOKENS);
    ensure_limit(parse_err(&source)?, "token count")
}

#[test]
fn lexer_rejects_expression_source_length_limit() -> Result<(), String> {
    let source = "1".repeat(MAX_EXPRESSION_SOURCE_BYTES.saturating_add(1));
    ensure_limit(parse_err(&source)?, "source length")
}

#[test]
fn parser_rejects_expression_parse_depth_limit() -> Result<(), String> {
    let source = nested_expression_source();
    ensure_limit(parse_err(&source)?, "parse depth")
}

#[test]
fn parser_rejects_helper_arg_limit() -> Result<(), String> {
    let source = helper_arg_limit_source();
    ensure_limit(parse_err(&source)?, "helper args")
}

fn nested_expression_source() -> String {
    let open = "(".repeat(MAX_EXPRESSION_DEPTH_USIZE.saturating_add(2));
    let close = ")".repeat(MAX_EXPRESSION_DEPTH_USIZE.saturating_add(2));
    format!("{open}true{close}")
}

fn helper_arg_limit_source() -> String {
    let args = std::iter::repeat_n("1", MAX_HELPER_ARGS.saturating_add(1))
        .collect::<Vec<_>>()
        .join(", ");
    format!("count({args})")
}

#[test]
fn lexer_reports_unexpected_char_deterministically() -> Result<(), String> {
    let error = parse_err("$input.value @ 3")?;

    ensure(
        matches!(
            error,
            CompileError::ExpressionUnexpectedChar { index: 13, .. }
        ),
        "unexpected character did not report stable byte index",
    )
}

#[test]
fn parser_reports_missing_rhs_deterministically() -> Result<(), String> {
    let error = parse_err("$input.value ==")?;

    ensure(
        matches!(
            error,
            CompileError::ExpressionUnexpectedToken { index: 15, .. }
        ),
        "missing rhs did not report end byte index",
    )
}
