#![forbid(unsafe_code)]
//! Expression parser producing a typed AST from token streams.

use crate::lexer::{BinaryOp, LiteralToken, Token, UnaryOp, infix_binding_power};
use crate::{ExprError, ExprResult};

pub use types::{ExprAst, ExprHelper, ExprLiteral};

pub mod miri_tests;
pub mod tests;
pub mod types;

/// Maximum nesting depth for the parser.
const MAX_DEPTH: u8 = 64;
/// Maximum helper call arguments.
const MAX_HELPER_ARGS: usize = 8;

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
        LiteralToken::F64(v) => ExprAst::Literal(ExprLiteral::F64(v)),
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
