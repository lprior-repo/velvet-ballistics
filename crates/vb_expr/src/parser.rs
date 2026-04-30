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
}
