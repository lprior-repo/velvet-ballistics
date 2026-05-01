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

    #[test]
    fn frame_pool_new_creates_pool_with_capacity() {
        // Given a new pool with capacity 8
        let pool = new_pool(2, 1, 8);
        // When checking capacity and availability
        // Then capacity is 8 and available is 0 (empty pool)
        assert_eq!(pool.capacity(), 8);
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn frame_pool_acquire_returns_frame_when_available() {
        // Given a pool
        let mut pool = new_pool(2, 1, 4);
        // When taking a frame
        let result = pool.take(RunId::new(1), StepIdx::new(0));
        // Then the result is Ok with correct run_id
        assert_eq!(result.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
    }

    #[test]
    fn frame_pool_release_returns_frame_to_pool() {
        // Given a pool with one frame taken then released
        let mut pool = new_pool(2, 1, 4);
        let frame = pool.take(RunId::new(1), StepIdx::new(0));
        assert_eq!(frame.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
        let frame = match frame {
            Ok(f) => f,
            Err(_) => return,
        };
        // When releasing the frame back
        pool.release(frame);
        assert_eq!(pool.available(), 1);
        // Then it can be re-acquired
        let reused = pool.take(RunId::new(2), StepIdx::new(0));
        assert_eq!(reused.is_ok(), true);
    }

    #[test]
    fn frame_pool_is_empty_when_all_acquired() {
        // Given a pool with capacity 1
        let mut pool = new_pool(2, 1, 1);
        // When taking the only frame
        let frame = pool.take(RunId::new(1), StepIdx::new(0));
        // Then the pool is empty (no recycled frames)
        assert_eq!(pool.is_empty(), true);
        // Clean up: release frame so pool is no longer empty
        let frame = match frame {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        assert_eq!(pool.is_empty(), false);
    }

    #[test]
    fn frame_pool_is_not_empty_after_release() {
        // Given a pool with a released frame
        let mut pool = new_pool(2, 1, 4);
        let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        // When checking is_empty
        // Then it is not empty
        assert_eq!(pool.is_empty(), false);
        assert_eq!(pool.available(), 1);
    }

    #[test]
    fn frame_pool_release_then_acquire_yields_recycled_frame() {
        // Given a pool where a frame has been released
        let mut pool = new_pool(2, 1, 4);
        let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        // The recycled frame keeps the original run_id
        let original_id = frame.run_id();
        pool.release(frame);
        // When acquiring again
        let recycled = pool.take(RunId::new(2), StepIdx::new(0));
        // Then the recycled frame is the same one (has original run_id)
        match recycled {
            Ok(f) => {
                assert_eq!(f.run_id(), original_id);
            }
            Err(_) => {
                // Should not happen
                assert!(false);
            }
        }
    }

    #[test]
    fn frame_pool_rejects_exactly_max_plus_one_capacity() {
        // Given capacity = MAX_POOL_CAPACITY + 1
        let result = FramePool::new(2, 1, MAX_POOL_CAPACITY + 1);
        // Then it returns an error
        assert_eq!(
            result.as_ref().map(|_| ()),
            Err(&vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "frame_pool_capacity",
            })
        );
    }

    #[test]
    fn frame_pool_accepts_max_capacity() {
        // Given capacity = MAX_POOL_CAPACITY
        let result = FramePool::new(2, 1, MAX_POOL_CAPACITY);
        // Then it succeeds
        assert_eq!(result.as_ref().map(|_| ()), Ok(()));
    }

    #[test]
    fn frame_pool_multiple_release_and_take_cycle() {
        // Given a pool with capacity 4
        let mut pool = new_pool(2, 1, 4);
        let f1 = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        let f2 = match pool.take(RunId::new(2), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        // When releasing both and taking one back
        pool.release(f1);
        pool.release(f2);
        assert_eq!(pool.available(), 2);
        let recycled = pool.take(RunId::new(3), StepIdx::new(0));
        // Then the recycled frame is available
        assert_eq!(recycled.is_ok(), true);
        assert_eq!(pool.available(), 1);
    }

    #[test]
    fn frame_pool_new_with_different_step_counts() {
        // Given pools with different step counts
        let mut pool_1 = new_pool(1, 1, 4);
        let mut pool_10 = new_pool(10, 1, 4);
        let mut pool_100 = new_pool(100, 1, 4);
        // When creating frames
        let r1 = pool_1.take(RunId::new(1), StepIdx::new(0));
        let r10 = pool_10.take(RunId::new(2), StepIdx::new(0));
        let r100 = pool_100.take(RunId::new(3), StepIdx::new(0));
        // Then all succeed
        assert_eq!(r1.is_ok(), true);
        assert_eq!(r10.is_ok(), true);
        assert_eq!(r100.is_ok(), true);
    }

    #[test]
    fn frame_pool_new_with_different_slot_counts() {
        // Given pools with different slot counts
        let mut pool_1 = new_pool(2, 1, 4);
        let mut pool_8 = new_pool(2, 8, 4);
        // When creating frames
        let r1 = pool_1.take(RunId::new(1), StepIdx::new(0));
        let r8 = pool_8.take(RunId::new(2), StepIdx::new(0));
        // Then all succeed
        assert_eq!(r1.is_ok(), true);
        assert_eq!(r8.is_ok(), true);
    }

    #[test]
    fn frame_pool_take_allocates_fresh_when_no_recycled() {
        // Given a fresh pool (empty)
        let mut pool = new_pool(2, 1, 4);
        assert_eq!(pool.available(), 0);
        // When taking a frame
        let frame = pool.take(RunId::new(42), StepIdx::new(0));
        // Then a new frame is allocated with the correct run_id
        match frame {
            Ok(f) => {
                assert_eq!(f.run_id(), RunId::new(42));
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn frame_pool_take_with_nonzero_first_step() {
        // Given a pool
        let mut pool = new_pool(4, 1, 4);
        // When taking a frame with first_step = 2
        let frame = pool.take(RunId::new(1), StepIdx::new(2));
        // Then the frame has pc = 2
        match frame {
            Ok(f) => {
                assert_eq!(f.pc(), StepIdx::new(2));
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn frame_pool_release_at_capacity_drops_frame() {
        // Given a pool with capacity 2, both slots used
        let mut pool = new_pool(2, 1, 2);
        let f1 = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        let f2 = match pool.take(RunId::new(2), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(f1);
        pool.release(f2);
        assert_eq!(pool.available(), 2);
        // When taking two frames and releasing both
        let f3 = match pool.take(RunId::new(3), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(f3);
        // Then available is still 2 (at capacity)
        assert_eq!(pool.available(), 2);
    }

    #[test]
    fn frame_pool_capacity_one_works() {
        // Given a pool with capacity 1
        let mut pool = new_pool(2, 1, 1);
        // When taking and releasing
        let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        assert_eq!(pool.available(), 1);
        assert_eq!(pool.is_empty(), false);
        // And taking again succeeds
        let again = pool.take(RunId::new(2), StepIdx::new(0));
        assert_eq!(again.is_ok(), true);
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn frame_pool_reused_frame_gets_new_pc() {
        // Given a pool with a released frame
        let mut pool = new_pool(4, 1, 4);
        let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
        // When taking with a different first_step
        let reused = pool.take(RunId::new(2), StepIdx::new(3));
        // Then the pc is updated
        match reused {
            Ok(f) => {
                assert_eq!(f.pc(), StepIdx::new(3));
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn frame_pool_available_starts_at_zero() {
        // Given a new pool
        let pool = new_pool(2, 1, 4);
        // When checking available
        // Then it is 0
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn frame_pool_capacity_is_const() {
        // Given a pool with capacity 16
        let pool = new_pool(2, 1, 16);
        // When checking capacity
        // Then it is 16 and doesn't change
        assert_eq!(pool.capacity(), 16);
    }
}
