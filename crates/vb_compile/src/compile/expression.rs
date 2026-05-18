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
mod tests {
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
}
