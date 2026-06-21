//! Storage identifiers: `BlobId` and `ActionId`.

#![forbid(unsafe_code)]

use crate::ids::macros::numeric_id;

numeric_id!(BlobId, u64, get);
numeric_id!(ActionId, u16, get);

// ── BlobId additional methods ──────────────────────────────────────────

impl BlobId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}
