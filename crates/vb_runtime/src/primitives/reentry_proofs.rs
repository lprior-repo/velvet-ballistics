#![forbid(unsafe_code)]
//! Body-re-entry proof harnesses for loop primitives.
//!
//! These harnesses verify that the step state machine correctly handles
//! re-entry into loop bodies after a previous iteration has completed.
//!
//! Body re-entry uses the explicit Succeeded -> Pending admission path
//! in `RunFrame::mark_pending` before `mark_running` (Pending -> Running).
//! No direct Succeeded -> Running edge is admitted; terminal states are
//! absorbing per the master contract.

#[cfg(kani)]
pub mod reentry_harnesses {
    use vb_core::errors::EngineError;
    use vb_core::frame::{RunFrame, StepState};
    use vb_core::ids::{ListId, SlotIdx, StepIdx};
    use vb_core::value::SlotValue;
    use vb_core::value_store::ValueStore;

    use crate::primitives::collect::{CollectStates, collect_page};

    fn fresh_frame(step_count: u16, slot_count: u16) -> Option<RunFrame> {
        match RunFrame::new(
            vb_core::ids::RunId::new(1),
            StepIdx::ZERO,
            step_count,
            slot_count,
        ) {
            Ok(frame) => Some(frame),
            Err(_) => {
                kani::assume(false);
                None
            }
        }
    }

    fn assume_ok<T, E>(result: Result<T, E>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(_) => {
                kani::assume(false);
                None
            }
        }
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

    fn apply_body_state(run: &mut RunFrame, body: StepIdx, state: StepState) -> bool {
        match state {
            StepState::Pending => assume_ok(run.mark_pending(body)).is_some(),
            StepState::Running => true,
            StepState::Succeeded => assume_ok(run.mark_succeeded(body)).is_some(),
            StepState::Failed => assume_ok(run.mark_failed(body)).is_some(),
            StepState::Skipped => assume_ok(run.mark_skipped(body)).is_some(),
            StepState::Waiting => assume_ok(run.mark_waiting(body)).is_some(),
            StepState::Asking => assume_ok(run.mark_asking(body)).is_some(),
            StepState::Cancelled => assume_ok(run.mark_cancelled(body)).is_some(),
            _ => true,
        }
    }

    fn continue_state_readable(
        result: &Result<vb_core::EngineSignal, EngineError>,
        run: &RunFrame,
        body: StepIdx,
    ) -> bool {
        if let Ok(vb_core::EngineSignal::Continue) = result {
            return run.step_state(body).is_ok();
        }
        true
    }

    fn is_invalid_transition(result: &Result<vb_core::EngineSignal, EngineError>) -> bool {
        if let Err(EngineError::InternalInvariantViolation { reason }) = result {
            return *reason == "invalid_state_transition";
        }
        false
    }

    /// K-REENTRY-FE-1: for_each_next re-entry harness.
    #[kani::proof]
    fn for_each_next_reentry() {
        let Some(mut run) = fresh_frame(3, 1) else {
            return;
        };
        let body = StepIdx::new(1);

        let body_state = step_state_from_u8(kani::any());
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
        if !run.kani_harness_set_step_state(body, body_state) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "for_each_next re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            kani::assert(false, "body step must remain readable after for_each_next");
            return;
        };
        let expected = if body_state == StepState::Succeeded {
            StepState::Running
        } else {
            body_state
        };
        kani::assert(
            state_after == expected,
            "for_each_next re-entry should preserve non-succeeded states and run succeeded bodies",
        );
        std::mem::forget(run);
    }

    /// K-REENTRY-RD-1: reduce_next re-entry harness.
    #[kani::proof]
    fn reduce_next_reentry() {
        let Some(mut run) = fresh_frame(3, 1) else {
            return;
        };
        let body = StepIdx::new(1);

        let body_state = step_state_from_u8(kani::any());
        kani::cover!(
            body_state == StepState::Succeeded,
            "reduce_next re-entry with Succeeded body state"
        );
        kani::cover!(
            body_state == StepState::Pending,
            "reduce_next re-entry with Pending body state"
        );
        if !run.kani_harness_set_step_state(body, body_state) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "reduce_next re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            kani::assert(false, "body step must remain readable after reduce_next");
            return;
        };
        let expected = if body_state == StepState::Succeeded {
            StepState::Running
        } else {
            body_state
        };
        kani::assert(
            state_after == expected,
            "reduce_next re-entry should preserve non-succeeded states and run succeeded bodies",
        );
        std::mem::forget(run);
    }

    /// K-REENTRY-CL-1: collect_next re-entry harness.
    #[kani::proof]
    #[kani::unwind(16)]
    fn collect_next_reentry() {
        let Some(mut run) = fresh_frame(2, 0) else {
            return;
        };
        let body = StepIdx::new(1);

        let body_state: StepState = kani::any();
        kani::cover!(
            body_state == StepState::Succeeded,
            "collect_next re-entry with Succeeded body state"
        );
        if !apply_body_state(&mut run, body, body_state) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(
            continue_state_readable(&result, &run, body),
            "step_state should be readable after collect_next",
        );
        kani::assert(
            !is_invalid_transition(&result),
            "collect_next re-entry should not fail with invalid_state_transition",
        );
        std::mem::forget(result);
    }

    /// K-REENTRY-CP-1: collect_page re-entry harness.
    #[kani::proof]
    fn collect_page_reentry() {
        let Some(mut run) = fresh_frame(2, 0) else {
            return;
        };
        let mut store = ValueStore::new();
        let mut states = CollectStates::new();
        let collector_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        if !run.kani_harness_write_slot_clean(collector_slot, SlotValue::List(ListId::new(1))) {
            return;
        }

        let body_state = StepState::Succeeded;
        kani::cover!(
            body_state == StepState::Succeeded,
            "collect_page re-entry with Succeeded body state"
        );
        if !run.kani_harness_set_step_state(body, body_state) {
            return;
        }

        let result = collect_page(&mut run, &mut store, &mut states, collector_slot, body, done);
        kani::assert(result.is_ok(), "collect_page re-entry should succeed");
        let Some(state_after) = run.kani_harness_step_state(body) else {
            kani::assert(false, "body step must remain readable after collect_page");
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "collect_page re-entry must mark body Running",
        );
        std::mem::forget(result);
        std::mem::forget(states);
        std::mem::forget(store);
        std::mem::forget(run);
    }

    /// K-REENTRY-RPA-1: repeat_attempt re-entry harness.
    #[kani::proof]
    fn repeat_attempt_reentry() {
        let Some(mut run) = fresh_frame(3, 0) else {
            return;
        };
        let body = StepIdx::new(1);

        let body_state = step_state_from_u8(kani::any());
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_attempt re-entry with Succeeded body state"
        );
        kani::cover!(
            body_state == StepState::Pending,
            "repeat_attempt re-entry with Pending body state"
        );
        if !run.kani_harness_set_step_state(body, body_state) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "repeat_attempt re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            kani::assert(false, "body step must remain readable after repeat_attempt");
            return;
        };
        let expected = if body_state == StepState::Succeeded {
            StepState::Running
        } else {
            body_state
        };
        kani::assert(
            state_after == expected,
            "repeat_attempt re-entry should preserve non-succeeded states and run succeeded bodies",
        );
        std::mem::forget(run);
    }

    /// PO-KANI-008: for_each_next body state re-entry.
    #[kani::proof]
    fn kani_for_each_reentry_body_state_immutable() {
        let Some(mut run) = fresh_frame(2, 0) else {
            return;
        };
        let body = StepIdx::new(1);
        if !run.kani_harness_set_step_state(body, StepState::Succeeded) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "for_each_next re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "for_each_next re-entry must mark body step Running",
        );
        std::mem::forget(run);
    }

    /// PO-KANI-009: reduce_next body state re-entry.
    #[kani::proof]
    fn kani_reduce_reentry_body_state_immutable() {
        let Some(mut run) = fresh_frame(2, 0) else {
            return;
        };
        let body = StepIdx::new(1);
        if !run.kani_harness_set_step_state(body, StepState::Succeeded) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "reduce_next re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "reduce_next re-entry must mark body step Running",
        );
        std::mem::forget(run);
    }

    /// PO-KANI-010: Combined remaining body state re-entry harness.
    #[kani::proof]
    #[kani::unwind(16)]
    fn kani_remaining_primitives_reentry_body_state_immutable() {
        verify_collect_next_reentry_body_state();
        verify_repeat_attempt_reentry_body_state();
        verify_repeat_check_reentry_body_state();
    }

    fn verify_collect_next_reentry_body_state() {
        let Some(mut run) = fresh_frame(2, 0) else {
            return;
        };
        let body = StepIdx::new(1);
        if !run.kani_harness_set_step_state(body, StepState::Succeeded) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "collect_next re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "collect_next must mark body state Running",
        );
        std::mem::forget(run);
    }

    fn verify_repeat_attempt_reentry_body_state() {
        let Some(mut run) = fresh_frame(3, 0) else {
            return;
        };
        let body = StepIdx::new(1);
        if !run.kani_harness_set_step_state(body, StepState::Succeeded) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "repeat_attempt re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(body) else {
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "repeat_attempt must mark body state Running",
        );
        std::mem::forget(run);
    }

    fn verify_repeat_check_reentry_body_state() {
        let Some(mut run) = fresh_frame(3, 0) else {
            return;
        };
        let next_body = StepIdx::new(1);
        if !run.kani_harness_set_step_state(next_body, StepState::Succeeded) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, next_body);
        kani::assert(result.is_ok(), "repeat_check re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(next_body) else {
            return;
        };
        kani::assert(
            state_after == StepState::Running,
            "repeat_check must mark body state Running",
        );
        std::mem::forget(run);
    }

    /// PO-KANI-011: jump_to_body re-entry transition.
    #[kani::proof]
    fn kani_jump_to_body_no_state_mutation() {
        let Some(mut run) = fresh_frame(4, 0) else {
            return;
        };
        let body = StepIdx::new(3);
        let body_state = step_state_from_u8(kani::any());
        if !run.kani_harness_set_step_state(body, body_state) {
            return;
        }
        let Some(state_before) = run.kani_harness_step_state(body) else {
            return;
        };
        let pc_before = run.pc();
        let executed_before = run.executed();

        let result = crate::primitives::helpers::jump_to_body(&mut run, body);
        kani::assert(result.is_ok(), "jump_to_body should succeed for valid body step");
        std::mem::forget(result);

        let Some(state_after) = run.kani_harness_step_state(body) else {
            return;
        };
        let expected = if state_before == StepState::Succeeded {
            StepState::Running
        } else {
            state_before
        };
        kani::assert(
            state_after == expected,
            "jump_to_body must only mutate Succeeded body state to Running",
        );
        kani::cover!(run.pc() != pc_before, "PC changed after jump_to_body");
        kani::cover!(run.executed() != executed_before, "executed incremented");
    }

    /// K-REENTRY-RPC-1: repeat_check re-entry harness.
    #[kani::proof]
    fn repeat_check_reentry() {
        let Some(mut run) = fresh_frame(3, 0) else {
            return;
        };
        let next_body = StepIdx::new(1);

        let body_state = step_state_from_u8(kani::any());
        kani::cover!(
            body_state == StepState::Succeeded,
            "repeat_check re-entry with Succeeded body state"
        );
        kani::cover!(
            body_state == StepState::Pending,
            "repeat_check re-entry with Pending body state"
        );
        if !run.kani_harness_set_step_state(next_body, body_state) {
            return;
        }

        let result = crate::primitives::helpers::jump_to_body(&mut run, next_body);
        kani::assert(result.is_ok(), "repeat_check re-entry jump should succeed");
        std::mem::forget(result);
        let Some(state_after) = run.kani_harness_step_state(next_body) else {
            kani::assert(false, "body step must remain readable after repeat_check");
            return;
        };
        let expected = if body_state == StepState::Succeeded {
            StepState::Running
        } else {
            body_state
        };
        kani::assert(
            state_after == expected,
            "repeat_check re-entry should preserve non-succeeded states and run succeeded bodies",
        );
        std::mem::forget(run);
    }
}
