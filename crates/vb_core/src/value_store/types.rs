//! Value store type definitions.

use crate::ids::SymbolId;
use crate::value::{SlotValue, Taint};

/// Deterministic object field stored in insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectField {
    /// Interned field name.
    pub key: SymbolId,
    /// Handle-only field value.
    pub value: SlotValue,
    /// Taint level of the field value.
    pub taint: Taint,
}

impl ObjectField {
    /// Creates a clean-tainted object field.
    #[must_use]
    pub const fn clean(key: SymbolId, value: SlotValue) -> Self {
        Self {
            key,
            value,
            taint: Taint::Clean,
        }
    }

    /// Creates an object field with explicit taint.
    #[must_use]
    pub const fn with_taint(key: SymbolId, value: SlotValue, taint: Taint) -> Self {
        Self { key, value, taint }
    }
}
