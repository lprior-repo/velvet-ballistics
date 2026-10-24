use super::{ContinuationTransportLimits, TransportFailure};
use serde::Serialize;
use vb_core::value_store::transport::{
    CheckpointTransportBudget, CheckpointTransportError as Error,
};

pub(super) struct Operation {
    pub(super) budget: CheckpointTransportBudget,
    pub(super) scratch: Vec<u8>,
    pub(super) decoded: bool,
}

impl Operation {
    pub(super) fn new(limits: ContinuationTransportLimits) -> Result<Self, TransportFailure> {
        let mut budget = CheckpointTransportBudget::new(limits.allocation, limits.work)?;
        // Scratch, output, strong digest/canonical temporaries and frame/graph baseline.
        budget.work(limits.component)?;
        budget.reserve(limits.component, 4)?;
        budget.reserve(1, 16_384)?;
        let scratch = buffer(limits.component)?;
        Ok(Self {
            budget,
            scratch,
            decoded: false,
        })
    }

    pub(super) fn known<T: Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<&[u8], TransportFailure> {
        let bytes = postcard::to_slice(value, &mut self.scratch)?;
        // Every nonzero-sized wire element in the accepted DTO schema consumes
        // >=1 byte. 128 covers owning metadata, validation clones and indexes.
        self.budget.reserve(bytes.len(), 128)?;
        self.budget.work(bytes.len())?;
        Ok(bytes)
    }
}

pub(super) fn buffer(size: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::Allocation)?;
    bytes.resize(size, 0);
    Ok(bytes)
}
