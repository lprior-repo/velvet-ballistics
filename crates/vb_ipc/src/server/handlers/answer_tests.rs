use super::{AnswerTicketSelector, decode_answer_selector};
use vb_core::ids::{RunId, SlotIdx, StepIdx};

#[test]
fn answer_ticket_decoder_preserves_step_and_slot_boundaries() {
    assert!(
        matches!(decode_answer_selector(0), Some(AnswerTicketSelector::Step(step)) if step == StepIdx::ZERO)
    );
    assert!(
        matches!(decode_answer_selector(65_535), Some(AnswerTicketSelector::Step(step)) if step.get() == u16::MAX)
    );
    assert!(
        matches!(decode_answer_selector(65_536), Some(AnswerTicketSelector::Slot(slot)) if slot == SlotIdx::ZERO)
    );
    assert!(
        matches!(decode_answer_selector(131_071), Some(AnswerTicketSelector::Slot(slot)) if slot.get() == u16::MAX)
    );
    assert_eq!(decode_answer_selector(131_072), None);
    assert_eq!(decode_answer_selector(u64::from(u32::MAX)), None);
}

#[test]
fn answer_slot_for_unowned_run_is_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(
        std::num::NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
    )
    .expect("valid test configuration");
    let run = RunId::new(404);
    let payload = postcard::to_allocvec(&crate::IpcPayload::AnswerAsk {
        run_id: run,
        ticket: 65_538,
        answer: postcard::to_allocvec(&vb_core::value::SlotValue::I64(8))?,
        taint: None,
    })?;
    let counters = runtime.counters_snapshot();
    let queue_depth = runtime.pending_boundary_snapshot(64).command_queue_depth();
    assert_eq!(
        super::handle_answer_ask(&payload, &mut runtime),
        crate::server::IpcResponse::BadRequest
    );
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot(), counters);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        queue_depth
    );
    Ok(())
}
