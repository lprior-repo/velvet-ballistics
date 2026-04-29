//! Runtime slot value model.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

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

/// Compact runtime value stored in numeric slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotValue {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Signed integer scalar for deterministic arithmetic scaffolding.
    I64(i64),
    /// UTF-8 text value.
    Text(Box<str>),
    /// Shared byte buffer for IPC/action boundaries.
    Bytes(Bytes),
}

impl SlotValue {
    /// Returns true only for `Bool(true)`.
    #[must_use]
    pub const fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}
