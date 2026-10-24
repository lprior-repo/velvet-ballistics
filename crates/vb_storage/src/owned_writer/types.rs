use super::{MAX_OWNED_WRITERS, OwnedWriterError as Error};
use core::num::{NonZeroU16, NonZeroU64};

#[derive(Clone, Copy)]
pub(super) struct Capacity(NonZeroU16);

impl Capacity {
    pub(super) fn new(requested: usize) -> Result<Self, Error> {
        let value = u16::try_from(requested).ok().and_then(NonZeroU16::new);
        value.map(Self).ok_or(Error::InvalidCapacity {
            requested,
            maximum: MAX_OWNED_WRITERS,
        })
    }

    pub(super) fn get(self) -> usize {
        usize::from(self.0.get())
    }
}

/// A private identity is never itself a public capability.
#[derive(Debug, Clone, Copy)]
pub(super) struct Key {
    pub(super) run: vb_core::RunId,
    pub(super) index: usize,
    pub(super) generation: NonZeroU64,
}

#[derive(Default)]
pub(super) struct Generations {
    pub(super) last: Option<NonZeroU64>,
    exhausted: bool,
}

impl Generations {
    pub(super) fn ensure_available(&self) -> Result<(), Error> {
        if self.exhausted {
            return Err(Error::GenerationExhausted);
        }
        Ok(())
    }

    pub(super) fn issue(&mut self) -> Result<NonZeroU64, Error> {
        self.ensure_available()?;
        let next = match self.last {
            None => Some(NonZeroU64::MIN),
            Some(previous) => previous.checked_add(1),
        };
        let Some(next) = next else {
            self.exhausted = true;
            return Err(Error::GenerationExhausted);
        };
        self.last = Some(next);
        Ok(next)
    }
}
