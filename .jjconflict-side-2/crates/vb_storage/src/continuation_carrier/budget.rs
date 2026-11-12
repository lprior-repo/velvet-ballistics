use super::{CarrierError, CarrierUsage, MAX_CONTINUATION_RECORD_BYTES};
use vb_core::value_store::transport::{
    CheckpointTransportBudget, CheckpointTransportError as Error,
};

/// Single operation account. Outer costs and inner costs share the same ceilings.
#[derive(Debug)]
pub struct ContinuationCarrierBudget {
    pub(super) record_limit: usize,
    ledger: CheckpointTransportBudget,
    allocation: usize,
    work: usize,
    reserved: bool,
    wire_used: bool,
    copy_used: bool,
    loan_used: bool,
    closed: bool,
}

impl ContinuationCarrierBudget {
    /// Rejects zero/oversized limits without silently expanding them.
    pub fn new(record: usize, allocation: usize, work: usize) -> Result<Self, CarrierError> {
        if record <= crate::RECORD_HEADER_BYTES || record > MAX_CONTINUATION_RECORD_BYTES {
            return Err(Error::InvalidLimit.into());
        }
        Ok(Self {
            record_limit: record,
            ledger: CheckpointTransportBudget::new(allocation, work)?,
            allocation,
            work,
            reserved: false,
            wire_used: false,
            copy_used: false,
            loan_used: false,
            closed: false,
        })
    }

    /// Reserve once BEFORE outer work or component capture. These are conservative
    /// cumulative bounds, not measurements of allocator behavior.
    pub fn reserve_outer(&mut self) -> Result<(), CarrierError> {
        if self.closed {
            return Err(CarrierError::AccountClosed);
        }
        if self.reserved {
            return Ok(());
        }
        self.closed = true;
        let allocation = self
            .record_limit
            .checked_mul(3)
            .and_then(|n| n.checked_add(512))
            .ok_or(Error::Overflow)?;
        let work = self
            .record_limit
            .checked_mul(4)
            .and_then(|n| n.checked_add(1024))
            .ok_or(Error::Overflow)?;
        self.ledger.reserve(allocation, 1)?;
        self.ledger.work(work)?;
        self.reserved = true;
        self.closed = false;
        Ok(())
    }

    pub(super) fn wire(&mut self) -> Result<(), CarrierError> {
        self.reserve_outer()?;
        if self.wire_used {
            return Err(CarrierError::AccountClosed);
        }
        self.wire_used = true;
        Ok(())
    }

    pub(super) fn copy(&mut self) -> Result<(), CarrierError> {
        if self.closed || !self.reserved || self.copy_used {
            return Err(CarrierError::AccountClosed);
        }
        self.copy_used = true;
        Ok(())
    }

    /// A non-Clone exclusive allowance; abandoning it permanently closes this account.
    pub fn component_allowance(&mut self) -> Result<ComponentAllowance<'_>, CarrierError> {
        self.reserve_outer()?;
        if self.loan_used {
            return Err(CarrierError::AccountClosed);
        }
        self.loan_used = true;
        let usage = self.usage();
        let allocation = self
            .allocation
            .checked_sub(usage.allocation_bytes)
            .ok_or(Error::Overflow)?;
        let work = self.work.checked_sub(usage.work).ok_or(Error::Overflow)?;
        Ok(ComponentAllowance {
            account: self,
            allocation,
            work,
            accounted: false,
        })
    }

    /// Cumulative accepted charges, also available after typed refusal.
    pub const fn usage(&self) -> CarrierUsage {
        self.ledger.usage()
    }
}

/// Residual limits are tied to this account by an exclusive borrow, never cloned.
#[derive(Debug)]
pub struct ComponentAllowance<'a> {
    account: &'a mut ContinuationCarrierBudget,
    allocation: usize,
    work: usize,
    accounted: bool,
}

impl ComponentAllowance<'_> {
    /// Pass these residual limits to the actual strong component transport API.
    pub const fn limits(&self) -> (usize, usize) {
        (self.allocation, self.work)
    }

    /// Debit the actual returned usage on BOTH success and typed error paths.
    /// An invalid returned charge closes the account instead of restoring authority.
    pub fn absorb(mut self, usage: CarrierUsage) -> Result<(), CarrierError> {
        self.account.closed = true;
        self.account.ledger.reserve(usage.allocation_bytes, 1)?;
        self.account.ledger.work(usage.work)?;
        self.account.closed = false;
        self.accounted = true;
        Ok(())
    }
}

impl Drop for ComponentAllowance<'_> {
    fn drop(&mut self) {
        if !self.accounted {
            self.account.closed = true;
        }
    }
}
