/// Hard ceiling on cumulative charged allocation in one transport operation.
const MAX_OPERATION_BYTES: usize = 67_108_864;
/// Hard ceiling on structural work in one transport operation.
const MAX_OPERATION_WORK: usize = 16_777_216;

/// Charges are conservative cumulative bounds, not allocator measurements.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TransportUsage {
    /// Bytes charged before any corresponding allocation or strong decode.
    pub allocation_bytes: usize,
    /// Structural elements/bytes inspected or reserved for later validators.
    pub work: usize,
}

/// Allocation-free structural transport refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CheckpointTransportError {
    /// A requested limit is zero or exceeds a hard ceiling.
    #[error("invalid transport limit")]
    InvalidLimit,
    /// Checked arithmetic failed.
    #[error("transport arithmetic overflow")]
    Overflow,
    /// A declared structure exceeds its independently accepted bound.
    #[error("transport bound exceeded: {0}")]
    Bound(&'static str),
    /// Allocation or structural work would exceed the operation budget.
    #[error("transport budget exceeded: {0}")]
    Budget(&'static str),
    /// The byte stream is truncated, noncanonical or structurally invalid.
    #[error("malformed transport bytes: {0}")]
    Malformed(&'static str),
    /// A fallible allocation failed.
    #[error("transport allocation failed")]
    Allocation,
}

/// Checked cumulative allocation/work ledger shared by structural cursors.
#[derive(Debug)]
pub struct CheckpointTransportBudget {
    allocation_limit: usize,
    work_limit: usize,
    usage: TransportUsage,
}

impl CheckpointTransportBudget {
    /// Validates explicit positive limits against fixed hard ceilings.
    pub fn new(
        allocation_limit: usize,
        work_limit: usize,
    ) -> Result<Self, CheckpointTransportError> {
        if allocation_limit == 0
            || allocation_limit > MAX_OPERATION_BYTES
            || work_limit == 0
            || work_limit > MAX_OPERATION_WORK
        {
            return Err(CheckpointTransportError::InvalidLimit);
        }
        Ok(Self {
            allocation_limit,
            work_limit,
            usage: TransportUsage::default(),
        })
    }

    /// Charges an allocation upper bound before growth, copying or decoding.
    pub fn reserve(&mut self, count: usize, width: usize) -> Result<(), CheckpointTransportError> {
        let bytes = count
            .checked_mul(width)
            .ok_or(CheckpointTransportError::Overflow)?;
        let total = self
            .usage
            .allocation_bytes
            .checked_add(bytes)
            .ok_or(CheckpointTransportError::Overflow)?;
        if total > self.allocation_limit {
            return Err(CheckpointTransportError::Budget("allocation_bytes"));
        }
        self.usage.allocation_bytes = total;
        Ok(())
    }

    /// Charges a checked upper bound on structural work.
    pub fn work(&mut self, count: usize) -> Result<(), CheckpointTransportError> {
        let total = self
            .usage
            .work
            .checked_add(count)
            .ok_or(CheckpointTransportError::Overflow)?;
        if total > self.work_limit {
            return Err(CheckpointTransportError::Budget("work"));
        }
        self.usage.work = total;
        Ok(())
    }

    /// Returns charges already accepted; a refused charge is not applied.
    #[must_use]
    pub const fn usage(&self) -> TransportUsage {
        self.usage
    }
}
