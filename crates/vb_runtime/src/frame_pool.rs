#![forbid(unsafe_code)]

//! Bounded frame pool for RunFrame reuse.

use vb_core::errors::CoreResult;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};

/// Maximum number of frames a pool can hold.
const MAX_POOL_CAPACITY: usize = 4_096;

/// Bounded pool of reusable RunFrame instances.
///
/// Frames are allocated with a fixed step_count and slot_count derived from
/// the workflow's resource contract. Callers take frames for exclusive use
/// and release them back when the run completes or is cancelled.
#[derive(Debug)]
pub struct FramePool {
    frames: Vec<RunFrame>,
    step_count: u16,
    slot_count: u16,
    capacity: usize,
}

impl FramePool {
    /// Creates a new frame pool with the given dimensions and capacity.
    pub fn new(step_count: u16, slot_count: u16, capacity: usize) -> CoreResult<Self> {
        if capacity == 0 || capacity > MAX_POOL_CAPACITY {
            return Err(vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "frame_pool_capacity",
            });
        }
        Ok(Self {
            frames: Vec::new(),
            step_count,
            slot_count,
            capacity,
        })
    }

    /// Takes a frame from the pool or allocates a new one if the pool is empty.
    /// Returns an error if the pool is empty and allocation is denied.
    pub fn take(&mut self, run_id: RunId, first_step: StepIdx) -> CoreResult<RunFrame> {
        if self.frames.is_empty() {
            RunFrame::new(run_id, first_step, self.step_count, self.slot_count)
        } else {
            let mut frame = self
                .frames
                .pop()
                .ok_or(vb_core::errors::CoreError::AllocationFailed)?;
            frame.reinitialize(run_id, first_step, self.step_count, self.slot_count)?;
            Ok(frame)
        }
    }

    /// Returns a frame to the pool for reuse. Drops the frame if the pool is
    /// at capacity.
    pub fn release(&mut self, frame: RunFrame) {
        if frame.step_count() == self.step_count
            && frame.slot_count() == self.slot_count
            && self.frames.len() < self.capacity
        {
            self.frames.push(frame);
        }
        // Frame is dropped when the pool is full.
    }

    /// Number of frames currently available in the pool.
    #[must_use]
    pub fn available(&self) -> usize {
        self.frames.len()
    }

    /// Returns true when the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Returns the pool's configured capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
#[path = "frame_pool/tests.rs"]
mod tests;
