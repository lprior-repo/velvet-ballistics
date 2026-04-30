//! Runtime slot value model.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains a secret value.
    Secret = 1,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 2,
}

/// Finite floating-point scalar accepted by the runtime value model.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FiniteF64(f64);

impl Eq for FiniteF64 {}

impl FiniteF64 {
    /// Creates a finite floating-point value, rejecting NaN and infinities.
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(CoreError::NonFiniteNumber)
        }
    }

    /// Returns the raw finite floating-point value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer).and_then(|value| {
            Self::new(value).map_err(|err| serde::de::Error::custom(err.to_string()))
        })
    }
}

/// Compact handle-based runtime value stored in numeric slots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SlotValue {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Signed integer scalar for deterministic arithmetic scaffolding.
    I64(i64),
    /// Finite floating-point scalar.
    F64(FiniteF64),
    /// Interned symbol handle.
    Symbol(SymbolId),
    /// Runtime list arena handle.
    List(ListId),
    /// Runtime object arena handle.
    Object(ObjectId),
    /// Runtime blob arena/storage handle.
    Blob(BlobId),
}

impl Eq for SlotValue {}

/// Compile-time constant value, smaller than SlotValue.
/// Constants cannot hold runtime-allocated handles (List, Object, Blob).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstValue {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Signed integer scalar for deterministic arithmetic scaffolding.
    I64(i64),
    /// Finite floating-point scalar.
    F64(FiniteF64),
    /// Interned symbol handle.
    Symbol(SymbolId),
}

impl ConstValue {
    /// Convert to a runtime slot value.
    pub fn to_slot_value(&self) -> CoreResult<SlotValue> {
        match self {
            Self::Null => Ok(SlotValue::Null),
            Self::Bool(v) => Ok(SlotValue::Bool(*v)),
            Self::I64(v) => Ok(SlotValue::I64(*v)),
            Self::F64(v) => Ok(SlotValue::F64(*v)),
            Self::Symbol(v) => Ok(SlotValue::Symbol(*v)),
        }
    }
}

impl SlotValue {
    /// Returns the stable runtime type name for diagnostics.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::I64(_) | Self::F64(_) => "number",
            Self::Symbol(_) => "symbol",
            Self::List(_) => "list",
            Self::Object(_) => "object",
            Self::Blob(_) => "blob",
        }
    }

    /// Returns true only for `Bool(true)`.
    #[must_use]
    pub const fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}
