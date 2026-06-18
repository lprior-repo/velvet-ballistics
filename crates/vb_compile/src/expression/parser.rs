#![forbid(unsafe_code)]
//! Recursive-descent parser for the cold expression grammar.

use crate::CompileError;
use crate::expression::domain::{BinaryOp, ExpressionLiteral, ParsedExpression, UnaryOp};
use crate::expression::helpers::parse_helper;
use crate::expression::lexer::{self, Token, TokenKind};

// ── Public entry point ───────────────────────────────────────────────────────

/// Parses one source expression string into the cold expression tree.
pub fn parse_expression(source: &str) -> Result<ParsedExpression, CompileError> {
    let tokens = lexer::lex(source)?;
    let mut parser = Parser::new(source, tokens);
    parser.parse_complete()
}

// ── Parser ───────────────────────────────────────────────────────────────────

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            index: 0,
        }
    }

    fn parse_complete(&mut self) -> Result<ParsedExpression, CompileError> {
        let expr = self.parse_precedence(0, 0)?;
        match self.current_kind() {
            TokenKind::End => Ok(expr),
            _ => Err(self.unexpected_token("end of expression")),
        }
    }

    fn parse_precedence(
        &mut self,
        min_bp: u8,
        depth: u8,
    ) -> Result<ParsedExpression, CompileError> {
        self.check_depth(depth)?;
        let mut left = self.parse_prefix(depth)?;
        while let TokenKind::Operator(op) = self.current_kind() {
            let (left_bp, right_bp) = infix_binding_power(op);
            if left_bp < min_bp {
                break;
            }
            self.advance();
            let right = self.parse_precedence(right_bp, depth.saturating_add(1))?;
            left = ParsedExpression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self, depth: u8) -> Result<ParsedExpression, CompileError> {
        match self.current_kind() {
            TokenKind::Integer(value) => self.literal(ExpressionLiteral::I64(value)),
            TokenKind::Float(value) => {
                let finite = vb_core::FiniteF64::new(value).map_err(|_| {
                    CompileError::ExpressionFloatOutOfRange {
                        expression: Box::<str>::from(self.source),
                        index: self.current_index(),
                    }
                })?;
                self.literal(ExpressionLiteral::F64(finite))
            }
            TokenKind::String(value) => self.literal(ExpressionLiteral::Text(value)),
            TokenKind::Reference(value) => self.reference(value),
            TokenKind::Ident(value) => self.ident(value, depth),
            TokenKind::Unary(op) => self.unary(op, depth),
            TokenKind::Operator(BinaryOp::Sub) => self.unary(UnaryOp::Neg, depth),
            TokenKind::LeftParen => self.parenthesized(depth),
            _ => Err(self.unexpected_token("expression")),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn literal(&mut self, value: ExpressionLiteral) -> Result<ParsedExpression, CompileError> {
        self.advance();
        Ok(ParsedExpression::Literal(value))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn reference(&mut self, value: Box<str>) -> Result<ParsedExpression, CompileError> {
        self.advance();
        Ok(ParsedExpression::Reference(value))
    }

    fn ident(&mut self, value: Box<str>, depth: u8) -> Result<ParsedExpression, CompileError> {
        match value.as_ref() {
            "true" => self.literal(ExpressionLiteral::Bool(true)),
            "false" => self.literal(ExpressionLiteral::Bool(false)),
            "null" => self.literal(ExpressionLiteral::Null),
            name if self.next_kind() == TokenKind::LeftParen => self.helper_call(name, depth),
            _ => Err(self.unexpected_ident(value)),
        }
    }

    fn unary(&mut self, op: UnaryOp, depth: u8) -> Result<ParsedExpression, CompileError> {
        self.advance();
        Ok(ParsedExpression::Unary {
            op,
            expr: Box::new(self.parse_precedence(11, depth.saturating_add(1))?),
        })
    }

    fn parenthesized(&mut self, depth: u8) -> Result<ParsedExpression, CompileError> {
        self.advance();
        let expr = self.parse_precedence(0, depth.saturating_add(1))?;
        if self.current_kind() != TokenKind::RightParen {
            return Err(self.unexpected_token("right parenthesis"));
        }
        self.advance();
        Ok(expr)
    }

    fn helper_call(&mut self, name: &str, depth: u8) -> Result<ParsedExpression, CompileError> {
        let helper = parse_helper(name).ok_or_else(|| self.unknown_identifier(name))?;
        self.advance();
        self.advance();
        let args = self.parse_helper_args(depth)?;
        Ok(ParsedExpression::HelperCall { name: helper, args })
    }

    fn parse_helper_args(&mut self, depth: u8) -> Result<Box<[ParsedExpression]>, CompileError> {
        let mut args = Vec::with_capacity(2);
        if self.current_kind() == TokenKind::RightParen {
            self.advance();
            return Ok(args.into_boxed_slice());
        }
        self.parse_one_or_more_args(&mut args, depth)
    }

    fn parse_one_or_more_args(
        &mut self,
        args: &mut Vec<ParsedExpression>,
        depth: u8,
    ) -> Result<Box<[ParsedExpression]>, CompileError> {
        loop {
            if args.len() >= lexer::MAX_HELPER_ARGS {
                return Err(self.limit_error("helper args", lexer::MAX_HELPER_ARGS));
            }
            args.push(self.parse_precedence(0, depth.saturating_add(1))?);
            match self.current_kind() {
                TokenKind::Comma => self.advance(),
                TokenKind::RightParen => return self.close_args(args),
                _ => return Err(self.unexpected_token("comma or right parenthesis")),
            }
        }
    }

    fn current_kind(&self) -> TokenKind {
        match self.tokens.get(self.index) {
            Some(token) => token.kind(),
            None => TokenKind::End,
        }
    }

    fn next_kind(&self) -> TokenKind {
        match self.tokens.get(self.index.saturating_add(1)) {
            Some(token) => token.kind(),
            None => TokenKind::End,
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn close_args(
        &mut self,
        args: &mut Vec<ParsedExpression>,
    ) -> Result<Box<[ParsedExpression]>, CompileError> {
        self.advance();
        Ok(std::mem::take(args).into_boxed_slice())
    }

    fn check_depth(&self, depth: u8) -> Result<(), CompileError> {
        if depth > lexer::MAX_EXPRESSION_DEPTH {
            Err(self.limit_error("parse depth", lexer::MAX_EXPRESSION_DEPTH_USIZE))
        } else {
            Ok(())
        }
    }

    fn current_index(&self) -> usize {
        self.tokens
            .get(self.index)
            .map_or(self.source.len(), |token| token.index())
    }

    fn advance(&mut self) {
        self.index = self.index.saturating_add(1);
    }

    fn unexpected_token(&self, expected: &'static str) -> CompileError {
        CompileError::ExpressionUnexpectedToken {
            expression: Box::<str>::from(self.source),
            index: self.current_index(),
            expected,
        }
    }

    fn unexpected_ident(&self, value: Box<str>) -> CompileError {
        CompileError::ExpressionUnknownIdentifier {
            expression: Box::<str>::from(self.source),
            index: self.current_index(),
            identifier: value,
        }
    }

    fn unknown_identifier(&self, value: &str) -> CompileError {
        CompileError::ExpressionUnknownIdentifier {
            expression: Box::<str>::from(self.source),
            index: self.current_index(),
            identifier: Box::<str>::from(value),
        }
    }

    fn limit_error(&self, limit: &'static str, max: usize) -> CompileError {
        lexer::limit_error(self.source, limit, max)
    }
}

/// Returns the left and right binding power for a left-associative infix operator.
pub(super) fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (3, 4),
        BinaryOp::Eq | BinaryOp::NotEq => (5, 6),
        BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => (7, 8),
        BinaryOp::Add | BinaryOp::Sub => (9, 10),
        BinaryOp::Mul | BinaryOp::Div => (11, 12),
    }
}
