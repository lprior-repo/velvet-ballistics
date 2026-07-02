#![forbid(unsafe_code)]

//! Pure shard-owned prefix completion watermark.

use vb_core::RunId;

/// Errors returned by the bounded completion watermark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompletionWatermarkError {
    /// Completion receipt belongs to a different run.
    #[error("completion receipt belongs to wrong run")]
    WrongRun {
        /// Run owned by this watermark.
        expected: RunId,
        /// Run carried by the receipt.
        actual: RunId,
    },
    /// Sequence zero is invalid for completion receipts and waiters.
    #[error("completion sequence must be one-based")]
    InvalidSequence {
        /// Invalid sequence.
        seq: u64,
    },
    /// Sequence was already drained or is already pending.
    #[error("completion sequence is duplicate or already drained")]
    Duplicate {
        /// Duplicate sequence.
        seq: u64,
    },
    /// Pending completion or waiter capacity is full.
    #[error("completion watermark queue is full")]
    QueueFull {
        /// Queue capacity.
        capacity: usize,
    },
}

/// Result of admitting a completion receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionDrain {
    /// New prefix boundary after draining.
    pub boundary: u64,
    /// Numeric completions drained by this receipt.
    pub drained: Box<[u64]>,
}

/// Bounded prefix-drain state for one shard-owned run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionWatermark {
    run: RunId,
    boundary: u64,
    max_pending: usize,
    max_waiters: usize,
    pending: Vec<u64>,
    waiters: Vec<u64>,
}

impl CompletionWatermark {
    /// Creates an empty watermark for one run.
    #[must_use]
    pub fn new(run: RunId, max_pending: usize, max_waiters: usize) -> Self {
        Self {
            run,
            boundary: 0,
            max_pending,
            max_waiters,
            pending: Vec::with_capacity(max_pending),
            waiters: Vec::with_capacity(max_waiters),
        }
    }

    /// Creates a watermark from a recovered durable boundary.
    #[must_use]
    pub fn from_boundary(
        run: RunId,
        boundary: u64,
        max_pending: usize,
        max_waiters: usize,
    ) -> Self {
        Self {
            run,
            boundary,
            max_pending,
            max_waiters,
            pending: Vec::with_capacity(max_pending),
            waiters: Vec::with_capacity(max_waiters),
        }
    }

    /// Returns the drained contiguous prefix boundary.
    #[must_use]
    pub const fn boundary(&self) -> u64 {
        self.boundary
    }

    /// Returns pending non-prefix completions.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Returns registered waiter count.
    #[must_use]
    pub fn waiter_len(&self) -> usize {
        self.waiters.len()
    }

    /// Registers interest in a future sequence.
    pub fn register_waiter(&mut self, seq: u64) -> Result<(), CompletionWatermarkError> {
        self.validate_sequence(seq)?;
        if seq <= self.boundary || self.waiters.contains(&seq) {
            return Ok(());
        }
        if self.waiters.len() >= self.max_waiters {
            return Err(CompletionWatermarkError::QueueFull {
                capacity: self.max_waiters,
            });
        }
        self.waiters.push(seq);
        Ok(())
    }

    /// Accepts a completion and drains every now-contiguous prefix sequence.
    pub fn complete(
        &mut self,
        run: RunId,
        seq: u64,
    ) -> Result<CompletionDrain, CompletionWatermarkError> {
        self.validate_run(run)?;
        self.validate_sequence(seq)?;
        self.reject_duplicate_or_drained(seq)?;
        self.push_pending(seq)?;
        let drained = self.drain_prefix();
        Ok(CompletionDrain {
            boundary: self.boundary,
            drained: drained.into_boxed_slice(),
        })
    }

    fn validate_run(&self, actual: RunId) -> Result<(), CompletionWatermarkError> {
        if actual == self.run {
            Ok(())
        } else {
            Err(CompletionWatermarkError::WrongRun {
                expected: self.run,
                actual,
            })
        }
    }

    const fn validate_sequence(&self, seq: u64) -> Result<(), CompletionWatermarkError> {
        if seq == 0 {
            Err(CompletionWatermarkError::InvalidSequence { seq })
        } else {
            Ok(())
        }
    }

    fn reject_duplicate_or_drained(&self, seq: u64) -> Result<(), CompletionWatermarkError> {
        if seq <= self.boundary || self.pending.contains(&seq) {
            Err(CompletionWatermarkError::Duplicate { seq })
        } else {
            Ok(())
        }
    }

    fn push_pending(&mut self, seq: u64) -> Result<(), CompletionWatermarkError> {
        if self.pending.len() >= self.max_pending {
            return Err(CompletionWatermarkError::QueueFull {
                capacity: self.max_pending,
            });
        }
        self.pending.push(seq);
        Ok(())
    }

    fn drain_prefix(&mut self) -> Vec<u64> {
        std::iter::from_fn(|| self.drain_next()).collect()
    }

    fn drain_next(&mut self) -> Option<u64> {
        let next = self.boundary.checked_add(1)?;
        self.remove_pending(next).then(|| {
            self.boundary = next;
            self.waiters.retain(|waiter| *waiter != next);
            next
        })
    }

    fn remove_pending(&mut self, seq: u64) -> bool {
        match self.pending.iter().position(|candidate| *candidate == seq) {
            Some(position) => {
                self.pending.swap_remove(position);
                true
            }
            None => false,
        }
    }
}
