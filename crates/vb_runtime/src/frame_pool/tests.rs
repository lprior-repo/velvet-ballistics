//! Tests for frame_pool

use super::{FramePool, MAX_POOL_CAPACITY};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

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
fn take_returns_preallocated_frame() {
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
    assert_eq!(pool.available(), 4);

    let reused = pool.take(RunId::new(2), StepIdx::new(0));
    assert_eq!(reused.map(|f| f.run_id()), Ok(RunId::new(2)));
}

#[test]
fn release_drops_when_at_capacity() {
    let mut pool = new_pool(2, 1, 1);
    let frame1 = pool.take(RunId::new(1), StepIdx::new(0));
    assert_eq!(frame1.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
    // Pool has capacity 1, so second take fails (no fresh alloc)
    let frame2 = pool.take(RunId::new(2), StepIdx::new(0));
    assert!(matches!(
        frame2,
        Err(vb_core::errors::CoreError::AllocationFailed)
    ));
    let frame1 = match frame1 {
        Ok(f) => f,
        Err(_) => return,
    };
    pool.release(frame1);
    assert_eq!(pool.available(), 1);
}

#[test]
fn is_empty_returns_false_for_new_pool() {
    let pool = new_pool(2, 1, 4);
    assert_eq!(pool.is_empty(), false, "pool is pre-allocated with capacity frames");
    assert_eq!(pool.available(), 4);
}

#[test]
fn is_empty_returns_true_after_taking_all() {
    let mut pool = new_pool(2, 1, 4);
    assert_eq!(pool.is_empty(), false);
    for i in 1u64..=4 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        drop(frame);
    }
    assert_eq!(pool.is_empty(), true);
}

#[test]
fn capacity_returns_configured_value() {
    let pool = new_pool(10, 5, 42);
    assert_eq!(pool.capacity(), 42);
    assert_eq!(pool.available(), 42, "pool is pre-allocated");
}

#[test]
fn frame_pool_new_preallocates_capacity_frames() {
    let pool = new_pool(2, 1, 8);
    assert_eq!(pool.capacity(), 8);
    assert_eq!(pool.available(), 8, "all 8 frames pre-allocated at construction");
    assert_eq!(pool.is_empty(), false);
}

#[test]
fn frame_pool_no_runtime_allocation_after_construction() {
    // Given a freshly-constructed pool
    let mut pool = new_pool(2, 1, 4);
    // When taking and releasing 100 frames (rapid cycle)
    for i in 1u64..=100 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(e) => panic!("take should never fail when pool not exhausted: {e}"),
        };
        pool.release(frame);
    }
    // Then available stays at capacity (no fresh allocation happened)
    assert_eq!(pool.available(), 4);
}

#[test]
fn frame_pool_exhausted_take_returns_error() {
    // Given a pool with capacity 2
    let mut pool = new_pool(2, 1, 2);
    // When taking all 2 frames without releasing
    let _f1 = pool.take(RunId::new(1), StepIdx::new(0)).expect("first take");
    let _f2 = pool.take(RunId::new(2), StepIdx::new(0)).expect("second take");
    // Then a third take fails (no fresh allocation as fallback)
    let result = pool.take(RunId::new(3), StepIdx::new(0));
    assert!(matches!(
        result,
        Err(vb_core::errors::CoreError::AllocationFailed)
    ));
}

#[test]
fn frame_pool_new_creates_pool_with_capacity() {
    // Given a new pool with capacity 8
    let pool = new_pool(2, 1, 8);
    // When checking capacity and availability
    // Then capacity is 8 and available is 8 (all pre-allocated)
    assert_eq!(pool.capacity(), 8);
    assert_eq!(pool.available(), 8);
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
    assert_eq!(pool.available(), 4);
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
    // Then the pool is empty
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
    assert_eq!(pool.available(), 4);
}

#[test]
fn frame_pool_release_then_acquire_yields_recycled_frame() {
    // Given a pool where a frame has been released
    let mut pool = new_pool(2, 1, 4);
    let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    pool.release(frame);
    // When acquiring again
    let recycled = pool.take(RunId::new(2), StepIdx::new(0));
    // Then the recycled frame is reinitialized for the new run
    match recycled {
        Ok(f) => {
            assert_eq!(f.run_id(), RunId::new(2));
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
    assert_eq!(pool.available(), 4);
    let recycled = pool.take(RunId::new(3), StepIdx::new(0));
    // Then the recycled frame is available
    assert_eq!(recycled.is_ok(), true);
    assert_eq!(pool.available(), 3);
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
fn frame_pool_take_returns_preallocated_frame_when_no_recycled() {
    // Given a fresh pool (all frames pre-allocated)
    let mut pool = new_pool(2, 1, 4);
    assert_eq!(pool.available(), 4);
    // When taking a frame
    let frame = pool.take(RunId::new(42), StepIdx::new(0));
    // Then a pre-allocated frame is returned with the correct run_id
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
fn frame_pool_reused_frame_clears_prior_state() {
    let mut pool = new_pool(4, 2, 4);
    let mut frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    assert_eq!(frame.mark_succeeded(StepIdx::new(1)), Ok(()));
    assert_eq!(
        frame.write_slot(SlotIdx::ZERO, vb_core::SlotValue::Bool(true)),
        Ok(())
    );
    assert_eq!(
        frame.write_taint(SlotIdx::ZERO, vb_core::Taint::Secret),
        Ok(())
    );
    assert_eq!(frame.increment_executed(), Ok(()));
    pool.release(frame);

    let reused = match pool.take(RunId::new(2), StepIdx::new(3)) {
        Ok(f) => f,
        Err(_) => return,
    };

    assert_eq!(reused.run_id(), RunId::new(2));
    assert_eq!(reused.pc(), StepIdx::new(3));
    assert_eq!(reused.executed(), 0);
    assert_eq!(
        reused.step_state(StepIdx::new(1)),
        Ok(vb_core::StepState::Pending)
    );
    assert_eq!(
        reused.read_slot(SlotIdx::ZERO),
        Err(vb_core::CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
    assert_eq!(
        reused.read_taint(SlotIdx::ZERO),
        Err(vb_core::CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

#[test]
fn frame_pool_available_starts_at_capacity() {
    // Given a new pool
    let pool = new_pool(2, 1, 4);
    // When checking available
    // Then it equals capacity (all pre-allocated)
    assert_eq!(pool.available(), 4);
}

#[test]
fn frame_pool_capacity_is_const() {
    // Given a pool with capacity 16
    let pool = new_pool(2, 1, 16);
    // When checking capacity
    // Then it is 16 and doesn't change
    assert_eq!(pool.capacity(), 16);
}

// =======================================================================
// Adversarial BDD tests — frame_pool
// =======================================================================

#[test]
fn frame_pool_exhausted_take_returns_error_no_fresh_alloc() {
    // Given a pool with capacity 2
    let mut pool = new_pool(2, 1, 2);
    // When taking 2 frames without releasing
    let f1 = pool.take(RunId::new(1), StepIdx::new(0));
    let f2 = pool.take(RunId::new(2), StepIdx::new(0));
    // Then both succeed (we have 2 pre-allocated frames)
    assert!(f1.is_ok());
    assert!(f2.is_ok());
    // When taking a 3rd frame (no recycled frames, no fresh allocation)
    let f3 = pool.take(RunId::new(3), StepIdx::new(0));
    // Then it fails with AllocationFailed (no fresh allocation fallback)
    assert!(matches!(
        f3,
        Err(vb_core::errors::CoreError::AllocationFailed)
    ));
}

#[test]
fn frame_pool_release_wrong_dimension_frame_is_silently_dropped() {
    // Given a pool configured for (step_count=2, slot_count=1) pre-allocated to capacity
    let mut pool_a = new_pool(2, 1, 4);
    assert_eq!(pool_a.available(), 4);
    // And a different pool for (step_count=4, slot_count=2)
    let mut pool_b = new_pool(4, 2, 4);
    assert_eq!(pool_b.available(), 4);
    // When taking a frame from pool_b and releasing it into pool_a
    let frame = match pool_b.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    assert_eq!(pool_a.available(), 4);
    pool_a.release(frame);
    // Then pool_a is unchanged — mismatched dimensions are silently dropped
    assert_eq!(pool_a.available(), 4);
    assert_eq!(pool_b.available(), 3);
}

#[test]
fn frame_pool_release_never_panics_or_overflows_capacity() {
    // Given a pool with capacity 1
    let mut pool = new_pool(2, 1, 1);
    // When releasing 5 frames (all from same pool dims)
    for i in 1u64..=5 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
    }
    // Then the pool stays at its capacity limit
    assert_eq!(pool.available(), 1);
}

#[test]
fn frame_pool_zero_step_count_rejects_construction() {
    // Given a pool with zero step count, construction itself fails because
    // RunFrame::new rejects step_count=0 (pre-allocation fails)
    let result = FramePool::new(0, 1, 4);
    // Then FramePool::new returns the error directly
    match result {
        Err(vb_core::errors::CoreError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "step_count_zero");
        }
        Err(other) => {
            let msg = format!("unexpected error: {other:?}");
            panic!("{msg}");
        }
        Ok(_) => {
            panic!("expected error for step_count=0");
        }
    }
}

#[test]
fn frame_pool_reused_frame_has_clean_taint_state() {
    // Given a pool with a frame that had tainted slots
    let mut pool = new_pool(4, 2, 4);
    let mut frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    assert_eq!(
        frame.write_slot(SlotIdx::ZERO, vb_core::SlotValue::I64(42)),
        Ok(())
    );
    assert_eq!(
        frame.write_taint(SlotIdx::ZERO, vb_core::Taint::Secret),
        Ok(())
    );
    pool.release(frame);
    // When reusing the frame
    let reused = match pool.take(RunId::new(2), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Then the slot is uninitialized (taint cannot be read without a value)
    assert_eq!(
        reused.read_taint(SlotIdx::ZERO),
        Err(vb_core::CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );
}

// =======================================================================
// Adversarial BDD tests - frame_pool attack vectors
// =======================================================================

#[test]
fn frame_pool_take_release_take_preserves_pool_consistency_under_rapid_cycle() {
    // Given a pool with capacity 1
    let mut pool = new_pool(2, 1, 1);
    // When rapidly cycling take/release 10 times
    for i in 1u64..=10 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
    }
    // Then the pool stays at capacity 1 and the last run_id is correct
    assert_eq!(pool.available(), 1);
    let reused = pool.take(RunId::new(99), StepIdx::new(0));
    match reused {
        Ok(f) => {
            assert_eq!(f.run_id(), RunId::new(99));
            assert_eq!(f.pc(), StepIdx::new(0));
        }
        Err(_) => {
            assert!(false);
        }
    }
}

#[test]
fn frame_pool_release_after_release_at_capacity_drops_all_extras() {
    // Given a pool with capacity 2 — pre-allocated means only 2 frames exist
    let mut pool = new_pool(2, 1, 2);
    // When taking all 2 (the only pre-allocated ones)
    let frames: Vec<RunFrame> = (1..=2u64)
        .filter_map(|i| pool.take(RunId::new(i), StepIdx::new(0)).ok())
        .collect();
    assert_eq!(frames.len(), 2);
    // When releasing all 2 frames
    for frame in frames {
        pool.release(frame);
    }
    // Then the pool has exactly 2 (its capacity)
    assert_eq!(pool.available(), 2);
    assert_eq!(pool.capacity(), 2);
}

#[test]
fn frame_pool_zero_slot_count_allocates_successfully() {
    // Given a pool with 0 slots
    let mut pool = new_pool(2, 0, 4);
    // When taking a frame
    let result = pool.take(RunId::new(1), StepIdx::new(0));
    // Then it succeeds with correct run_id
    assert_eq!(result.as_ref().map(|f| f.run_id()), Ok(RunId::new(1)));
}

#[test]
fn frame_pool_large_capacity_at_boundary_succeeds() {
    // Given capacity at exactly MAX_POOL_CAPACITY (4096)
    let result = FramePool::new(2, 1, 4096);
    // Then it succeeds
    assert_eq!(result.as_ref().map(|_| ()), Ok(()));
    let pool = match result {
        Ok(p) => p,
        Err(_) => return,
    };
    assert_eq!(pool.capacity(), 4096);
    assert_eq!(pool.available(), 4096, "all frames pre-allocated");
}

#[test]
fn frame_pool_reused_frame_step_count_matches_pool_config() {
    // Given a pool with step_count=8
    let mut pool = new_pool(8, 2, 4);
    let frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    assert_eq!(frame.step_count(), 8);
    pool.release(frame);
    // When taking again
    let reused = match pool.take(RunId::new(2), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Then the reused frame still has step_count=8
    assert_eq!(reused.step_count(), 8);
    assert_eq!(reused.slot_count(), 2);
}

#[test]
fn frame_pool_concurrent_dimension_pools_do_not_interfere() {
    // Given two pools with different dimensions, both pre-allocated
    let mut pool_a = new_pool(2, 1, 4);
    let mut pool_b = new_pool(4, 2, 4);
    assert_eq!(pool_a.available(), 4);
    assert_eq!(pool_b.available(), 4);
    // When releasing a frame from pool_b into pool_a
    let frame_b = match pool_b.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    pool_a.release(frame_b);
    // Then pool_a is unchanged (dimension mismatch silently dropped)
    assert_eq!(pool_a.available(), 4);
    assert_eq!(pool_b.available(), 3);
    // And pool_a can still release its own frame type
    let frame_a = match pool_a.take(RunId::new(2), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    pool_a.release(frame_a);
    assert_eq!(pool_a.available(), 4);
}

// =====================================================================
// BLACKHAT security review: frame_pool additional adversarial tests
// =====================================================================

#[test]
fn frame_pool_take_returns_error_when_exhausted() {
    // Given a pool with capacity 1 (pre-allocated, so only 1 frame exists)
    let mut pool = new_pool(4, 2, 1);
    // When taking the one frame
    let frame = pool.take(RunId::new(1), StepIdx::new(0));
    assert!(frame.is_ok());
    // Then taking a second frame returns AllocationFailed (no fresh alloc fallback)
    let second = pool.take(RunId::new(2), StepIdx::new(0));
    assert!(matches!(
        second,
        Err(vb_core::errors::CoreError::AllocationFailed)
    ));
}

#[test]
fn frame_pool_reused_frame_has_zero_executed_count() {
    // Given a pool with a frame that had non-zero executed count
    let mut pool = new_pool(4, 2, 4);
    let mut frame = match pool.take(RunId::new(1), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    for _ in 0..10 {
        let _ = frame.increment_executed();
    }
    assert_eq!(frame.executed(), 10);
    pool.release(frame);

    // When taking the reused frame
    let reused = match pool.take(RunId::new(2), StepIdx::new(0)) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Then executed count must be zero
    assert_eq!(
        reused.executed(),
        0,
        "reused frame must have clean executed counter"
    );
}

#[test]
fn frame_pool_capacity_one_never_exceeds_limit() {
    // Given a pool with capacity 1
    let mut pool = new_pool(2, 1, 1);
    // When releasing 100 frames
    for i in 1u64..=100 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
    }
    // Then the pool never exceeds capacity 1
    assert_eq!(pool.available(), 1);
    assert_eq!(pool.capacity(), 1);
}
