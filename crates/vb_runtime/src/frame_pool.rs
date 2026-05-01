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

    fn new_pool(step_count: u16, slot_count: u16, capacity: usize) -> FramePool {
        let result = FramePool::new(step_count, slot_count, capacity);
        assert_eq!(result.as_ref().map(|_| ()), Ok(()));
        match result {
            Ok(pool) => pool,
            Err(_) => unreachable!("asserted Ok above"),
        }
    }

    #[test]
    fn pool_creates_with_valid_capacity() {
        assert_eq!(FramePool::new(10, 5, 16).as_ref().map(|_| ()), Ok(()));
    }

    #[test]
    fn pool_rejects_zero_capacity() {
        let result = FramePool::new(10, 5, 0);
        assert_eq!(
            result.as_ref().map(|_| ()),
            Err(&vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "frame_pool_capacity",
            })
        );
    }

    #[test]
    fn pool_rejects_excessive_capacity() {
        let result = FramePool::new(10, 5, MAX_POOL_CAPACITY + 1);
        assert_eq!(
            result.as_ref().map(|_| ()),
            Err(&vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "frame_pool_capacity",
            })
        );
    }

    #[test]
    fn take_allocates_when_empty() {
        let mut pool = new_pool(2, 1, 4);
        let frame = pool.take(RunId::new(1), StepIdx::new(0));
        assert_eq!(frame.map(|f| f.run_id()), Ok(RunId::new(1)));
    }

    #[test]
    fn release_and_reuse_frame() {
        let mut pool = new_pool(2, 1, 4);
        let frame = pool.take(RunId::new(1), StepIdx::new(0));
        assert_eq!(frame.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
        let frame = match frame {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        assert_eq!(pool.available(), 1);

        let reused = pool.take(RunId::new(2), StepIdx::new(0));
        assert_eq!(reused.map(|f| f.run_id()), Ok(RunId::new(1)));
    }

    #[test]
    fn release_drops_when_at_capacity() {
        let mut pool = new_pool(2, 1, 1);
        let frame1 = pool.take(RunId::new(1), StepIdx::new(0));
        assert_eq!(frame1.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
        let frame2 = pool.take(RunId::new(2), StepIdx::new(0));
        assert_eq!(frame2.as_ref().map(|f| f.run_id()), Ok(RunId::new(2)));
        let frame1 = match frame1 {
            Ok(f) => f,
            Err(_) => return,
        };
        let frame2 = match frame2 {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame1);
        pool.release(frame2);
        assert_eq!(pool.available(), 1);
    }

    #[test]
    fn is_empty_returns_true_for_new_pool() {
        let pool = new_pool(2, 1, 4);
        assert_eq!(pool.is_empty(), true);
    }

    #[test]
    fn is_empty_returns_false_after_release() {
        let mut pool = new_pool(2, 1, 4);
        assert_eq!(pool.is_empty(), true);
        let frame = pool.take(RunId::new(1), StepIdx::new(0));
        assert_eq!(pool.is_empty(), true);
        let frame = match frame {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        assert_eq!(pool.is_empty(), false);
    }

    #[test]
    fn capacity_returns_configured_value() {
        let pool = new_pool(10, 5, 42);
        assert_eq!(pool.capacity(), 42);
    }
}
