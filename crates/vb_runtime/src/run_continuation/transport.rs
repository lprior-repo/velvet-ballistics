//! Bounded cold component transport; durable outer event overhead is separate.
mod capture;
mod encode;
mod error;
mod operation;
mod preflight;
mod reference;
mod workflow;

pub use error::{ContinuationTransportError, TransportFailure};
use vb_core::value_store::transport::{CheckpointTransportError, TransportUsage};

/// Maximum complete component wire bytes, including its digest.
pub const MAX_CONTINUATION_COMPONENT_BYTES: usize = 1_048_576;

/// Explicit validated operation limits; accepted workflow/store caps further restrict them.
#[derive(Debug, Clone, Copy)]
pub struct ContinuationTransportLimits {
    pub(super) component: usize,
    pub(super) allocation: usize,
    pub(super) work: usize,
}

impl ContinuationTransportLimits {
    /// Refuses zero/oversized budgets; never silently increases a requested limit.
    pub fn new(
        component: usize,
        allocation: usize,
        work: usize,
    ) -> Result<Self, CheckpointTransportError> {
        if component <= 32 || component > MAX_CONTINUATION_COMPONENT_BYTES {
            return Err(CheckpointTransportError::InvalidLimit);
        }
        vb_core::value_store::transport::CheckpointTransportBudget::new(allocation, work)?;
        Ok(Self {
            component,
            allocation,
            work,
        })
    }
}

/// Immutable bounded complete component bytes. This is not a journal envelope.
#[derive(Debug)]
pub struct BoundedContinuationPayload {
    bytes: Vec<u8>,
    usage: TransportUsage,
}

impl BoundedContinuationPayload {
    /// Exact canonical component wire, including its complete content digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Conservative cumulative allocation/work charges, not allocator measurements.
    #[must_use]
    pub const fn usage(&self) -> TransportUsage {
        self.usage
    }
}
