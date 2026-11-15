use super::limits::{MAX_DEPTH, MAX_INPUT_BYTES};
use super::{DecodeError, DecodeLimits, IntegerField};

#[derive(Clone, Copy)]
enum State {
    ObjectKey { allow_end: bool },
    ObjectColon { selected: bool },
    ObjectValue { selected: bool },
    ObjectAfterValue,
    ArrayValue { allow_end: bool },
    ArrayAfterValue,
}

pub(super) struct Parser<'input, 'field> {
    pub(super) input: &'input [u8],
    pub(super) field: &'field str,
    pub(super) position: usize,
    pub(super) limits: DecodeLimits,
    nodes: usize,
    stack: [State; MAX_DEPTH],
    depth: usize,
    selected: IntegerField,
}

impl<'input, 'field> Parser<'input, 'field> {
    pub(super) fn new(input: &'input [u8], limits: DecodeLimits, field: &'field str) -> Self {
        Self {
            input,
            field,
            position: 0,
            limits,
            nodes: 0,
            stack: [State::ObjectAfterValue; MAX_DEPTH],
            depth: 0,
            selected: IntegerField::Missing,
        }
    }

    pub(super) fn run(mut self) -> Result<IntegerField, DecodeError> {
        self.whitespace()?;
        self.expect_byte(b'{')?;
        self.node()?;
        self.push(State::ObjectKey { allow_end: true })?;
        // Every successful nonterminal step consumes at least one input byte.
        for _ in 0..=MAX_INPUT_BYTES {
            self.whitespace()?;
            if self.depth == 0 {
                return if self.position == self.input.len() {
                    Ok(self.selected)
                } else {
                    Err(DecodeError::InvalidJson)
                };
            }
            let state = self.pop()?;
            self.step(state)?;
        }
        Err(DecodeError::InvalidJson)
    }

    fn step(&mut self, state: State) -> Result<(), DecodeError> {
        match state {
            State::ObjectKey { allow_end } => self.object_key(allow_end),
            State::ObjectColon { selected } => {
                self.expect_byte(b':')?;
                self.push(State::ObjectValue { selected })
            }
            State::ObjectValue { selected } => {
                self.push(State::ObjectAfterValue)?;
                self.value(selected)
            }
            State::ObjectAfterValue => self.after_value(true),
            State::ArrayValue { allow_end } => {
                if allow_end && self.eat_byte(b']')? {
                    return Ok(());
                }
                self.push(State::ArrayAfterValue)?;
                self.value(false)
            }
            State::ArrayAfterValue => self.after_value(false),
        }
    }

    fn object_key(&mut self, allow_end: bool) -> Result<(), DecodeError> {
        if allow_end && self.eat_byte(b'}')? {
            return Ok(());
        }
        self.node()?;
        let matches = self.string()?;
        self.push(State::ObjectColon {
            selected: self.depth == 0 && matches,
        })
    }

    fn after_value(&mut self, object: bool) -> Result<(), DecodeError> {
        let end = if object { b'}' } else { b']' };
        if self.eat_byte(end)? {
            return Ok(());
        }
        self.expect_byte(b',')?;
        let next = if object {
            State::ObjectKey { allow_end: false }
        } else {
            State::ArrayValue { allow_end: false }
        };
        self.push(next)
    }

    fn value(&mut self, selected: bool) -> Result<(), DecodeError> {
        self.node()?;
        let value = match self.peek_byte() {
            Some(b'{') => {
                self.take_byte()?;
                self.push(State::ObjectKey { allow_end: true })?;
                IntegerField::InvalidType
            }
            Some(b'[') => {
                self.take_byte()?;
                self.push(State::ArrayValue { allow_end: true })?;
                IntegerField::InvalidType
            }
            Some(b'"') => {
                let _matches = self.string()?;
                IntegerField::InvalidType
            }
            Some(b't') => {
                self.literal(b"true")?;
                IntegerField::InvalidType
            }
            Some(b'f') => {
                self.literal(b"false")?;
                IntegerField::InvalidType
            }
            Some(b'n') => {
                self.literal(b"null")?;
                IntegerField::InvalidType
            }
            Some(b'-' | b'0'..=b'9') => self.number()?,
            _ => return Err(DecodeError::InvalidJson),
        };
        if selected {
            self.selected = match self.selected {
                IntegerField::Missing => value,
                _ => IntegerField::Duplicate,
            };
        }
        Ok(())
    }

    fn literal(&mut self, expected: &[u8]) -> Result<(), DecodeError> {
        if expected.len() > 5 {
            return Err(DecodeError::InvalidJson);
        }
        for byte in expected.iter().take(5) {
            self.expect_byte(*byte)?;
        }
        Ok(())
    }

    fn push(&mut self, state: State) -> Result<(), DecodeError> {
        if self.depth >= self.limits.depth {
            return Err(DecodeError::DepthLimit);
        }
        let slot = self.stack.get_mut(self.depth).ok_or(DecodeError::DepthLimit)?;
        *slot = state;
        self.depth = self.depth.checked_add(1).ok_or(DecodeError::DepthLimit)?;
        Ok(())
    }

    fn pop(&mut self) -> Result<State, DecodeError> {
        let index = self.depth.checked_sub(1).ok_or(DecodeError::InvalidJson)?;
        let state = self.stack.get(index).copied().ok_or(DecodeError::InvalidJson)?;
        self.depth = index;
        Ok(state)
    }

    pub(super) fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    pub(super) fn take_byte(&mut self) -> Result<u8, DecodeError> {
        let value = self.peek_byte().ok_or(DecodeError::InvalidJson)?;
        self.position = self.position.checked_add(1).ok_or(DecodeError::InvalidJson)?;
        Ok(value)
    }

    pub(super) fn eat_byte(&mut self, expected: u8) -> Result<bool, DecodeError> {
        if self.peek_byte() != Some(expected) {
            return Ok(false);
        }
        self.take_byte()?;
        Ok(true)
    }

    pub(super) fn expect_byte(&mut self, expected: u8) -> Result<(), DecodeError> {
        if self.take_byte()? == expected {
            Ok(())
        } else {
            Err(DecodeError::InvalidJson)
        }
    }

    fn whitespace(&mut self) -> Result<(), DecodeError> {
        for _ in 0..MAX_INPUT_BYTES {
            if !matches!(self.peek_byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                return Ok(());
            }
            self.take_byte()?;
        }
        Ok(())
    }

    fn node(&mut self) -> Result<(), DecodeError> {
        self.nodes = self.nodes.checked_add(1).ok_or(DecodeError::NodeLimit)?;
        if self.nodes > self.limits.nodes {
            Err(DecodeError::NodeLimit)
        } else {
            Ok(())
        }
    }
}
