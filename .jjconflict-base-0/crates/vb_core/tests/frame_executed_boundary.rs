use vb_core::frame::RunFrame;
use vb_core::{CoreError, RunId, SlotIdx, SlotValue, StepIdx, Taint};

#[test]
fn increment_executed_preserves_frame_at_success_and_overflow_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let mut seed = RunFrame::new(RunId::new(93), StepIdx::ZERO, 2, 2)?;
    seed.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(42), Taint::Secret)?;
    seed.mark_running(StepIdx::ZERO)?;
    seed.set_max_parallel_in_flight(7);
    seed.add_parallel_in_flight(1)?;
    for (initial, expected) in [
        (0, Ok(1)),
        (u64::MAX.saturating_sub(1), Ok(u64::MAX)),
        (u64::MAX, Err(CoreError::StepCounterOverflow)),
    ] {
        let mut checkpoint = seed.checkpoint_v1();
        checkpoint.executed = initial;
        let mut run = RunFrame::from_checkpoint_v1(checkpoint.clone())
            .map_err(|error| format!("checkpoint setup: {error:?}"))?;
        let result = run.increment_executed();
        match expected {
            Ok(executed) => {
                assert_eq!(result, Ok(()));
                checkpoint.executed = executed;
                assert_eq!(run.checkpoint_v1(), checkpoint);
            }
            Err(error) => {
                assert_eq!(result, Err(error));
                assert_eq!(run.checkpoint_v1(), checkpoint);
            }
        }
    }
    Ok(())
}
