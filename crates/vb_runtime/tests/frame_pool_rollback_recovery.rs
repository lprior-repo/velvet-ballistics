use vb_core::{CoreError, RunId, SlotIdx, SlotValue, StepIdx};
use vb_runtime::frame_pool::FramePool;

type TestResult = Result<(), CoreError>;

#[test]
fn rejected_count_and_max_step_preserve_capacity_for_next_valid_take() -> TestResult {
    for invalid in [StepIdx::new(2), StepIdx::new(u16::MAX)] {
        let mut pool = FramePool::new(2, 1, 1)?;
        assert_eq!(
            pool.take(RunId::new(1), invalid).map(|_| ()),
            Err(CoreError::InvalidProgramCounter { step: invalid })
        );
        assert_eq!(pool.available(), 1);
        assert_eq!(pool.capacity(), 1);
        let frame = pool.take(RunId::new(2), StepIdx::new(1))?;
        assert_eq!(frame.run_id(), RunId::new(2));
        assert_eq!(frame.pc(), StepIdx::new(1));
        assert_eq!(pool.available(), 0);
        pool.release(frame);
        assert_eq!(pool.available(), 1);
    }
    Ok(())
}

#[test]
fn repeated_refusals_preserve_the_returned_frame_and_clear_it_on_success() -> TestResult {
    let mut pool = FramePool::new(1, 1, 1)?;
    let mut frame = pool.take(RunId::new(1), StepIdx::ZERO)?;
    frame.write_slot(SlotIdx::ZERO, SlotValue::I64(9))?;
    pool.release(frame);
    for _ in 0..64 {
        assert_eq!(
            pool.take(RunId::new(9), StepIdx::new(1)).map(|_| ()),
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(1)
            })
        );
        assert_eq!(pool.available(), 1);
    }
    let frame = pool.take(RunId::new(3), StepIdx::ZERO)?;
    assert_eq!(frame.run_id(), RunId::new(3));
    assert!(frame.slots_snapshot().iter().all(Option::is_none));
    assert!(pool.is_empty());
    pool.release(frame);
    assert_eq!(pool.available(), 1);
    Ok(())
}

#[test]
fn exhausted_pool_keeps_allocation_error_precedence_over_invalid_step() -> TestResult {
    let mut pool = FramePool::new(1, 1, 1)?;
    let frame = pool.take(RunId::new(1), StepIdx::ZERO)?;
    for first in [StepIdx::ZERO, StepIdx::new(1), StepIdx::new(u16::MAX)] {
        assert_eq!(
            pool.take(RunId::new(2), first).map(|_| ()),
            Err(CoreError::AllocationFailed)
        );
        assert_eq!(pool.available(), 0);
    }
    pool.release(frame);
    assert_eq!(pool.available(), 1);
    Ok(())
}
