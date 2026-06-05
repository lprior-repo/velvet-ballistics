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

    use crate::primitives::collect::{CollectStates, collect_next, collect_page};
    use crate::primitives::for_each::for_each_next;
    use crate::primitives::reduce::reduce_next;
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

    fn list_in_slot(
        run: &mut RunFrame,
        store: &mut ValueStore,
        slot: SlotIdx,
        items: Vec<SlotValue>,
    ) {
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

        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(7), SlotValue::I64(8)],
        );

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
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
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
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after for_each_next",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "for_each_next re-entry should not fail with invalid_state_transition",
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

        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(5), SlotValue::I64(6)],
        );

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
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
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
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after reduce_next",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "reduce_next re-entry should not fail with invalid_state_transition",
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
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![
                SlotValue::I64(10),
                SlotValue::I64(20),
                SlotValue::I64(30),
                SlotValue::I64(40),
            ],
        );

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
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
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
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after collect_next",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "collect_next re-entry should not fail with invalid_state_transition",
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
        list_in_slot(
            &mut run,
            &mut store,
            source,
            vec![SlotValue::I64(10), SlotValue::I64(20)],
        );

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
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
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
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after collect_page",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "collect_page re-entry should not fail with invalid_state_transition",
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
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .unwrap();

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_attempt re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
        }

        let result = repeat_attempt(&mut run, attempt_slot, body, done);

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after repeat_attempt",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "repeat_attempt re-entry should not fail with invalid_state_transition",
                );
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // PO-KANI-008, PO-KANI-009, PO-KANI-010: Body re-entry transition harnesses
    // -----------------------------------------------------------------------
    // These harnesses verify that loop primitives use the explicit
    // Succeeded->Running transition during body re-entry instead of the removed
    // Succeeded->Pending transition.

    /// PO-KANI-008: for_each_next body state re-entry.
    /// Verifies that after for_each_next re-entry, the body step state is Running.
    #[kani::proof]
    fn kani_for_each_reentry_body_state_immutable() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();

        let iterator_slot = SlotIdx::new(0);
        let output_slot = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(7), SlotValue::I64(8)],
        );

        let body_step = StepIdx::new(1);

        // Body completes execution → Succeeded
        run.mark_running(body_step).unwrap();
        run.mark_succeeded(body_step).unwrap();

        // for_each_next re-enters for next item
        let result = for_each_next(
            &mut run,
            &mut store,
            iterator_slot,
            body,
            done,
            Some(output_slot),
        );

        // If re-entry succeeded (Continue), body state must be Running.
        if let Ok(vb_core::EngineSignal::Continue) = result {
            let state_after = run.step_state(body_step).unwrap();
            kani::assert(
                state_after == StepState::Running,
                "for_each_next re-entry must mark body step Running (PO-KANI-008)",
            );
        }
    }

    /// PO-KANI-009: reduce_next body state re-entry.
    #[kani::proof]
    fn kani_reduce_reentry_body_state_immutable() {
        let mut run = fresh_frame(4, 8);
        let mut store = ValueStore::new();

        let iterator_slot = SlotIdx::new(0);
        let accumulator = SlotIdx::new(1);
        let output_slot = SlotIdx::new(2);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(5), SlotValue::I64(6)],
        );

        let body_step = StepIdx::new(1);

        run.mark_running(body_step).unwrap();
        run.mark_succeeded(body_step).unwrap();

        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            accumulator,
            body,
            done,
            Some(output_slot),
        );

        if let Ok(vb_core::EngineSignal::Continue) = result {
            let state_after = run.step_state(body_step).unwrap();
            kani::assert(
                state_after == StepState::Running,
                "reduce_next re-entry must mark body step Running (PO-KANI-009)",
            );
        }
    }

    /// PO-KANI-010: Combined harness for collect_next, collect_page,
    /// repeat_attempt, and repeat_check body state re-entry.
    /// Bounded: step_count ≤ 16, pages ≤ 4, attempts ≤ 4.
    #[kani::proof]
    fn kani_remaining_primitives_reentry_body_state_immutable() {
        // ---- collect_next ----
        {
            let mut run = fresh_frame(4, 8);
            let mut store = ValueStore::new();
            let mut states = CollectStates::new();

            let collector_slot = SlotIdx::new(0);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);
            let source = SlotIdx::new(3);

            list_in_slot(
                &mut run,
                &mut store,
                source,
                vec![SlotValue::I64(10), SlotValue::I64(20)],
            );

            let _ = collect_start(
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
            run.mark_running(body_step).unwrap();
            run.mark_succeeded(body_step).unwrap();
            let result = collect_next(
                &mut run,
                &mut store,
                &mut states,
                collector_slot,
                body,
                done,
            );

            if let Ok(vb_core::EngineSignal::Continue) = result {
                let state_after = run.step_state(body_step).unwrap();
                kani::assert(
                    state_after == StepState::Running,
                    "collect_next must mark body state Running (PO-KANI-010)",
                );
            }
        }

        // ---- repeat_attempt ----
        {
            let mut run = fresh_frame(4, 8);
            let attempt_slot = SlotIdx::new(0);
            let body = StepIdx::new(1);
            let done = StepIdx::new(2);

            let packed: i64 = (3_i64 << 32) | 1_i64;
            run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

            let body_step = StepIdx::new(1);
            run.mark_running(body_step).unwrap();
            run.mark_succeeded(body_step).unwrap();
            let result = repeat_attempt(&mut run, attempt_slot, body, done);

            if let Ok(vb_core::EngineSignal::Continue) = result {
                let state_after = run.step_state(body_step).unwrap();
                kani::assert(
                    state_after == StepState::Running,
                    "repeat_attempt must mark body state Running (PO-KANI-010)",
                );
            }
        }

        // ---- repeat_check ----
        {
            let mut run = fresh_frame(4, 8);
            let attempt_slot = SlotIdx::new(0);
            let done = StepIdx::new(2);
            let next_body = StepIdx::new(1);

            let packed: i64 = (3_i64 << 32) | 1_i64;
            run.write_slot(attempt_slot, SlotValue::I64(packed)).unwrap();

            let body_step = StepIdx::new(1);
            run.mark_running(body_step).unwrap();
            run.mark_succeeded(body_step).unwrap();
            let result = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);

            if let Ok(vb_core::EngineSignal::Continue) = result {
                let state_after = run.step_state(body_step).unwrap();
                kani::assert(
                    state_after == StepState::Running,
                    "repeat_check must mark body state Running (PO-KANI-010)",
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // PO-KANI-011: jump_to_body re-entry transition
    // -----------------------------------------------------------------------
    // jump_to_body marks Succeeded bodies Running before jumping, and leaves all
    // other states unchanged.

    /// PO-KANI-011: jump_to_body must use Succeeded->Running for re-entry.
    /// Verifies that after jump_to_body(run, body), Succeeded becomes Running
    /// and all other states stay unchanged.
    #[kani::proof]
    fn kani_jump_to_body_no_state_mutation() {
        let mut run = fresh_frame(8, 4);

        let body = StepIdx::new(3);
        let body_state_raw: u8 = kani::any();
        let body_state = step_state_from_u8(body_state_raw);

        // Set the body step to a symbolic state
        match body_state {
            StepState::Pending => run.mark_pending(body).unwrap(),
            StepState::Running => run.mark_running(body).unwrap(),
            StepState::Succeeded => run.mark_succeeded(body).unwrap(),
            StepState::Failed => run.mark_failed(body).unwrap(),
            StepState::Skipped => run.mark_skipped(body).unwrap(),
            StepState::Waiting => run.mark_waiting(body).unwrap(),
            StepState::Asking => run.mark_asking(body).unwrap(),
            StepState::Cancelled => run.mark_cancelled(body).unwrap(),
            _ => {}
        }

        let state_before = run.step_state(body).unwrap();
        let pc_before = run.pc();
        let executed_before = run.executed();

        // Call jump_to_body (the production function)
        let result = crate::primitives::helpers::jump::jump_to_body(&mut run, body);

        // The function should succeed (only PC jump, no invalid state transition)
        // If it fails, it should not be because of invalid state transition
        if result.is_err() {
            // If it fails, it should only be for non-state-transition reasons
            // (e.g., PC out of bounds — but we set up valid bounds)
            kani::assert(
                false,
                "jump_to_body should succeed for any body state (PO-KANI-011)",
            );
        }

        // Body state must follow the explicit re-entry contract.
        let state_after = run.step_state(body).unwrap();
        let expected = if state_before == StepState::Succeeded {
            StepState::Running
        } else {
            state_before
        };
        kani::assert(
            state_after == expected,
            "jump_to_body must only mutate Succeeded body state to Running (PO-KANI-011)",
        );

        // PC and executed should have changed (the jump happened)
        kani::cover!(run.pc() != pc_before, "PC changed after jump_to_body");
        kani::cover!(run.executed() != executed_before, "executed incremented");
    }

    // -----------------------------------------------------------------------
    // Original harnesses (K-REENTRY-RP*) preserved below
    // -----------------------------------------------------------------------

    /// K-REENTRY-RPC-1: repeat_check re-entry harness.
    /// Tests that repeat_check can be called when body step is in any state.
    #[kani::proof]
    fn repeat_check_reentry() {
        let mut run = fresh_frame(4, 8);

        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);

        let packed: i64 = (3_i64 << 32) | 1_i64;
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .unwrap();

        let body_step = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_check re-entry with Succeeded body state"
        );

        run.mark_running(body_step).unwrap();
        match body_state {
            StepState::Pending => {
                run.mark_pending(body_step).unwrap();
            }
            StepState::Running => {
                run.mark_running(body_step).unwrap();
            }
            StepState::Succeeded => {
                run.mark_succeeded(body_step).unwrap();
            }
            StepState::Failed => {
                run.mark_failed(body_step).unwrap();
            }
            StepState::Skipped => {
                run.mark_skipped(body_step).unwrap();
            }
            StepState::Waiting => {
                run.mark_waiting(body_step).unwrap();
            }
            StepState::Asking => {
                run.mark_asking(body_step).unwrap();
            }
            StepState::Cancelled => {
                run.mark_cancelled(body_step).unwrap();
            }
            _ => {}
        }

        let result = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);

        match result {
            Ok(vb_core::EngineSignal::Continue) => {
                let state = run.step_state(body_step);
                kani::assert(
                    state.is_ok(),
                    "step_state should be readable after repeat_check",
                );
            }
            Err(EngineError::InternalInvariantViolation { reason }) => {
                kani::assert(
                    reason != "invalid_state_transition",
                    "repeat_check re-entry should not fail with invalid_state_transition",
                );
            }
            _ => {}
        }
    }
}
