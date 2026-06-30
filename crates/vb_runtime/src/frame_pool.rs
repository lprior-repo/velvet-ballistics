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
    /// All frames are pre-allocated at construction so `take` never falls back
    /// to fresh allocation on the runtime hot path.
    pub fn new(step_count: u16, slot_count: u16, capacity: usize) -> CoreResult<Self> {
        if capacity == 0 || capacity > MAX_POOL_CAPACITY {
            return Err(vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "frame_pool_capacity",
            });
        }
        let mut frames = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            match RunFrame::new(RunId::ZERO, StepIdx::ZERO, step_count, slot_count) {
                Ok(frame) => frames.push(frame),
                Err(_) => {
                    return Err(vb_core::errors::CoreError::InvalidCompiledWorkflow {
                        reason: "step_count_zero",
                    });
                }
            }
        }
        Ok(Self {
            frames,
            step_count,
            slot_count,
            capacity,
        })
    }

    /// Takes a frame from the pre-allocated pool.
    /// Returns an error if the pool is exhausted (all `capacity` frames are in use).
    /// This replaces the previous behavior of falling back to fresh allocation
    /// on the runtime hot path, which violated determinism and budget guarantees.
    pub fn take(&mut self, run_id: RunId, first_step: StepIdx) -> CoreResult<RunFrame> {
        let mut frame = self
            .frames
            .pop()
            .ok_or(vb_core::errors::CoreError::AllocationFailed)?;
        frame.reinitialize(run_id, first_step, self.step_count, self.slot_count)?;
        Ok(frame)
    }

    /// Returns a frame to the pool for reuse. Drops the frame if dimensions
    /// don't match or if the pool is unexpectedly at capacity.
    /// Since the pool is pre-allocated at `capacity` frames, this is normally
    /// a no-op push (the pool holds exactly `capacity` frames).
    pub fn release(&mut self, frame: RunFrame) {
        if frame.step_count() != self.step_count || frame.slot_count() != self.slot_count {
            // Mismatched dimensions: silently drop.
            return;
        }
        if self.frames.len() >= self.capacity {
            // Pool is at capacity: silently drop (should not happen in practice
            // since pre-allocation guarantees pool size matches capacity).
            return;
        }
        self.frames.push(frame);
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
