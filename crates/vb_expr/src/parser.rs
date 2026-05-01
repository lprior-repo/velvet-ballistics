//! Expression parser producing a typed AST from token streams.

use crate::lexer::{BinaryOp, LiteralToken, Token, UnaryOp, infix_binding_power};
use crate::{ExprError, ExprResult};

/// Maximum nesting depth for the parser.
const MAX_DEPTH: u8 = 64;
/// Maximum helper call arguments.
const MAX_HELPER_ARGS: usize = 8;

/// Parsed expression AST node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprAst {
    /// Null, boolean, integer, or string literal.
    Literal(ExprLiteral),
    /// Source reference beginning with `$`.
    Reference(Box<str>),
    /// Prefix unary expression.
    Unary {
        /// Unary operator.
        op: UnaryOp,
        /// Operand expression.
        expr: Box<ExprAst>,
    },
    /// Infix binary expression.
    Binary {
        /// Binary operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<ExprAst>,
        /// Right operand.
        right: Box<ExprAst>,
    },
    /// Built-in helper call.
    Helper {
        /// Helper name.
        name: ExprHelper,
        /// Parsed helper arguments.
        args: Box<[ExprAst]>,
    },
}

/// Literal value in the expression AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprLiteral {
    /// Null literal.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// Double-quoted string literal.
    Text(Box<str>),
}

/// Built-in helper function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprHelper {
    /// `contains(value, needle)`.
    Contains,
    /// `starts_with(value, prefix)`.
    StartsWith,
    /// `ends_with(value, suffix)`.
    EndsWith,
    /// `has(object, key)`.
    Has,
    /// `exists(value)`.
    Exists,
    /// `length(value)`.
    Length,
    /// `empty(value)`.
    Empty,
    /// `append(list, value)`.
    Append,
    /// `append_if(list, value, condition)`.
    AppendIf,
    /// `merge(left, right)`.
    Merge,
    /// `sum(list)`.
    Sum,
    /// `count(list)`.
    Count,
    /// `unique(list)`.
    Unique,
}

/// Parses a complete expression from a token slice.
pub fn parse_expr(tokens: &[Token]) -> ExprResult<ExprAst> {
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_precedence(0, 0)?;
    match parser.current() {
        Token::End => Ok(expr),
        _ => Err(ExprError::UnexpectedToken {
            token: format!("{:?}", parser.current()),
        }),
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_precedence(&mut self, min_bp: u8, depth: u8) -> ExprResult<ExprAst> {
        self.check_depth(depth)?;
        let mut left = self.parse_prefix(depth)?;
        while let Token::Operator(op) = self.current().clone() {
            let (left_bp, right_bp) = infix_binding_power(op);
            if left_bp < min_bp {
                break;
            }
            self.advance();
            let right = self.parse_precedence(right_bp, depth.saturating_add(1))?;
            left = ExprAst::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self, depth: u8) -> ExprResult<ExprAst> {
        match self.current().clone() {
            Token::Literal(lit) => {
                let ast = literal_ast(lit);
                self.advance();
                Ok(ast)
            }
            Token::Reference(value) => {
                let ast = ExprAst::Reference(value);
                self.advance();
                Ok(ast)
            }
            Token::Unary(op) => self.parse_unary(op, depth),
            Token::Operator(BinaryOp::Sub) => self.parse_unary(UnaryOp::Neg, depth),
            Token::LParen => self.parse_parenthesized(depth),
            Token::Identifier(name) => self.parse_ident(name, depth),
            _ => Err(ExprError::UnexpectedToken {
                token: format!("{:?}", self.current()),
            }),
        }
    }

    fn parse_unary(&mut self, op: UnaryOp, depth: u8) -> ExprResult<ExprAst> {
        self.advance();
        let expr = self.parse_precedence(11, depth.saturating_add(1))?;
        Ok(ExprAst::Unary {
            op,
            expr: Box::new(expr),
        })
    }

    fn parse_parenthesized(&mut self, depth: u8) -> ExprResult<ExprAst> {
        self.advance();
        let expr = self.parse_precedence(0, depth.saturating_add(1))?;
        if !matches!(self.current(), Token::RParen) {
            return Err(ExprError::UnexpectedToken {
                token: "expected right parenthesis".into(),
            });
        }
        self.advance();
        Ok(expr)
    }

    fn parse_ident(&mut self, name: Box<str>, depth: u8) -> ExprResult<ExprAst> {
        if matches!(self.peek(), Token::LParen) {
            return self.parse_helper_call(&name, depth);
        }
        Err(ExprError::UnexpectedToken {
            token: format!("unknown identifier: {name}"),
        })
    }

    fn parse_helper_call(&mut self, name: &str, depth: u8) -> ExprResult<ExprAst> {
        let helper = parse_helper_name(name).ok_or_else(|| ExprError::UnknownHelper {
            helper: name.into(),
        })?;
        self.advance();
        self.advance();
        let args = self.parse_helper_args(depth)?;
        validate_helper_arity(helper, args.len())?;
        Ok(ExprAst::Helper { name: helper, args })
    }

    fn parse_helper_args(&mut self, depth: u8) -> ExprResult<Box<[ExprAst]>> {
        let mut args = Vec::with_capacity(2);
        if matches!(self.current(), Token::RParen) {
            self.advance();
            return Ok(args.into_boxed_slice());
        }
        self.parse_one_or_more_args(&mut args, depth)
    }

    fn parse_one_or_more_args(
        &mut self,
        args: &mut Vec<ExprAst>,
        depth: u8,
    ) -> ExprResult<Box<[ExprAst]>> {
        loop {
            if args.len() >= MAX_HELPER_ARGS {
                return Err(ExprError::TooManyHelperArgs {
                    len: args.len().saturating_add(1),
                    max: MAX_HELPER_ARGS,
                });
            }
            args.push(self.parse_precedence(0, depth.saturating_add(1))?);
            match self.current() {
                Token::Comma => self.advance(),
                Token::RParen => {
                    self.advance();
                    return Ok(core::mem::take(args).into_boxed_slice());
                }
                _ => {
                    return Err(ExprError::UnexpectedToken {
                        token: "expected comma or right parenthesis".into(),
                    });
                }
            }
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.index).unwrap_or(&Token::End)
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.index.saturating_add(1))
            .unwrap_or(&Token::End)
    }

    fn advance(&mut self) {
        self.index = self.index.saturating_add(1);
    }

    fn check_depth(&self, depth: u8) -> ExprResult<()> {
        if depth > MAX_DEPTH {
            Err(ExprError::ParseDepthExceeded {
                max: usize::from(MAX_DEPTH),
            })
        } else {
            Ok(())
        }
    }
}

fn literal_ast(lit: LiteralToken) -> ExprAst {
    match lit {
        LiteralToken::Null => ExprAst::Literal(ExprLiteral::Null),
        LiteralToken::Bool(v) => ExprAst::Literal(ExprLiteral::Bool(v)),
        LiteralToken::I64(v) => ExprAst::Literal(ExprLiteral::I64(v)),
        LiteralToken::Text(v) => ExprAst::Literal(ExprLiteral::Text(v)),
    }
}

/// Maps a helper name string to the typed helper enum.
pub fn parse_helper_name(name: &str) -> Option<ExprHelper> {
    match name {
        "contains" => Some(ExprHelper::Contains),
        "starts_with" => Some(ExprHelper::StartsWith),
        "ends_with" => Some(ExprHelper::EndsWith),
        "has" => Some(ExprHelper::Has),
        "exists" => Some(ExprHelper::Exists),
        "length" => Some(ExprHelper::Length),
        "empty" => Some(ExprHelper::Empty),
        "append" => Some(ExprHelper::Append),
        "append_if" => Some(ExprHelper::AppendIf),
        "merge" => Some(ExprHelper::Merge),
        "sum" => Some(ExprHelper::Sum),
        "count" => Some(ExprHelper::Count),
        "unique" => Some(ExprHelper::Unique),
        _ => None,
    }
}

/// Returns the expected arity for a helper.
pub const fn helper_arity(helper: ExprHelper) -> usize {
    match helper {
        ExprHelper::Exists
        | ExprHelper::Length
        | ExprHelper::Empty
        | ExprHelper::Sum
        | ExprHelper::Count
        | ExprHelper::Unique => 1,
        ExprHelper::AppendIf => 3,
        _ => 2,
    }
}

/// Returns the canonical name of a helper.
pub const fn helper_name(helper: ExprHelper) -> &'static str {
    match helper {
        ExprHelper::Contains => "contains",
        ExprHelper::StartsWith => "starts_with",
        ExprHelper::EndsWith => "ends_with",
        ExprHelper::Has => "has",
        ExprHelper::Exists => "exists",
        ExprHelper::Length => "length",
        ExprHelper::Empty => "empty",
        ExprHelper::Append => "append",
        ExprHelper::AppendIf => "append_if",
        ExprHelper::Merge => "merge",
        ExprHelper::Sum => "sum",
        ExprHelper::Count => "count",
        ExprHelper::Unique => "unique",
    }
}

fn validate_helper_arity(helper: ExprHelper, actual: usize) -> ExprResult<()> {
    let expected = helper_arity(helper);
    if actual == expected {
        Ok(())
    } else {
        Err(ExprError::HelperArityMismatch {
            helper: helper_name(helper).into(),
            expected,
            actual,
        })
    }
}

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ExprResult<ExprAst> {
        let tokens = crate::lexer::lex_expr(source)?;
        parse_expr(&tokens)
    }

    #[test]
    fn parses_addition_with_multiplication_precedence() -> ExprResult<()> {
        let expr = parse("1 + 2 * 3")?;
        let (op, _, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Add);
        let (right_op, _, _) = as_binary(right)?;
        assert_eq!(right_op, BinaryOp::Mul);
        Ok(())
    }

    #[test]
    fn parses_left_associative_subtraction() -> ExprResult<()> {
        let expr = parse("1 - 2 - 3")?;
        let (op, left, _) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Sub);
        let (left_op, _, _) = as_binary(left)?;
        assert_eq!(left_op, BinaryOp::Sub);
        Ok(())
    }

    #[test]
    fn parses_not_and_or_precedence() -> ExprResult<()> {
        let expr = parse("not $a and $b or $c")?;
        let (op, left, _) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Or);
        let (left_op, not_expr, _) = as_binary(left)?;
        assert_eq!(left_op, BinaryOp::And);
        let (not_op, _) = as_unary(not_expr)?;
        assert_eq!(not_op, UnaryOp::Not);
        Ok(())
    }

    #[test]
    fn parses_helper_call() -> ExprResult<()> {
        let expr = parse("contains($tags, \"urgent\")")?;
        let (name, args) = as_helper(&expr)?;
        assert_eq!(name, ExprHelper::Contains);
        assert_eq!(args.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_unknown_helper() {
        let result = parse("unknown_func(1)");
        assert!(matches!(result, Err(ExprError::UnknownHelper { .. })));
    }

    #[test]
    fn rejects_wrong_arity() {
        let result = parse("contains(1)");
        assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
    }

    #[test]
    fn rejects_parse_depth() {
        let open = "(".repeat(usize::from(MAX_DEPTH).saturating_add(2));
        let close = ")".repeat(usize::from(MAX_DEPTH).saturating_add(2));
        let source = format!("{open}true{close}");
        let result = parse(&source);
        assert!(matches!(result, Err(ExprError::ParseDepthExceeded { .. })));
    }

    // --- BDD parser tests ---

    #[test]
    fn parse_expr_parses_simple_addition() -> ExprResult<()> {
        // Given: the expression "5 + 3"
        // When: parse_expr is called
        // Then: the AST is Binary(Add, Literal(I64(5)), Literal(I64(3)))
        let expr = parse("5 + 3")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
        Ok(())
    }

    #[test]
    fn parse_expr_parses_operator_precedence_correctly() -> ExprResult<()> {
        // Given: the expression "1 + 2 * 3"
        // When: parse_expr is called
        // Then: multiplication binds tighter than addition
        let expr = parse("1 + 2 * 3")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
        let (inner_op, _, _) = as_binary(right)?;
        assert_eq!(inner_op, BinaryOp::Mul);
        Ok(())
    }

    #[test]
    fn parse_expr_parses_parenthesized_grouping() -> ExprResult<()> {
        // Given: the expression "(1 + 2) * 3"
        // When: parse_expr is called
        // Then: addition is grouped inside the multiplication
        let expr = parse("(1 + 2) * 3")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Mul);
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
        let (inner_op, _, _) = as_binary(left)?;
        assert_eq!(inner_op, BinaryOp::Add);
        Ok(())
    }

    #[test]
    fn parse_expr_parses_unary_negation() -> ExprResult<()> {
        // Given: the expression "-5"
        // When: parse_expr is called
        // Then: the AST is Unary(Neg, Literal(I64(5)))
        let expr = parse("-5")?;
        let (op, inner) = as_unary(&expr)?;
        assert_eq!(op, UnaryOp::Neg);
        assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(5)));
        Ok(())
    }

    #[test]
    fn parse_expr_parses_boolean_not() -> ExprResult<()> {
        // Given: the expression "not true"
        // When: parse_expr is called
        // Then: the AST is Unary(Not, Literal(Bool(true)))
        let expr = parse("not true")?;
        let (op, inner) = as_unary(&expr)?;
        assert_eq!(op, UnaryOp::Not);
        assert_eq!(*inner, ExprAst::Literal(ExprLiteral::Bool(true)));
        Ok(())
    }

    #[test]
    fn parse_expr_parses_comparison_operators() -> ExprResult<()> {
        // Given: the expression "5 == 5"
        // When: parse_expr is called
        // Then: the AST is Binary(Eq, Literal(I64(5)), Literal(I64(5)))
        let expr = parse("5 == 5")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Eq);
        assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(5)));

        let expr_ne = parse("5 != 3")?;
        let (op_ne, _, _) = as_binary(&expr_ne)?;
        assert_eq!(op_ne, BinaryOp::NotEq);

        let expr_lt = parse("1 < 2")?;
        let (op_lt, _, _) = as_binary(&expr_lt)?;
        assert_eq!(op_lt, BinaryOp::Lt);

        let expr_gt = parse("2 > 1")?;
        let (op_gt, _, _) = as_binary(&expr_gt)?;
        assert_eq!(op_gt, BinaryOp::Gt);

        let expr_lte = parse("1 <= 2")?;
        let (op_lte, _, _) = as_binary(&expr_lte)?;
        assert_eq!(op_lte, BinaryOp::Lte);

        let expr_gte = parse("2 >= 1")?;
        let (op_gte, _, _) = as_binary(&expr_gte)?;
        assert_eq!(op_gte, BinaryOp::Gte);
        Ok(())
    }

    #[test]
    fn parse_expr_parses_logical_and_or() -> ExprResult<()> {
        // Given: the expression "true and false or true"
        // When: parse_expr is called
        // Then: 'or' is the top-level op and 'and' binds tighter
        let expr = parse("true and false or true")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Or);
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::Bool(true)));
        let (left_op, _, _) = as_binary(left)?;
        assert_eq!(left_op, BinaryOp::And);
        Ok(())
    }

    #[test]
    fn parse_expr_parses_helper_call_with_arguments() -> ExprResult<()> {
        // Given: the expression "contains($x, $y)"
        // When: parse_expr is called
        // Then: the AST is Helper(Contains, [Reference("$x"), Reference("$y")])
        let expr = parse("contains($x, $y)")?;
        let (name, args) = as_helper(&expr)?;
        assert_eq!(name, ExprHelper::Contains);
        assert_eq!(args.len(), 2);
        assert_eq!(args.first(), Some(&ExprAst::Reference(Box::from("$x"))));
        assert_eq!(args.get(1), Some(&ExprAst::Reference(Box::from("$y"))));
        Ok(())
    }

    #[test]
    fn parse_expr_parses_variable_reference() -> ExprResult<()> {
        // Given: the expression "$data.field"
        // When: parse_expr is called
        // Then: the AST is Reference("$data.field")
        let expr = parse("$data.field")?;
        assert_eq!(expr, ExprAst::Reference(Box::from("$data.field")));
        Ok(())
    }

    #[test]
    fn parse_expr_returns_error_for_empty_input() -> ExprResult<()> {
        // Given: an empty token stream (only End)
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken { token }) containing "End"
        let tokens = crate::lexer::lex_expr("")?;
        let result = parse_expr(&tokens);
        let Err(ExprError::UnexpectedToken { token }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedToken".into(),
            });
        };
        assert!(
            token.contains("End"),
            "token should contain 'End', got: {token}"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_returns_unknown_helper_for_bad_helper() -> ExprResult<()> {
        // Given: the expression "bogus_func(1)"
        // When: parse_expr is called
        // Then: the result is Err(UnknownHelper { helper: "bogus_func" })
        let result = parse("bogus_func(1)");
        let Err(ExprError::UnknownHelper { helper }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnknownHelper".into(),
            });
        };
        assert_eq!(helper, "bogus_func");
        Ok(())
    }

    #[test]
    fn parse_expr_returns_wrong_arity_error_for_contains_with_one_arg() -> ExprResult<()> {
        // Given: the expression "contains(1)" (wrong arity)
        // When: parse_expr is called
        // Then: the result is Err(HelperArityMismatch { helper: "contains", expected: 2, actual: 1 })
        let result = parse("contains(1)");
        let Err(ExprError::HelperArityMismatch {
            helper,
            expected,
            actual,
        }) = result
        else {
            return Err(ExprError::UnexpectedToken {
                token: "expected HelperArityMismatch".into(),
            });
        };
        assert_eq!(helper, "contains");
        assert_eq!(expected, 2);
        assert_eq!(actual, 1);
        Ok(())
    }

    #[test]
    fn parse_expr_returns_error_for_missing_right_paren() -> ExprResult<()> {
        // Given: the expression "(1 + 2"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken { token }) where token mentions right parenthesis
        let result = parse("(1 + 2");
        let Err(ExprError::UnexpectedToken { token }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedToken".into(),
            });
        };
        assert!(
            token.contains("right parenthesis"),
            "token should mention right parenthesis, got: {token}"
        );
        Ok(())
    }

    fn as_binary(expr: &ExprAst) -> ExprResult<(BinaryOp, &ExprAst, &ExprAst)> {
        match expr {
            ExprAst::Binary { op, left, right } => Ok((*op, left, right)),
            other => Err(ExprError::UnexpectedToken {
                token: format!("expected binary, got {other:?}"),
            }),
        }
    }

    fn as_unary(expr: &ExprAst) -> ExprResult<(UnaryOp, &ExprAst)> {
        match expr {
            ExprAst::Unary { op, expr } => Ok((*op, expr)),
            other => Err(ExprError::UnexpectedToken {
                token: format!("expected unary, got {other:?}"),
            }),
        }
    }

    fn as_helper(expr: &ExprAst) -> ExprResult<(ExprHelper, &[ExprAst])> {
        match expr {
            ExprAst::Helper { name, args } => Ok((*name, args)),
            other => Err(ExprError::UnexpectedToken {
                token: format!("expected helper, got {other:?}"),
            }),
        }
    }

    // --- Adversarial BDD parser tests ---

    #[test]
    fn parse_expr_chained_unary_not_true() -> ExprResult<()> {
        // Given: the expression "not not not not true"
        // When: parse_expr is called
        // Then: the result is a nested chain of 4 Unary(Not) operators around Bool(true)
        let expr = parse("not not not not true")?;
        // Unwrap 4 layers of Not
        let (op1, inner1) = as_unary(&expr)?;
        assert_eq!(op1, UnaryOp::Not);
        let (op2, inner2) = as_unary(inner1)?;
        assert_eq!(op2, UnaryOp::Not);
        let (op3, inner3) = as_unary(inner2)?;
        assert_eq!(op3, UnaryOp::Not);
        let (op4, inner4) = as_unary(inner3)?;
        assert_eq!(op4, UnaryOp::Not);
        assert_eq!(*inner4, ExprAst::Literal(ExprLiteral::Bool(true)));
        Ok(())
    }

    #[test]
    fn parse_expr_double_negation_parses_correctly() -> ExprResult<()> {
        // Given: the expression "--5"
        // When: parse_expr is called
        // Then: the result is Unary(Neg, Unary(Neg, Literal(I64(5))))
        let expr = parse("--5")?;
        let (op1, inner1) = as_unary(&expr)?;
        assert_eq!(op1, UnaryOp::Neg);
        let (op2, inner2) = as_unary(inner1)?;
        assert_eq!(op2, UnaryOp::Neg);
        assert_eq!(*inner2, ExprAst::Literal(ExprLiteral::I64(5)));
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_trailing_operator() -> ExprResult<()> {
        // Given: the expression "1 +"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken) because there is no right operand
        let result = parse("1 +");
        assert!(
            matches!(result, Err(ExprError::UnexpectedToken { .. })),
            "trailing operator should produce UnexpectedToken"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_double_operator() -> ExprResult<()> {
        // Given: the expression "1 + * 2"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken) because * is not a valid prefix
        let result = parse("1 + * 2");
        assert!(
            matches!(result, Err(ExprError::UnexpectedToken { .. })),
            "double operator should produce UnexpectedToken"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_deeply_nested_parentheses_within_limit() -> ExprResult<()> {
        // Given: the expression "(((((((1 + 2)))))))" (7 levels of nesting)
        // When: parse_expr is called
        // Then: the result is Binary(Add, I64(1), I64(2))
        let expr = parse("(((((((1 + 2)))))))")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(2)));
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_empty_parentheses() -> ExprResult<()> {
        // Given: the expression "()"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken) because empty parens have no expression
        let result = parse("()");
        assert!(
            matches!(result, Err(ExprError::UnexpectedToken { .. })),
            "empty parentheses should produce UnexpectedToken"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_extra_right_paren() -> ExprResult<()> {
        // Given: the expression "1)"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken) because the trailing ) is unexpected
        let result = parse("1)");
        assert!(
            matches!(result, Err(ExprError::UnexpectedToken { .. })),
            "trailing right paren should produce UnexpectedToken"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_unknown_identifier_without_paren() -> ExprResult<()> {
        // Given: the expression "foo"
        // When: parse_expr is called
        // Then: the result is Err(UnexpectedToken) mentioning "unknown identifier: foo"
        let result = parse("foo");
        let Err(ExprError::UnexpectedToken { token }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedToken".into(),
            });
        };
        assert!(
            token.contains("unknown identifier"),
            "token should mention unknown identifier, got: {token}"
        );
        Ok(())
    }

    #[test]
    fn parse_expr_null_equality_parses_as_binary_eq() -> ExprResult<()> {
        // Given: the expression "null == null"
        // When: parse_expr is called
        // Then: the AST is Binary(Eq, Literal(Null), Literal(Null))
        let expr = parse("null == null")?;
        let (op, left, right) = as_binary(&expr)?;
        assert_eq!(op, BinaryOp::Eq);
        assert_eq!(*left, ExprAst::Literal(ExprLiteral::Null));
        assert_eq!(*right, ExprAst::Literal(ExprLiteral::Null));
        Ok(())
    }

    #[test]
    fn parse_expr_rejects_helper_with_too_many_args() -> ExprResult<()> {
        // Given: the expression "contains(1, 2, 3, 4, 5, 6, 7, 8, 9)"
        // When: parse_expr is called
        // Then: the result is Err(TooManyHelperArgs) because 9 exceeds MAX_HELPER_ARGS
        let result = parse("contains(1, 2, 3, 4, 5, 6, 7, 8, 9)");
        assert!(
            matches!(result, Err(ExprError::TooManyHelperArgs { len: 9, max: 8 })),
            "9 helper args should exceed the 8-arg limit"
        );
        Ok(())
    }
}
