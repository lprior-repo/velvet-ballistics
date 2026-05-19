#![forbid(unsafe_code)]
//! Body-re-entry proof harnesses for loop primitives.
//!
//! These harnesses verify that the step state machine correctly handles
//! re-entry into loop bodies after a previous iteration has completed.
//!
//! Bug: When a loop body step completes (Succeeded) and control returns
//! to the loop primitive (for_each_next, reduce_next, collect_next,
//! repeat_attempt, repeat_check), the step is still in Succeeded state.
//! The loop primitive needs to transition Succeeded→Pending before
//! re-entering the body, but this transition was missing.

#[cfg(kani)]
pub mod reentry_harnesses {
    use vb_core::errors::EngineError;
    use vb_core::frame::{RunFrame, StepState};
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::value::SlotValue;
    use vb_core::value_store::ValueStore;

    use crate::primitives::for_each::for_each_next;
    use crate::primitives::reduce::reduce_next;
    use crate::primitives::collect::{collect_next, collect_page, CollectStates};
    use crate::primitives::repeat::{repeat_attempt, repeat_check};

    fn fresh_frame(step_count: u16, slot_count: u16) -> RunFrame {
        RunFrame::new(
            vb_core::ids::RunId::new(1),
            StepIdx::ZERO,
            step_count,
            slot_count,
        )
        .unwrap()
    }

    fn list_in_slot(run: &mut RunFrame, store: &mut ValueStore, slot: SlotIdx, items: Vec<SlotValue>) {
        let id = store.insert_list(items.into_boxed_slice()).unwrap();
        run.write_slot(slot, SlotValue::List(id)).unwrap();
    }

    fn step_state_from_u8(v: u8) -> StepState {
        match v % 8 {
            0 => StepState::Pending,
            1 => StepState::Running,
            2 => StepState::Succeeded,
            3 => StepState::Failed,
            4 => StepState::Skipped,
            5 => StepState::Waiting,
            6 => StepState::Asking,
            _ => StepState::Cancelled,
        }
    }

    /// K-REENTRY-FE-1: for_each_next re-entry harness.
    /// Models the scenario where a for_each body step completes and
    /// for_each_next is called again to process the next item.
    ///
    /// Uses kani::any::<StepState>() to test all possible body step states,
    /// not just Succeeded. The kani::cover statements verify specific
    /// state transitions are explored.
    #[kani::proof]
    fn for_each_next_reentry() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();

        let iterator_slot = SlotIdx::new(0);
        let output_slot = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(7), SlotValue::I64(8)]);

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "for_each_next re-entry with Succeeded body state"
        );
        kani::cover!(
            body_state == StepState::Pending,
            "for_each_next re-entry with Pending body state"
        );
        kani::cover!(
            body_state == StepState::Running,
            "for_each_next re-entry with Running body state"
        );
        kani::cover!(
            body_state == StepState::Failed,
            "for_each_next re-entry with Failed body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = for_each_next(
            &mut run,
            &mut store,
            iterator_slot,
            body,
            done,
            Some(output_slot),
        );

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after for_each_next");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "for_each_next re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }

    /// K-REENTRY-RD-1: reduce_next re-entry harness.
    /// Same pattern as for_each_next but for reduce primitive.
    #[kani::proof]
    fn reduce_next_reentry() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();

        let iterator_slot = SlotIdx::new(0);
        let accumulator = SlotIdx::new(1);
        let output_slot = SlotIdx::new(2);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(5), SlotValue::I64(6)]);

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "reduce_next re-entry with Succeeded body state"
        );
        kani::cover!(
            body_state == StepState::Pending,
            "reduce_next re-entry with Pending body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            accumulator,
            body,
            done,
            Some(output_slot),
        );

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after reduce_next");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "reduce_next re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }

    /// K-REENTRY-CL-1: collect_next re-entry harness.
    /// Tests that collect_next can re-enter after a page body completes.
    #[kani::proof]
    fn collect_next_reentry() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();
        let mut states = CollectStates::new();

        let collector_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        let source = SlotIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(30),
            SlotValue::I64(40),
        ]);

        let start_result = crate::primitives::collect::collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector_slot),
            None,
        );

        if start_result.is_err() {
            return;
        }

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "collect_next re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = collect_next(
            &mut run,
            &mut store,
            &mut states,
            collector_slot,
            body,
            done,
        );

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after collect_next");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "collect_next re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }

    /// K-REENTRY-CP-1: collect_page re-entry harness.
    /// Tests that collect_page can be called when body step is in any state.
    #[kani::proof]
    fn collect_page_reentry() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();
        let mut states = CollectStates::new();

        let collector_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        let source = SlotIdx::new(3);
        list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(10), SlotValue::I64(20)]);

        let _ = crate::primitives::collect::collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            100,
            2,
            body,
            done,
            Some(collector_slot),
            None,
        );

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "collect_page re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = collect_page(
            &mut run,
            &mut store,
            &mut states,
            collector_slot,
            body,
            done,
        );

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after collect_page");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "collect_page re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }

    /// K-REENTRY-RPA-1: repeat_attempt re-entry harness.
    /// Tests that repeat_attempt can be called when body step is in any state.
    #[kani::proof]
    fn repeat_attempt_reentry() {
        let mut run = fresh_frame(4, 8);

        let attempt_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        let packed: i64 = (3_i64 << 32) | 1_i64;
        run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_attempt re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = repeat_attempt(&mut run, attempt_slot, body, done);

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after repeat_attempt");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "repeat_attempt re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }

    /// K-REENTRY-RPC-1: repeat_check re-entry harness.
    /// Tests that repeat_check can be called when body step is in any state.
    #[kani::proof]
    fn repeat_check_reentry() {
        let mut run = fresh_frame(4, 8);

        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);

        let packed: i64 = (3_i64 << 32) | 1_i64;
        run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_check re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => { run.mark_pending(body_step).unwrap(); }
            StepState::Running => { run.mark_running(body_step).unwrap(); }
            StepState::Succeeded => { run.mark_succeeded(body_step).unwrap(); }
            StepState::Failed => { run.mark_failed(body_step).unwrap(); }
            StepState::Skipped => { run.mark_skipped(body_step).unwrap(); }
            StepState::Waiting => { run.mark_waiting(body_step).unwrap(); }
            StepState::Asking => { run.mark_asking(body_step).unwrap(); }
            StepState::Cancelled => { run.mark_cancelled(body_step).unwrap(); }
        }

        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(next_body),
            StepIdx::ZERO,
        );

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(state.is_ok(), "step_state should be readable after repeat_check");
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "repeat_check re-entry should not fail with invalid_state_transition"
                );
            }
            _ => {}
        }
    }
}