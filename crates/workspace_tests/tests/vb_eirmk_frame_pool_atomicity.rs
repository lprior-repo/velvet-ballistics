//! Public frame-pool rejection atomicity and recovery behavior.

use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::frame_pool::FramePool;

fn assert_invalid_step(
    pool: &mut FramePool,
    step: StepIdx,
    expected_available: usize,
    expected_capacity: usize,
) {
    assert_eq!(
        pool.take(RunId::new(1), step),
        Err(CoreError::InvalidProgramCounter { step })
    );
    assert_eq!(pool.available(), expected_available);
    assert_eq!(pool.capacity(), expected_capacity);
}

#[test]
fn invalid_step_preserves_single_frame_and_valid_take_recovers() -> Result<(), CoreError> {
    let mut pool = FramePool::new(2, 1, 1)?;
    assert_invalid_step(&mut pool, StepIdx::new(2), 1, 1);
    let frame = pool.take(RunId::new(2), StepIdx::new(1))?;
    assert_eq!(frame.run_id(), RunId::new(2));
    assert_eq!(frame.pc(), StepIdx::new(1));
    pool.release(frame);
    assert_eq!(pool.available(), 1);
    Ok(())
}

#[test]
fn repeated_invalid_steps_preserve_capacity_and_exhaustion_is_exact() -> Result<(), CoreError> {
    let mut pool = FramePool::new(2, 1, 2)?;
    for step in [StepIdx::new(2), StepIdx::MAX, StepIdx::new(2)] {
        assert_invalid_step(&mut pool, step, 2, 2);
    }
    assert_two_frame_exhaustion(&mut pool)?;
    Ok(())
}

fn assert_two_frame_exhaustion(pool: &mut FramePool) -> Result<(), CoreError> {
    let first = pool.take(RunId::new(10), StepIdx::ZERO)?;
    let second = pool.take(RunId::new(11), StepIdx::new(1))?;
    assert_eq!(
        (first.run_id(), first.pc()),
        (RunId::new(10), StepIdx::ZERO)
    );
    assert_eq!(
        (second.run_id(), second.pc()),
        (RunId::new(11), StepIdx::new(1))
    );
    assert_eq!(pool.available(), 0);
    assert_eq!(
        pool.take(RunId::new(12), StepIdx::ZERO),
        Err(CoreError::AllocationFailed)
    );
    pool.release(first);
    pool.release(second);
    assert_eq!(pool.available(), 2);
    Ok(())
}

#[test]
fn exhaustion_precedes_invalid_step_and_release_restores_availability() -> Result<(), CoreError> {
    let mut pool = FramePool::new(2, 1, 1)?;
    let held = pool.take(RunId::new(1), StepIdx::ZERO)?;
    assert_eq!(
        pool.take(RunId::new(2), StepIdx::MAX),
        Err(CoreError::AllocationFailed)
    );
    assert_eq!(pool.available(), 0);
    assert_eq!(pool.capacity(), 1);
    pool.release(held);
    let recovered = pool.take(RunId::new(3), StepIdx::new(1))?;
    assert_eq!(recovered.run_id(), RunId::new(3));
    assert_eq!(recovered.pc(), StepIdx::new(1));
    pool.release(recovered);
    assert_eq!(pool.available(), 1);
    Ok(())
}

#[test]
fn invalid_step_and_recycle_preserve_full_fresh_frame_state() -> Result<(), CoreError> {
    let mut pool = FramePool::new(2, 1, 1)?;
    let mut populated = pool.take(RunId::new(7), StepIdx::ZERO)?;
    populated.mark_succeeded(StepIdx::new(1))?;
    populated.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(7), Taint::Secret)?;
    populated.increment_executed()?;
    pool.release(populated);
    assert_invalid_step(&mut pool, StepIdx::new(2), 1, 1);
    let recycled = pool.take(RunId::new(9), StepIdx::new(1))?;
    let fresh = RunFrame::new(RunId::new(9), StepIdx::new(1), 2, 1)?;
    assert_eq!(recycled, fresh);
    pool.release(recycled);
    assert_eq!(pool.available(), 1);
    Ok(())
}
