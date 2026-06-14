#![forbid(unsafe_code)]
//! Cold expression lexer/parser used by the compiler AST boundary.

use crate::CompileError;
use vb_core::FiniteF64;

/// Parsed v1 expression tree retained by the cold compiler AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParsedExpression {
    /// Null, boolean, integer, or string literal.
    Literal(ExpressionLiteral),
    /// Source reference beginning with `$`.
    Reference(Box<str>),
    /// Prefix unary expression.
    Unary {
        /// Unary operator.
        op: UnaryOp,
        /// Operand expression.
        expr: Box<ParsedExpression>,
    },
    /// Infix binary expression.
    Binary {
        /// Binary operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<ParsedExpression>,
        /// Right operand.
        right: Box<ParsedExpression>,
    },
    /// Built-in helper call retained for later bytecode lowering.
    HelperCall {
        /// Helper name.
        name: ExpressionHelper,
        /// Parsed helper arguments.
        args: Box<[ParsedExpression]>,
    },
}

/// Built-in helper accepted by the v1 expression grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpressionHelper {
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
    /// `coalesce(value, fallback)`.
    Coalesce,
}

/// Literal value accepted by the expression grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpressionLiteral {
    /// Null literal.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// IEEE-754 double-precision literal.
    F64(FiniteF64),
    /// Double-quoted string literal.
    Text(Box<str>),
}

/// Prefix unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnaryOp {
    /// Logical negation.
    Not,
    /// Numeric negation.
    Neg,
}

/// Left-associative infix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BinaryOp {
    /// Logical OR.
    Or,
    /// Logical AND.
    And,
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    NotEq,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
}

const MAX_EXPRESSION_SOURCE_BYTES: usize = 4096;
const MAX_EXPRESSION_TOKENS: usize = 256;
const MAX_EXPRESSION_DEPTH: u8 = 64;
const MAX_EXPRESSION_DEPTH_USIZE: usize = 64;
const MAX_HELPER_ARGS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Integer(i64),
    Float(f64),
    String(Box<str>),
    Reference(Box<str>),
    Ident(Box<str>),
    Operator(BinaryOp),
    Unary(UnaryOp),
    LeftParen,
    RightParen,
    Comma,
    End,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    index: usize,
}

/// Parses one source expression string into the cold expression tree.
pub fn parse_expression(source: &str) -> Result<ParsedExpression, CompileError> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(source, tokens);
    parser.parse_complete()
}

fn lex(source: &str) -> Result<Vec<Token>, CompileError> {
    if source.len() > MAX_EXPRESSION_SOURCE_BYTES {
        return Err(limit_error(
            source,
            "source length",
            MAX_EXPRESSION_SOURCE_BYTES,
        ));
    }
    let mut lexer = Lexer::new(source);
    lexer.lex_all()
}

struct Lexer<'a> {
    source: &'a str,
    index: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            tokens: Vec::new(),
        }
    }

    fn lex_all(&mut self) -> Result<Vec<Token>, CompileError> {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.bump_char(ch);
            } else {
                self.lex_one(ch)?;
            }
        }
        self.push(TokenKind::End, self.index)?;
        Ok(std::mem::take(&mut self.tokens))
    }

    fn lex_one(&mut self, ch: char) -> Result<(), CompileError> {
        if ch.is_ascii_digit() {
            self.lex_integer_or_float()
        } else if is_ident_start(ch) {
            self.lex_ident()
        } else {
            self.lex_symbol(ch)
        }
    }

    fn lex_integer_or_float(&mut self) -> Result<(), CompileError> {
        let start = self.index;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump_current();
        }
        let has_dot = self.peek_char().is_some_and(|ch| ch == '.');
        if has_dot {
            self.bump_current();
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.bump_current();
            }
            let text = self.slice(start, self.index)?;
            let value =
                text.parse::<f64>()
                    .map_err(|_| CompileError::ExpressionFloatOutOfRange {
                        expression: Box::<str>::from(self.source),
                        index: start,
                    })?;
            return self.push(TokenKind::Float(value), start);
        }
        let text = self.slice(start, self.index)?;
        let value = text
            .parse::<i64>()
            .map_err(|_| CompileError::ExpressionIntegerOutOfRange {
                expression: Box::<str>::from(self.source),
                index: start,
            })?;
        self.push(TokenKind::Integer(value), start)
    }

    fn lex_ident(&mut self) -> Result<(), CompileError> {
        let start = self.index;
        while self.peek_char().is_some_and(is_ident_continue) {
            self.bump_current();
        }
        let text = self.slice(start, self.index)?;
        let kind = match text {
            "and" => TokenKind::Operator(BinaryOp::And),
            "or" => TokenKind::Operator(BinaryOp::Or),
            "not" => TokenKind::Unary(UnaryOp::Not),
            _ => TokenKind::Ident(Box::<str>::from(text)),
        };
        self.push(kind, start)
    }

    fn lex_symbol(&mut self, ch: char) -> Result<(), CompileError> {
        match ch {
            '$' => self.lex_reference(),
            '"' => self.lex_string(),
            '(' => self.single(TokenKind::LeftParen, ch),
            ')' => self.single(TokenKind::RightParen, ch),
            ',' => self.single(TokenKind::Comma, ch),
            '!' | '=' | '<' | '>' => self.lex_compound_operator(ch),
            '+' | '-' | '*' | '/' => self.lex_arithmetic_operator(ch),
            _ => Err(self.unexpected_char(ch)),
        }
    }

    fn lex_reference(&mut self) -> Result<(), CompileError> {
        let start = self.index;
        self.bump_current();
        let body_start = self.index;
        while self.peek_char().is_some_and(is_reference_continue) {
            self.bump_current();
        }
        if self.index == body_start {
            return Err(self.unexpected_char('$'));
        }
        let reference = self.slice(start, self.index)?;
        self.push(TokenKind::Reference(Box::<str>::from(reference)), start)
    }

    fn lex_string(&mut self) -> Result<(), CompileError> {
        let start = self.index;
        self.bump_current();
        let value_start = self.index;
        while let Some(ch) = self.peek_char() {
            if ch == '"' {
                let value = Box::<str>::from(self.slice(value_start, self.index)?);
                self.bump_current();
                return self.push(TokenKind::String(value), start);
            }
            self.bump_char(ch);
        }
        Err(CompileError::ExpressionUnterminatedString {
            expression: Box::<str>::from(self.source),
            index: start,
        })
    }

    fn lex_compound_operator(&mut self, ch: char) -> Result<(), CompileError> {
        let start = self.index;
        self.bump_current();
        let next = self.peek_char();
        match (ch, next) {
            ('!', Some('=')) => self.second(TokenKind::Operator(BinaryOp::NotEq), start),
            ('=', Some('=')) => self.second(TokenKind::Operator(BinaryOp::Eq), start),
            ('<', Some('=')) => self.second(TokenKind::Operator(BinaryOp::Lte), start),
            ('>', Some('=')) => self.second(TokenKind::Operator(BinaryOp::Gte), start),
            ('<', _) => self.push(TokenKind::Operator(BinaryOp::Lt), start),
            ('>', _) => self.push(TokenKind::Operator(BinaryOp::Gt), start),
            _ => Err(self.unexpected_char_at(ch, start)),
        }
    }

    fn lex_arithmetic_operator(&mut self, ch: char) -> Result<(), CompileError> {
        let start = self.index;
        self.bump_current();
        let kind = match ch {
            '+' => TokenKind::Operator(BinaryOp::Add),
            '-' => TokenKind::Operator(BinaryOp::Sub),
            '*' => TokenKind::Operator(BinaryOp::Mul),
            '/' => TokenKind::Operator(BinaryOp::Div),
            _ => return Err(self.unexpected_char_at(ch, start)),
        };
        self.push(kind, start)
    }

    fn second(&mut self, kind: TokenKind, start: usize) -> Result<(), CompileError> {
        self.bump_current();
        self.push(kind, start)
    }

    fn single(&mut self, kind: TokenKind, ch: char) -> Result<(), CompileError> {
        let start = self.index;
        self.bump_char(ch);
        self.push(kind, start)
    }

    fn push(&mut self, kind: TokenKind, index: usize) -> Result<(), CompileError> {
        if self.tokens.len() >= MAX_EXPRESSION_TOKENS {
            return Err(limit_error(
                self.source,
                "token count",
                MAX_EXPRESSION_TOKENS,
            ));
        }
        self.tokens.push(Token { kind, index });
        Ok(())
    }

    fn peek_char(&self) -> Option<char> {
        self.source
            .get(self.index..)
            .and_then(|tail| tail.chars().next())
    }

    fn bump_current(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.bump_char(ch);
        }
    }

    fn bump_char(&mut self, ch: char) {
        self.index = self.index.saturating_add(ch.len_utf8());
    }

    fn unexpected_char(&self, ch: char) -> CompileError {
        self.unexpected_char_at(ch, self.index)
    }

    fn unexpected_char_at(&self, ch: char, index: usize) -> CompileError {
        CompileError::ExpressionUnexpectedChar {
            expression: Box::<str>::from(self.source),
            index,
            found: ch,
        }
    }

    fn slice(&self, start: usize, end: usize) -> Result<&str, CompileError> {
        self.source
            .get(start..end)
            .ok_or(CompileError::ExpressionUnexpectedToken {
                expression: Box::<str>::from(self.source),
                index: start,
                expected: "valid UTF-8 token boundary",
            })
    }
}

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
                let finite =
                    FiniteF64::new(value).map_err(|_| CompileError::ExpressionFloatOutOfRange {
                        expression: Box::<str>::from(self.source),
                        index: self.current_index(),
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
            if args.len() >= MAX_HELPER_ARGS {
                return Err(limit_error(self.source, "helper args", MAX_HELPER_ARGS));
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
            Some(token) => token.kind.clone(),
            None => TokenKind::End,
        }
    }

    fn next_kind(&self) -> TokenKind {
        match self.tokens.get(self.index.saturating_add(1)) {
            Some(token) => token.kind.clone(),
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
        if depth > MAX_EXPRESSION_DEPTH {
            Err(limit_error(
                self.source,
                "parse depth",
                MAX_EXPRESSION_DEPTH_USIZE,
            ))
        } else {
            Ok(())
        }
    }

    fn current_index(&self) -> usize {
        self.tokens
            .get(self.index)
            .map_or(self.source.len(), |token| token.index)
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
}

fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (3, 4),
        BinaryOp::Eq | BinaryOp::NotEq => (5, 6),
        BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => (7, 8),
        BinaryOp::Add | BinaryOp::Sub => (9, 10),
        BinaryOp::Mul | BinaryOp::Div => (11, 12),
    }
}

pub(crate) fn parse_helper(name: &str) -> Option<ExpressionHelper> {
    match name {
        "contains" => Some(ExpressionHelper::Contains),
        "starts_with" => Some(ExpressionHelper::StartsWith),
        "ends_with" => Some(ExpressionHelper::EndsWith),
        "has" => Some(ExpressionHelper::Has),
        "exists" => Some(ExpressionHelper::Exists),
        "length" => Some(ExpressionHelper::Length),
        "empty" => Some(ExpressionHelper::Empty),
        "append" => Some(ExpressionHelper::Append),
        "append_if" => Some(ExpressionHelper::AppendIf),
        "merge" => Some(ExpressionHelper::Merge),
        "sum" => Some(ExpressionHelper::Sum),
        "count" => Some(ExpressionHelper::Count),
        "unique" => Some(ExpressionHelper::Unique),
        "coalesce" => Some(ExpressionHelper::Coalesce),
        _ => None,
    }
}

fn limit_error(source: &str, limit: &'static str, max: usize) -> CompileError {
    CompileError::ExpressionLimitExceeded {
        expression: Box::<str>::from(source),
        limit,
        max,
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_reference_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

#[cfg(test)]
#[path = "expression/tests.rs"]
mod tests;
