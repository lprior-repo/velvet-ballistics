use super::{CarrierError, ContinuationCarrierBudget, ContinuationCarrierHeader};
use std::sync::Arc;

/// Validated cold record bytes, emitted only by the budgeted manual writer.
#[derive(Debug)]
pub struct BoundedContinuationRecord {
    pub(super) bytes: Vec<u8>,
}
impl BoundedContinuationRecord {
    /// Complete envelope and payload bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Opaque shared immutable event data; deliberately implements neither Serde trait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedContinuationCarrier {
    header: ContinuationCarrierHeader,
    component: Arc<Vec<u8>>,
}
impl OwnedContinuationCarrier {
    /// Exact immutable validated scalar fields.
    pub const fn header(&self) -> ContinuationCarrierHeader {
        self.header
    }

    /// Shared immutable opaque bytes; no nested runtime authority is implied.
    pub fn component(&self) -> &[u8] {
        self.component.as_slice()
    }
}

/// Allocation-free, exact-key-validated view into a complete cold record.
#[derive(Debug)]
pub struct OwnedContinuationCarrierView<'a> {
    pub(super) header: ContinuationCarrierHeader,
    pub(super) component: &'a [u8],
}
impl<'a> OwnedContinuationCarrierView<'a> {
    /// Exact immutable validated scalar fields.
    pub const fn header(&self) -> ContinuationCarrierHeader {
        self.header
    }

    /// Borrow the complete component including its digest.
    pub const fn component(&self) -> &'a [u8] {
        self.component
    }

    /// One precharged opaque owning copy. Event clones share the vector.
    pub fn into_event(
        self,
        operation: &mut ContinuationCarrierBudget,
    ) -> Result<crate::JournalEvent, CarrierError> {
        if self.component.len() > operation.record_limit {
            return Err(
                vb_core::value_store::transport::CheckpointTransportError::Bound("whole_record")
                    .into(),
            );
        }
        operation.copy()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.component.len())
            .map_err(|_| vb_core::value_store::transport::CheckpointTransportError::Allocation)?;
        bytes.extend_from_slice(self.component);
        // The precharged 512-byte fixed overhead includes the standard Arc
        // control/value allocation; std provides no stable fallible Arc::new.
        Ok(crate::JournalEvent::RunContinuation {
            carrier: OwnedContinuationCarrier {
                header: self.header,
                component: Arc::new(bytes),
            },
        })
    }
}
