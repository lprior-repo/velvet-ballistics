use super::types::Key;
use super::{OwnedWriterError as Error, OwnedWriterQuarantine as Reason};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

pub(super) struct Entry {
    occupied: AtomicBool,
    run: AtomicU64,
    generation: AtomicU64,
    uncertainty: AtomicU8,
}

impl Entry {
    pub(super) fn empty() -> Self {
        Self {
            occupied: AtomicBool::new(false),
            run: AtomicU64::new(0),
            generation: AtomicU64::new(0),
            uncertainty: AtomicU8::new(0),
        }
    }

    pub(super) fn occupied(&self) -> bool {
        self.occupied.load(Ordering::Acquire)
    }

    pub(super) fn run(&self) -> vb_core::RunId {
        vb_core::RunId::new(self.run.load(Ordering::Acquire))
    }

    /// Called only for a proven free slot under the sole registry gate.
    pub(super) fn claim(&self, key: Key) {
        self.run.store(key.run.get(), Ordering::Relaxed);
        self.generation
            .store(key.generation.get(), Ordering::Relaxed);
        self.uncertainty.store(0, Ordering::Relaxed);
        self.occupied.store(true, Ordering::Release);
    }

    pub(super) fn check(&self, key: Key) -> Result<(), Error> {
        if !self.occupied()
            || self.run() != key.run
            || self.generation.load(Ordering::Acquire) != key.generation.get()
        {
            return Err(Error::StaleOwnership { run: key.run });
        }
        Ok(())
    }

    pub(super) fn reason(&self) -> Result<Option<Reason>, Error> {
        match self.uncertainty.load(Ordering::Acquire) {
            0 => Ok(None),
            1 => Ok(Some(Reason::IndeterminateCommit)),
            2 => Ok(Some(Reason::OwnershipUncertain)),
            _ => Err(Error::RegistryInvariant),
        }
    }

    /// No gate acquisition: uncertainty is published in the store even if its
    /// gate is busy/poisoned. Safe leases cannot be consumed while borrowed here.
    pub(super) fn quarantine(&self, key: Key, reason: Reason) -> Result<Reason, Error> {
        self.check(key)?;
        let encoded = match reason {
            Reason::IndeterminateCommit => 1,
            Reason::OwnershipUncertain => 2,
        };
        match self
            .uncertainty
            .compare_exchange(0, encoded, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(reason),
            Err(_) => self.reason()?.ok_or(Error::RegistryInvariant),
        }
    }

    /// Only this checked healthy transition can make a slot reusable.
    pub(super) fn release(&self, key: Key) -> Result<(), Error> {
        self.check(key)?;
        if let Some(reason) = self.reason()? {
            return Err(Error::Quarantined {
                run: key.run,
                reason,
            });
        }
        self.occupied.store(false, Ordering::Release);
        Ok(())
    }
}
