#![forbid(unsafe_code)]

use super::display::SlotValueDisplay;
use super::finite::FiniteF64;
use crate::errors::CoreResult;
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value_store::ValueStore;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Compact handle-based runtime value stored in numeric slots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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

impl fmt::Display for SlotValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::I64(v) => write!(f, "{v}"),
            Self::F64(v) => write!(f, "{v}"),
            Self::Symbol(id) => write!(f, "symbol:{}", id.get()),
            Self::List(id) => write!(f, "list:{}", id.get()),
            Self::Object(id) => write!(f, "object:{}", id.get()),
            Self::Blob(id) => write!(f, "blob:{}", id.get()),
        }
    }
}

/// Compile-time constant value, smaller than SlotValue.
/// Constants cannot hold runtime-allocated handles (List, Object, Blob).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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

    /// Resolves arena handles against the store and returns a human-readable
    /// string.  Falls back to the bare `Display` representation when the
    /// handle cannot be resolved (out-of-bounds, missing field, etc.).
    ///
    /// # Performance Note
    /// This method allocates only when formatting output. The [`SlotValueDisplay`]
    /// type defers all formatting to its `Display` implementation, keeping the
    /// hot-path value module allocation-free.
    pub fn display_with_store(&self, store: &ValueStore) -> String {
        SlotValueDisplay::new(self, store).to_string()
    }
}
