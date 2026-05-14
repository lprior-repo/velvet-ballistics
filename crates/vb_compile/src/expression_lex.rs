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
            self.lex_integer()
        } else if is_ident_start(ch) {
            self.lex_ident()
        } else {
            self.lex_symbol(ch)
        }
    }

    fn lex_integer(&mut self) -> Result<(), CompileError> {
        let start = self.index;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump_current();
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

