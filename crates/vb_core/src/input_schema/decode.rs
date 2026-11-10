//! Bound recursive binary decoding before an untrusted tree can reach validation.
use super::{CompiledInputField, InputDefaultValue, MAX_INPUT_SCHEMA_DEPTH};
use crate::{CompiledInputConstraint, InputSlotKind, SlotValue};
use serde::{Deserialize, Deserializer, de::Error};
use std::cell::Cell;

thread_local! {
    static SCHEMA_DEPTH: Cell<usize> = const { Cell::new(0) };
    static VALUE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct DepthGuard<'a> {
    depth: &'a Cell<usize>,
    previous: usize,
}
impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.depth.set(self.previous);
    }
}

fn bounded<T, E: Error>(
    depth: &Cell<usize>,
    decode: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let previous = depth.get();
    if previous > MAX_INPUT_SCHEMA_DEPTH {
        return Err(E::custom("input schema decode depth exceeded"));
    }
    depth.set(
        previous
            .checked_add(1)
            .ok_or_else(|| E::custom("input schema decode depth overflow"))?,
    );
    let _guard = DepthGuard { depth, previous };
    decode()
}

// Field and variant ordering intentionally match the versioned production wire.
#[derive(Deserialize)]
struct ConstraintWire {
    optional: bool,
    nullable: bool,
    default_value: Option<SlotValue>,
    min: Option<i64>,
    max: Option<i64>,
    min_length: Option<i64>,
    max_length: Option<i64>,
    element: Option<InputSlotKind>,
    allow_extra: Option<bool>,
    secret: bool,
    #[serde(default)]
    fields: Option<Box<[CompiledInputField]>>,
    #[serde(default)]
    default_cold: Option<InputDefaultValue>,
}

impl<'de> Deserialize<'de> for CompiledInputConstraint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SCHEMA_DEPTH.with(|depth| {
            bounded(depth, || {
                let wire = ConstraintWire::deserialize(deserializer)?;
                Ok(Self {
                    optional: wire.optional,
                    nullable: wire.nullable,
                    default_value: wire.default_value,
                    min: wire.min,
                    max: wire.max,
                    min_length: wire.min_length,
                    max_length: wire.max_length,
                    element: wire.element,
                    allow_extra: wire.allow_extra,
                    secret: wire.secret,
                    fields: wire.fields,
                    default_cold: wire.default_cold,
                })
            })
        })
    }
}

#[derive(Deserialize)]
enum ValueWire {
    Null,
    Bool(bool),
    I64(i64),
    Text(Box<str>),
    List(Box<[InputDefaultValue]>),
    Object(Vec<(Box<str>, InputDefaultValue)>),
}

impl<'de> Deserialize<'de> for InputDefaultValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        VALUE_DEPTH.with(|depth| {
            bounded(depth, || {
                Ok(match ValueWire::deserialize(deserializer)? {
                    ValueWire::Null => Self::Null,
                    ValueWire::Bool(value) => Self::Bool(value),
                    ValueWire::I64(value) => Self::I64(value),
                    ValueWire::Text(value) => Self::Text(value),
                    ValueWire::List(values) => Self::List(values),
                    ValueWire::Object(values) => Self::Object(values),
                })
            })
        })
    }
}
