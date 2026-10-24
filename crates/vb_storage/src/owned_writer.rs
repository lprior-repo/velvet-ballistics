//! Store-owned bounded writer capabilities, not yet attached to any append path.
//!
//! The first accepted capacity binds this journal for its entire lifetime. Future
//! Runtime wiring must supply the actual owning shard's nonzero max_active_runs.
//! This primitive proves neither durable-prefix provenance nor commit certainty.
mod entry;
mod error;
mod lease;
mod registry;
mod table;
mod types;

pub use error::{OwnedWriterError, OwnedWriterQuarantine};
pub use lease::OwnedWriterLease;
pub(crate) use registry::OwnedWriterRegistry;

use crate::FjallJournal;
use std::sync::Arc;

/// Maximum fixed capacity of one journal's owned-writer registry.
pub const MAX_OWNED_WRITERS: usize = 65_535;

impl FjallJournal {
    /// Acquires one store-bound writer without waiting, spinning or retrying.
    /// Capacity binds only after the initial fallible table reservation succeeds.
    pub fn try_acquire_owned_writer(
        self: &Arc<Self>,
        run: vb_core::RunId,
        requested_capacity: usize,
    ) -> Result<OwnedWriterLease, OwnedWriterError> {
        let capacity = types::Capacity::new(requested_capacity)?;
        let key = self.owned_writers.acquire(run, capacity)?;
        Ok(OwnedWriterLease::new(Arc::clone(self), key))
    }

    /// Checks actual journal identity and current unquarantined ownership.
    /// This is an ownership check, never evidence of a successful commit.
    pub fn validate_owned_writer(
        self: &Arc<Self>,
        lease: &OwnedWriterLease,
    ) -> Result<(), OwnedWriterError> {
        if !Arc::ptr_eq(self, lease.journal()) {
            return Err(OwnedWriterError::WrongStore);
        }
        self.owned_writers.validate(lease.key)
    }
}
