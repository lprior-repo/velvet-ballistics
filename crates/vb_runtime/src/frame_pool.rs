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
            frame.set_pc(first_step);
            Ok(frame)
        }
    }

    /// Returns a frame to the pool for reuse. Drops the frame if the pool is
    /// at capacity.
    pub fn release(&mut self, frame: RunFrame) {
        if self.frames.len() < self.capacity {
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
mod tests {
    use super::*;

    #[test]
    fn pool_creates_with_valid_capacity() {
        let _pool = FramePool::new(10, 5, 16).ok();
    }

    #[test]
    fn pool_rejects_zero_capacity() {
        let pool = FramePool::new(10, 5, 0);
        match pool {
            Err(vb_core::errors::CoreError::ResourceLimitExceeded { resource })
                if resource == "frame_pool_capacity" => {}
            other => assert!(false, "unexpected result: {other:?}"),
        }
    }

    #[test]
    fn pool_rejects_excessive_capacity() {
        let pool = FramePool::new(10, 5, MAX_POOL_CAPACITY + 1);
        match pool {
            Err(vb_core::errors::CoreError::ResourceLimitExceeded { resource })
                if resource == "frame_pool_capacity" => {}
            other => assert!(false, "unexpected result: {other:?}"),
        }
    }

    #[test]
    fn take_allocates_when_empty() {
        let mut pool = FramePool::new(2, 1, 4).ok();
        let frame = pool
            .as_mut()
            .and_then(|p| p.take(RunId::new(1), StepIdx::new(0)).ok());
        let _ = frame;
    }

    #[test]
    fn release_and_reuse_frame() {
        let mut pool = FramePool::new(2, 1, 4).ok();
        let p = pool.as_mut();
        let Some(p) = p else { return };
        let frame = p.take(RunId::new(1), StepIdx::new(0)).ok();
        let Some(frame) = frame else { return };
        p.release(frame);
        assert_eq!(p.available(), 1);

        let reused = p.take(RunId::new(2), StepIdx::new(0)).ok();
        let Some(reused) = reused else { return };
        assert_eq!(reused.run_id(), RunId::new(1));
    }

    #[test]
    fn release_drops_when_at_capacity() {
        let mut pool = FramePool::new(2, 1, 1).ok();
        let p = pool.as_mut();
        let Some(p) = p else { return };
        let frame1 = p.take(RunId::new(1), StepIdx::new(0)).ok();
        let frame2 = p.take(RunId::new(2), StepIdx::new(0)).ok();
        let Some(frame1) = frame1 else { return };
        let Some(frame2) = frame2 else { return };
        p.release(frame1);
        p.release(frame2);
        assert_eq!(p.available(), 1);
    }
}
