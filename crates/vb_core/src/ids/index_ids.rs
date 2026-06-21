//! Index identifiers for compiled IR: `ExprIdx`, `AccessorIdx`, and `ConstIdx`.
//!
//! These wrap `u16` and provide checked `as_usize` conversion for safe
//! slice access in the hot path.

#![forbid(unsafe_code)]

use crate::ids::macros::{checked_index, numeric_id};

numeric_id!(ExprIdx, u16, get);
numeric_id!(AccessorIdx, u16, get);
numeric_id!(ConstIdx, u16, get);

checked_index!(ExprIdx);
checked_index!(AccessorIdx);
checked_index!(ConstIdx);

// ── ConstIdx additional methods ────────────────────────────────────────

impl ConstIdx {
    /// Adds without overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: u16) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}
