//! RED PHASE Tests for vb-99n6 — Timer Wheel Driven Resume and Cancellation Hardening
//!
//! These tests define the expected behavior per the contract and test-plan.
//! They MUST FAIL against the current implementation.
//!
//! Tests focus on the critical paths:
//! 1. Resume correctness - timer must NOT be consumed on resume
//! 2. Cancellation cleanup - timer must be atomically removed
//! 3. Timer fire atomicity - stale timer fires must return InvalidTimerFire

#![allow(dead_code, unused_imports)]

use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{PendingTimer, PendingTimerKind};
use crate::shard::timer_wheel::TimerWheel;
use crate::shard::helpers::{
    advance_after_timer_fire,
    timer_registration_required,
};

fn run(id: u64) -> vb_core::ids::RunId {
    vb_core::ids::RunId::new(id)
}

// =============================================================================
// TIMER WHEEL UNIT TESTS (TW-UT-*)
// =============================================================================

mod timer_wheel_tests {
    use super::*;

    // TW-UT-001: insert stores entry in both by_deadline and by_run
    #[test]
    fn insert_stores_in_both_indexes() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let deadline = now + std::time::Duration::from_millis(100);

        wheel.insert(run(1), deadline, PendingTimerKind::Wait);

        // After insert, the timer should be retrievable
        assert!(
            wheel.get_kind(run(1)).is_some(),
            "get_kind should find the inserted timer"
        );
        assert!(
            wheel.next_deadline().is_some(),
            "next_deadline should return the deadline"
        );
    }

    // TW-UT-002: insert returns previous timer when replacing
    #[test]
    fn insert_returns_previous_on_replacement() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let d1 = now + std::time::Duration::from_millis(10);
        let d2 = now + std::time::Duration::from_millis(20);

        wheel.insert(run(1), d1, PendingTimerKind::Wait);
        wheel.insert(run(1), d2, PendingTimerKind::Ask);

        // After replacement, len should still be 1
        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
        // The next_deadline should be the new deadline
        assert_eq!(wheel.next_deadline(), Some(d2));
    }

    // TW-UT-003: cancel returns true and removes from both indexes when present
    #[test]
    fn cancel_removes_from_both_indexes() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let deadline = now + std::time::Duration::from_millis(100);

        wheel.insert(run(1), deadline, PendingTimerKind::Wait);
        let result = wheel.cancel(run(1));

        assert_eq!(result, true, "cancel should return true when present");
        assert!(wheel.is_empty(), "wheel should be empty after cancel");
        assert_eq!(wheel.get_kind(run(1)), None);
    }

    // TW-UT-004: cancel returns false when run has no timer
    #[test]
    fn cancel_returns_false_when_not_present() {
        let mut wheel = TimerWheel::new();
        let result = wheel.cancel(run(99));
        assert_eq!(result, false, "cancel should return false when not present");
    }

    // TW-UT-005: fire_expired returns only expired entries
    #[test]
    fn fire_expired_returns_only_expired() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);
        let future = now + std::time::Duration::from_secs(60);

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        wheel.insert(run(2), future, PendingTimerKind::Ask);

        let fired = wheel.fire_expired(now);

        assert_eq!(fired.len(), 1, "only expired timer should fire");
        assert_eq!(fired[0].run, run(1));
        assert_eq!(wheel.len(), 1, "future entry should remain");
    }

    // TW-UT-006: fire_expired removes from both indexes
    #[test]
    fn fire_expired_removes_from_both_indexes() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        let _fired = wheel.fire_expired(now);

        assert!(wheel.is_empty(), "wheel should be empty after fire_expired");
    }

    // TW-UT-007: fire_expired returns empty when no entries expired
    #[test]
    fn fire_expired_returns_empty_when_no_expired() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let future = now + std::time::Duration::from_secs(60);

        wheel.insert(run(1), future, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(now);

        assert_eq!(fired.len(), 0, "no expired timers should return empty");
        assert_eq!(wheel.len(), 1, "future entry should remain");
    }

    // TW-UT-008: next_deadline returns earliest deadline
    #[test]
    fn next_deadline_returns_earliest() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let early = now + std::time::Duration::from_millis(10);
        let late = now + std::time::Duration::from_millis(100);

        wheel.insert(run(1), late, PendingTimerKind::Wait);
        wheel.insert(run(2), early, PendingTimerKind::Ask);

        assert_eq!(wheel.next_deadline(), Some(early));
    }

    // TW-UT-009: next_deadline returns None when empty
    #[test]
    fn next_deadline_none_when_empty() {
        let wheel = TimerWheel::new();
        assert_eq!(wheel.next_deadline(), None);
    }

    // TW-UT-010: len equals count of unique runs
    #[test]
    fn len_equals_run_count() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        wheel.insert(run(2), now, PendingTimerKind::Ask);
        assert_eq!(wheel.len(), 2);

        // Replacement should not increase count
        wheel.insert(run(1), now, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 2, "replacement should not increase len");
    }

    // TW-UT-011: get_kind returns correct kind
    #[test]
    fn get_kind_returns_registered_kind() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Wait));
    }

    // TW-UT-012: get_kind returns None for cancelled run
    #[test]
    fn get_kind_none_after_cancel() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        wheel.cancel(run(1));
        assert_eq!(wheel.get_kind(run(1)), None);
    }

    // TW-UT-013: fire_expired is idempotent (double-fire returns empty)
    #[test]
    fn fire_expired_idempotent() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        let first = wheel.fire_expired(now);
        let second = wheel.fire_expired(now);

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 0, "second fire_expired should return empty");
        assert_eq!(wheel.len(), 0);
    }
}

// =============================================================================
// HELPERS UNIT TESTS (HP-UT-*)
// =============================================================================

mod helpers_tests {
    use super::*;

    fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_then_finish"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set_timeout = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_then_finish"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
                ConstValue::I64(10),
            ]),
            slot_count: 3,
            symbols_count: 2,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn simple_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("simple"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn make_run_state(
        workflow: vb_core::workflow::CompiledWorkflow,
        run_id: vb_core::ids::RunId,
    ) -> Option<crate::shard::types::RunState> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let frame = vb_core::frame::RunFrame::new(run_id, workflow.entry(), step_count, slot_count).ok()?;
        Some(crate::shard::types::RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::new(),
            action_attempts: crate::shard::helpers::new_action_attempts(step_count),
            admission: None,
            collect_states: crate::primitives::collect::CollectStates::new(),
        })
    }

    // HP-UT-001: timer_registration_required returns true for WaitUntil step
    #[test]
    fn timer_reg_required_true_for_wait_until() {
        let wf = wait_workflow().unwrap();
        let state = make_run_state(wf, run(1)).unwrap();
        let step = StepIdx::new(1); // WaitUntil step

        assert_eq!(
            timer_registration_required(&state, step),
            true,
            "timer_registration_required should be true for WaitUntil"
        );
    }

    // HP-UT-002: timer_registration_required returns true for Ask(timeout) step
    #[test]
    fn timer_reg_required_true_for_ask() {
        let wf = ask_workflow().unwrap();
        let state = make_run_state(wf, run(1)).unwrap();
        let step = StepIdx::new(2); // Ask step

        assert_eq!(
            timer_registration_required(&state, step),
            true,
            "timer_registration_required should be true for Ask"
        );
    }

    // HP-UT-003: timer_registration_required returns false for Finish step
    #[test]
    fn timer_reg_required_false_for_finish() {
        let wf = simple_workflow().unwrap();
        let state = make_run_state(wf, run(1)).unwrap();
        let step = StepIdx::new(1); // Finish step

        assert_eq!(
            timer_registration_required(&state, step),
            false,
            "timer_registration_required should be false for Finish"
        );
    }

    // HP-UT-004: advance_after_timer_fire updates frame for PendingTimerKind::Wait
    #[test]
    fn advance_after_timer_fire_for_wait() {
        let wf = wait_workflow().unwrap();
        let mut state = make_run_state(wf, run(1)).unwrap();

        // Drive to WaitUntil step
        state.frame.mark_running(StepIdx::ZERO).unwrap();
        state.frame.mark_succeeded(StepIdx::ZERO).unwrap();
        state.frame.set_pc(StepIdx::new(1)).unwrap();
        state.frame.mark_running(StepIdx::new(1)).unwrap();

        let timer = PendingTimer {
            step: StepIdx::new(1),
            kind: PendingTimerKind::Wait,
        };

        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Ok(()));
        assert_eq!(
            state.frame.pc(),
            StepIdx::new(2),
            "PC should advance past WaitUntil after timer fire"
        );
    }

    // HP-UT-005: advance_after_timer_fire signals failure for PendingTimerKind::Ask
    #[test]
    fn advance_after_timer_fire_for_ask() {
        let wf = ask_workflow().unwrap();
        let mut state = make_run_state(wf, run(1)).unwrap();

        // Drive to Ask step
        state.frame.mark_running(StepIdx::ZERO).unwrap();
        state.frame.mark_succeeded(StepIdx::ZERO).unwrap();
        state.frame.set_pc(StepIdx::new(1)).unwrap();
        state.frame.mark_running(StepIdx::new(1)).unwrap();
        state.frame.mark_succeeded(StepIdx::new(1)).unwrap();
        state.frame.set_pc(StepIdx::new(2)).unwrap();
        state.frame.mark_running(StepIdx::new(2)).unwrap();

        let timer = PendingTimer {
            step: StepIdx::new(2),
            kind: PendingTimerKind::Ask,
        };

        let result = advance_after_timer_fire(&mut state, timer);
        // Ask timer fire should signal failure, not advance
        assert!(
            result.is_err(),
            "advance_after_timer_fire for Ask should return error"
        );
    }
}

// =============================================================================
// INTEGRATION TESTS (IT-*)
// These tests verify the handle_timer, handle_resume, and handle_cancel behaviors
// =============================================================================

mod integration_tests {
    use super::*;

    fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("suspended"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_then_finish"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn wait_workflow_with_past_deadline() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_past_deadline"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            // Use a past deadline value
            constants: Box::from([ConstValue::I64(-1000)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set_timeout = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_then_finish"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
                ConstValue::I64(10),
            ]),
            slot_count: 3,
            symbols_count: 2,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn small_config() -> crate::shard::ShardConfig {
        crate::shard::ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    // IT-TIMER-001: Timer fire advances WaitUntil to completion
    #[test]
    fn timer_fire_advances_wait_until_to_completion() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = wait_workflow_with_past_deadline().unwrap();
        let run_id = run(1);

        // Submit the workflow
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // After submit, timer should be registered
        assert_eq!(
            shard.pending_timers.len(),
            1,
            "pending_timers should have 1 entry after submit"
        );

        // Enqueue TimerFired
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::TimerFired { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // After timer fire, pending_timers should be empty and run completed
        assert_eq!(
            shard.pending_timers.len(),
            0,
            "pending_timers should be empty after timer fire"
        );
        assert_eq!(
            shard.active_run_count(),
            0,
            "run should be completed"
        );
    }

    // IT-TIMER-004: TimerFired on unknown run returns RunNotFound
    #[test]
    fn timer_fired_on_unknown_run_returns_run_not_found() {
        let mut shard = crate::shard::Shard::new(small_config());
        let nonexistent = run(9999);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::TimerFired { run: nonexistent }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::RunNotFound),
            "TimerFired on unknown run should return RunNotFound"
        );
    }

    // IT-TIMER-005: TimerFired on run with no pending timer returns InvalidTimerFire
    #[test]
    fn timer_fired_on_run_with_no_pending_timer_returns_invalid_timer_fire() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = suspended_workflow().unwrap();
        let run_id = run(1);

        // Submit action-suspended workflow (no timer)
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // No timer should be registered
        assert_eq!(
            shard.pending_timers.len(),
            0,
            "no pending timer for action-suspended workflow"
        );

        // Enqueue TimerFired
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::TimerFired { run: run_id }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::InvalidTimerFire),
            "TimerFired with no timer should return InvalidTimerFire"
        );
    }

    // IT-TIMER-006: TimerFired after cancel returns RunNotFound
    #[test]
    fn timer_fired_after_cancel_returns_run_not_found() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = wait_workflow().unwrap();
        let run_id = run(1);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // Cancel
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Cancel { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // Run should be removed
        assert_eq!(shard.runs.get(&run_id), None);
        assert_eq!(shard.pending_timers.get(&run_id), None);

        // Enqueue TimerFired
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::TimerFired { run: run_id }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::RunNotFound),
            "TimerFired after cancel should return RunNotFound"
        );
    }

    // IT-RESUME-001: Resume re-drives action-suspended run
    #[test]
    fn resume_re_drives_action_suspended_run() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = suspended_workflow().unwrap();
        let run_id = run(1);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // Resume
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Resume { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // Run should still be in runs map
        assert_eq!(
            shard.runs.get(&run_id).is_some(),
            true,
            "run should remain in runs map after resume"
        );
        assert_eq!(
            shard.pending_timers.len(),
            0,
            "pending_timers should be unchanged"
        );
    }

    // IT-RESUME-002: Resume re-drives wait-suspended run without consuming timer
    #[test]
    fn resume_re_drives_wait_suspended_run_without_consuming_timer() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = wait_workflow().unwrap();
        let run_id = run(1);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.pending_timers.len(),
            1,
            "timer should be registered"
        );

        // Resume
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Resume { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // Timer should still be present after resume
        assert_eq!(
            shard.pending_timers.len(),
            1,
            "timer should still be present after resume"
        );

        // TimerFired should still succeed
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::TimerFired { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.active_run_count(),
            0,
            "run should complete after timer fire"
        );
    }

    // IT-RESUME-004: Resume on unknown run returns RunNotFound
    #[test]
    fn resume_on_unknown_run_returns_run_not_found() {
        let mut shard = crate::shard::Shard::new(small_config());
        let nonexistent = run(9999);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Resume { run: nonexistent }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::RunNotFound),
            "Resume on unknown run should return RunNotFound"
        );
    }

    // IT-CANCEL-001: Cancel removes run and timer atomically
    #[test]
    fn cancel_removes_run_and_timer_atomically() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = wait_workflow().unwrap();
        let run_id = run(1);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.pending_timers.len(),
            1,
            "pending_timers should have 1 entry"
        );

        // Cancel
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Cancel { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.runs.get(&run_id),
            None,
            "run should be removed from runs"
        );
        assert_eq!(
            shard.pending_timers.get(&run_id),
            None,
            "timer should be removed from pending_timers"
        );
        assert_eq!(
            shard.counters().snapshot().runs_failed,
            1,
            "runs_failed counter should be incremented"
        );
    }

    // IT-CANCEL-002: Cancel on non-existent run succeeds silently
    #[test]
    fn cancel_on_nonexistent_run_succeeds_silently() {
        let mut shard = crate::shard::Shard::new(small_config());
        let nonexistent = run(9999);
        let counter_before = shard.counters().snapshot().runs_failed;

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Cancel { run: nonexistent }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.counters().snapshot().runs_failed,
            counter_before,
            "runs_failed counter should not change for nonexistent run"
        );
    }

    // IT-CANCEL-003: Duplicate cancel is idempotent
    #[test]
    fn duplicate_cancel_is_idempotent() {
        let mut shard = crate::shard::Shard::new(small_config());
        let wf = suspended_workflow().unwrap();
        let run_id = run(1);

        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Submit {
                run: run_id,
                workflow: wf,
                caps: vb_core::capability::CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        // First cancel
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Cancel { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        let counter_after_first = shard.counters().snapshot().runs_failed;
        assert_eq!(counter_after_first, 1);

        // Second cancel (duplicate)
        assert_eq!(
            shard.enqueue(crate::shard::ShardCommand::Cancel { run: run_id }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.counters().snapshot().runs_failed,
            counter_after_first,
            "runs_failed counter should not increment on duplicate cancel"
        );
    }
}

// =============================================================================
// PROPERTY-BASED TESTS (PB-*)
// These test invariants that should hold true
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;

    // PB-TW-001: Dual-index consistency after insert
    #[test]
    fn dual_index_consistency_after_insert() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        wheel.insert(run(2), now + std::time::Duration::from_secs(1), PendingTimerKind::Ask);

        // Invariant: by_deadline and by_run contain same set of entries
        assert_eq!(wheel.len(), 2);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Wait));
        assert_eq!(wheel.get_kind(run(2)), Some(PendingTimerKind::Ask));
    }

    // PB-TW-002: Dual-index consistency after cancel
    #[test]
    fn dual_index_consistency_after_cancel() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        wheel.cancel(run(1));

        // After cancel, both indexes should be empty
        assert_eq!(wheel.len(), 0);
        assert!(wheel.is_empty());
        assert_eq!(wheel.get_kind(run(1)), None);
    }

    // PB-TW-003: Dual-index consistency after fire_expired
    #[test]
    fn dual_index_consistency_after_fire_expired() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(now);

        // After fire_expired, both indexes should be empty for fired entries
        assert_eq!(fired.len(), 1);
        assert!(wheel.is_empty());
    }

    // PB-TW-004: Replacement cancels previous timer
    #[test]
    fn replacement_cancels_previous() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let d1 = now + std::time::Duration::from_secs(10);
        let d2 = now + std::time::Duration::from_secs(20);

        wheel.insert(run(1), d1, PendingTimerKind::Wait);
        wheel.insert(run(1), d2, PendingTimerKind::Ask);

        // len should be 1, not 2
        assert_eq!(wheel.len(), 1);
        // get_kind should return the latest
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
    }

    // PB-TW-005: fire_expired never returns non-expired entries
    #[test]
    fn fire_expired_only_returns_expired() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);
        let future = now + std::time::Duration::from_secs(60);

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        wheel.insert(run(2), future, PendingTimerKind::Ask);

        let fired = wheel.fire_expired(now);

        // All fired entries should have deadline <= now
        for entry in fired {
            // The entry.run should not be findable after fire
            assert_eq!(wheel.get_kind(entry.run), None);
        }
        // Only the future entry should remain
        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(run(2)), Some(PendingTimerKind::Ask));
    }

    // PB-SM-001: At most one pending timer per run (I-1)
    #[test]
    fn at_most_one_timer_per_run() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        // Multiple inserts for same run
        wheel.insert(run(1), now, PendingTimerKind::Wait);
        wheel.insert(run(1), now + std::time::Duration::from_secs(1), PendingTimerKind::Ask);
        wheel.insert(run(1), now + std::time::Duration::from_secs(2), PendingTimerKind::Wait);

        // Should still only have 1 timer
        assert_eq!(wheel.len(), 1);
    }

    // PB-GLOBAL-002: handle_cancel idempotent (I-6)
    #[test]
    fn cancel_idempotent() {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        let first = wheel.cancel(run(1));
        let second = wheel.cancel(run(1));

        assert_eq!(first, true);
        assert_eq!(second, false, "second cancel should return false");
    }
}
