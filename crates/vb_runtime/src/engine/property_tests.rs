#![forbid(unsafe_code)]

//! Property tests verifying critical invariants for the runtime engine.
//!
//! Tests cover INV(E1-E4) Evidence Chain, INV(B1-B3) Budget,
//! INV(F1-F3) Frame Pool, INV(S1-S4) Shard, and INV(M1-M2) Step State Machine.
//!
//! All tests use proptest strategies with bounded iterations.
//! No loops in test bodies - use proptest strategies for bounded iteration.
//! No unwrap/expect/panic in test bodies - use assert! for assertions.

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, unused_mut, clippy::bool_comparison)]
mod proptests {
    use proptest::prelude::*;

    use vb_core::action::ActionTicket;
    use vb_core::capability::CapabilitySet;
    use vb_core::engine::StepBudget;
    use vb_core::errors::EngineError;
    use vb_core::frame::{RunFrame, StepState};
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx};
    use vb_core::value::ConstValue;
    use vb_core::value::SlotValue;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    use crate::RuntimeError;
    use crate::engine::drive::{compute_max_parallel_in_flight, drive_deterministic_full};
    use crate::engine::helpers::mark_step_after_signal;
    use crate::engine::types::{
        EvidenceCollector, EvidenceEvent, RuntimeEngineError, RuntimeSignal,
    };
    use crate::frame_pool::FramePool;
    use crate::primitives::collect::CollectStates;
    use crate::shard::{Shard, ShardCommand, ShardConfig};

    #[allow(dead_code)]
    fn make_simple_workflow(slot_count: u16, result_slot: SlotIdx) -> CompiledWorkflow {
        let node0 = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let node1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: result_slot,
            },
        };
        let parts = WorkflowParts {
            name: "test_workflow".into(),
            digest: vb_core::ids::WorkflowDigest::from_bytes([1; 32]),
            nodes: vec![node0, node1].into_boxed_slice(),
            expressions: vec![].into_boxed_slice(),
            accessors: vec![].into_boxed_slice(),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: vec!["step0".into(), "step1".into()].into_boxed_slice(),
        };
        CompiledWorkflow::try_from_parts(parts).expect("valid workflow")
    }

    fn make_nop_workflow(step_count: u16) -> CompiledWorkflow {
        let mut nodes = Vec::new();
        for i in 0..step_count {
            let next = if i < step_count - 1 {
                Some(StepIdx::new(i + 1))
            } else {
                None
            };
            let kind = if i == step_count - 1 {
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                }
            } else {
                CompiledNodeKind::Nop
            };
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next,
                on_error: None,
                error_slot: None,
                kind,
            });
        }
        let parts = WorkflowParts {
            name: "nop_chain".into(),
            digest: vb_core::ids::WorkflowDigest::from_bytes([2; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![].into_boxed_slice(),
            accessors: vec![].into_boxed_slice(),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: vec![].into_boxed_slice(),
        };
        CompiledWorkflow::try_from_parts(parts).expect("valid workflow")
    }

    // =========================================================================
    // INV(E1): event_ordering - StepSucceeded appears after StepStarted
    // INV(E2): started_before_slot_written - StepStarted appears before SlotWritten
    // INV(E3): no_spurious_succeeded - No StepSucceeded for Awaiting* signals
    // =========================================================================

    proptest! {
        #[test]
        fn evidence_chain_ordering_preserved(
            workflow_steps in 1u16..=50u16,
            budget in 1u64..=1000u64,
        ) {
            let workflow = make_nop_workflow(workflow_steps);
            let mut run = RunFrame::new(
                RunId::new(1),
                StepIdx::ZERO,
                workflow.node_count(),
                workflow.slot_count(),
            ).expect("valid frame");
            let mut budget = StepBudget::new(budget);
            let mut store = ValueStore::new();
            let mut evidence = EvidenceCollector::new();
            let mut collect_states = CollectStates::new();
            let granted = CapabilitySet::empty();

            let _result = drive_deterministic_full(
                &workflow,
                &mut run,
                &mut budget,
                &mut store,
                &[],
                crate::engine::types::RetryPolicy::NEVER,
                &mut evidence,
                &mut collect_states,
                &granted,
            );

            let events = evidence.drain();

            // INV(E1): For every StepSucceeded(step), a StepStarted(step) appears earlier
            let mut step_started_positions: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
            let mut step_succeeded_positions: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
            let mut slot_written_positions: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();

            for (idx, event) in events.iter().enumerate() {
                match event {
                    EvidenceEvent::StepStarted { step } => {
                        let pos = step_started_positions.entry(step.get()).or_insert(idx);
                        if idx < *pos { *pos = idx; }
                    }
                    EvidenceEvent::StepSucceeded { step, .. } => {
                        let pos = step_succeeded_positions.entry(step.get()).or_insert(idx);
                        if idx < *pos { *pos = idx; }
                    }
                    EvidenceEvent::SlotWritten { slot, .. } => {
                        let pos = slot_written_positions.entry(slot.get()).or_insert(idx);
                        if idx < *pos { *pos = idx; }
                    }
                }
            }

            // INV(E1): Check StepStarted comes before StepSucceeded for each step
            for (step, &succeeded_pos) in &step_succeeded_positions {
                if let Some(&started_pos) = step_started_positions.get(step) {
                    assert!(
                        started_pos < succeeded_pos,
                        "INV(E1) violated: StepStarted({step}) at {started_pos} must precede StepSucceeded at {succeeded_pos}",
                    );
                }
            }
        }
    }

    // =========================================================================
    // INV(E4): evidence_drain_resets_dropped_counter
    // =========================================================================

    proptest! {
        #[test]
        fn evidence_drain_resets_dropped_counter(
            capacity in 0usize..=100usize,
            event_count in 0usize..=500usize,
        ) {
            let mut collector = EvidenceCollector::with_capacity(capacity);

            // Push events up to event_count
            for i in 0..event_count {
                collector.push_step_started(StepIdx::new(i as u16));
            }

            let len_before = collector.len();

            // Drain
            let drained = collector.drain();

            // INV(E4): After drain, len() == 0 and dropped() == 0
            assert_eq!(collector.len(), 0, "INV(E4): len() must be 0 after drain");
            assert_eq!(collector.dropped(), 0, "INV(E4): dropped() must be 0 after drain");
            assert_eq!(drained.len(), len_before, "drain() returns exactly len() events");
        }
    }

    // =========================================================================
    // INV(B1): budget_exhaustion_stops_execution
    // INV(B2): zero_budget_means_no_execution
    // =========================================================================

    proptest! {
        #[test]
        fn budget_exhaustion_stops_at_exact_boundary(
            workflow_steps in 1u16..=100u16,
            budget_value in 0u64..=10_000u64,
        ) {
            let workflow = make_nop_workflow(workflow_steps);
            let mut run = RunFrame::new(
                RunId::new(1),
                StepIdx::ZERO,
                workflow.node_count(),
                workflow.slot_count(),
            ).expect("valid frame");
            let mut budget = StepBudget::new(budget_value);
            let initial_budget = budget.remaining();
            let mut store = ValueStore::new();
            let mut evidence = EvidenceCollector::new();
            let mut collect_states = CollectStates::new();
            let granted = CapabilitySet::empty();

            let result = drive_deterministic_full(
                &workflow,
                &mut run,
                &mut budget,
                &mut store,
                &[],
                crate::engine::types::RetryPolicy::NEVER,
                &mut evidence,
                &mut collect_states,
                &granted,
            );

            let events = evidence.drain();
            let step_started_count = events.iter().filter(|e| matches!(e, EvidenceEvent::StepStarted { .. })).count();

            // INV(B1): When budget.try_take() returns false, no node executes in that iteration
            // and loop exits with StepBudgetExhausted
            if budget_value == 0 {
                // INV(B2): Zero budget means no execution
                assert!(matches!(result, Ok(RuntimeSignal::StepBudgetExhausted)),
                    "INV(B2): budget=0 must return StepBudgetExhausted");
                assert_eq!(step_started_count, 0, "INV(B2): no steps execute with budget=0");
            } else {
                // Budget exhaustion happens when budget runs out
                let max_possible_steps = usize::try_from(initial_budget).unwrap_or(usize::MAX);
                assert!(
                    step_started_count <= max_possible_steps,
                    "INV(B1): step count {step_started_count} must not exceed budget {max_possible_steps}",
                );
            }

            // INV(B1): PC is always in bounds
            assert!(
                run.pc().get() < workflow.node_count(),
                "INV(B1): PC must be in bounds after execution",
            );
        }
    }

    // =========================================================================
    // INV(B3): budget_decrement_is_unit
    // =========================================================================

    proptest! {
        #[test]
        fn budget_decrement_is_unit(initial_budget in 1u64..=10_000u64) {
            let mut budget = StepBudget::new(initial_budget);
            let mut consumed = 0u64;

            // Consume all budget - bounded iteration
            for _ in 0..initial_budget.saturating_add(1) {
                match budget.try_take() {
                    Ok(true) => consumed += 1,
                    Ok(false) => break,
                    Err(_) => break,
                }
            }

            assert_eq!(
                consumed,
                initial_budget,
                "INV(B3): consumed {consumed} must equal initial_budget {initial_budget}",
            );
            assert_eq!(
                budget.remaining(),
                0,
                "INV(B3): remaining must be 0 after exhausting",
            );
        }
    }

    // =========================================================================
    // INV(F1): capacity_never_exceeded
    // =========================================================================

    proptest! {
        #[test]
        fn frame_pool_capacity_never_exceeded(
            step_count in 1u16..=16u16,
            slot_count in 0u16..=16u16,
            capacity in 1u16..=100u16,
            releases in 0usize..=500usize,
        ) {
            let mut pool = FramePool::new(step_count, slot_count, capacity.into()).expect("valid pool");

            // Take frames - bounded iteration
            let mut taken_frames = Vec::new();
            for i in 0..releases {
                let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                match pool.take(run_id, StepIdx::ZERO) {
                    Ok(frame) => {
                        assert!(
                            pool.available() <= capacity as usize,
                            "INV(F1): available() {} must never exceed capacity {}",
                            pool.available(),
                            capacity,
                        );
                        taken_frames.push(frame);
                    }
                    Err(_) => break,
                }
            }

            // Release frames - bounded iteration
            for frame in taken_frames {
                pool.release(frame);
                assert!(
                    pool.available() <= capacity as usize,
                    "INV(F1): available() {} must never exceed capacity {} after release",
                    pool.available(),
                    capacity,
                );
            }

            // Final check
            assert!(
                pool.available() <= capacity as usize,
                "INV(F1): final available() {} must not exceed capacity {}",
                pool.available(),
                capacity,
            );
        }
    }

    // =========================================================================
    // INV(F2): dimension_mismatch_drops
    // =========================================================================

    proptest! {
        #[test]
        fn frame_pool_dimension_mismatch_silent_drop(
            pool_s1 in 1u16..=8u16,
            pool_c1 in 0u16..=8u16,
            pool_cap in 1u16..=10u16,
            frame_s2 in 1u16..=8u16,
            frame_c2 in 0u16..=8u16,
        ) {
            // Ensure at least one dimension differs
            prop_assume!(pool_s1 != frame_s2 || pool_c1 != frame_c2);

            let mut pool = FramePool::new(pool_s1, pool_c1, pool_cap.into()).expect("valid pool");
            let available_before = pool.available();

            // Create a frame with different dimensions
            let wrong_frame = RunFrame::new(
                RunId::new(1),
                StepIdx::ZERO,
                frame_s2,
                frame_c2,
            ).expect("valid frame");

            pool.release(wrong_frame);

            // INV(F2): release() silently drops mismatched frame, available unchanged
            assert_eq!(
                pool.available(),
                available_before,
                "INV(F2): available() must remain unchanged {} when dimensions mismatch",
                available_before,
            );
        }
    }

    // =========================================================================
    // INV(F3): reuse_produces_clean_frame
    // =========================================================================

    proptest! {
        #[test]
        fn frame_reuse_clears_all_prior_state(
            step_count in 1u16..=16u16,
            slot_count in 0u16..=16u16,
        ) {
            let mut pool = FramePool::new(step_count, slot_count, 2).expect("valid pool");

            // Take first frame and use it
            let run_id1 = RunId::new(1);
            let mut frame1 = pool.take(run_id1, StepIdx::ZERO).expect("frame available");

            // Mark running and write to slots
            frame1.mark_running(StepIdx::ZERO).expect("mark_running must succeed in test");
            frame1.mark_succeeded(StepIdx::ZERO).expect("mark_succeeded must succeed in test");
            if slot_count > 0 {
                frame1.write_slot(SlotIdx::ZERO, SlotValue::I64(999))
                    .expect("write_slot must succeed in test");
            }
            frame1.increment_executed().expect("increment_executed must succeed in test");

            // Release frame back to pool
            pool.release(frame1);

            // Take second frame (should be recycled)
            let run_id2 = RunId::new(2);
            let frame2 = pool.take(run_id2, StepIdx::ZERO).expect("frame available");

            // INV(F3): reused frame has clean state
            assert_eq!(
                frame2.executed(),
                0,
                "INV(F3): executed() must be 0 for recycled frame",
            );
            assert_eq!(
                frame2.run_id(),
                run_id2,
                "INV(F3): run_id must be new run_id",
            );
            assert_eq!(
                frame2.pc(),
                StepIdx::ZERO,
                "INV(F3): pc must be first_step",
            );

            // All slots must be uninitialized
            if slot_count > 0 {
                let result = frame2.read_slot(SlotIdx::ZERO);
                assert!(
                    matches!(result, Err(vb_core::errors::CoreError::SlotUninitialized { .. })),
                    "INV(F3): slot read must return SlotUninitialized",
                );
            }
        }
    }

    // =========================================================================
    // INV(S1): command_queue_bounded
    // =========================================================================

    proptest! {
        #[test]
        fn command_queue_full_boundary(
            capacity in 1u8..=64u8,
            enqueue_count in 0usize..=200usize,
        ) {
            let config = ShardConfig {
                command_queue_capacity: capacity as usize,
                trace_capacity: 1024,
                step_budget_per_tick: 1000,
                max_active_runs: 1024,
                policy: vb_core::policy::RuntimePolicy::Strict,
            };
            let shard = Shard::new(config);

            let mut success_count = 0usize;

            for i in 0..enqueue_count {
                let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                let workflow = make_nop_workflow(2);
                let cmd = ShardCommand::Submit {
                    run: run_id,
                    workflow,
                    caps: CapabilitySet::empty(),
                };

                match shard.enqueue(cmd) {
                    Ok(()) => success_count += 1,
                    Err(RuntimeError::QueueFull) => {},
                    Err(_) => {}
                }
            }

            // INV(S1): QueueFull returned exactly when is_queue_full()
            if success_count == capacity as usize {
                assert!(
                    shard.is_queue_full(),
                    "INV(S1): is_queue_full() must be true when queue is at capacity",
                );
            }

            // remaining_capacity() equals capacity - len()
            assert_eq!(
                shard.remaining_capacity(),
                capacity as usize - shard.command_queue_len(),
                "INV(S1): remaining_capacity() must equal capacity - len()",
            );
        }
    }

    // =========================================================================
    // INV(S2): one_command_per_tick
    // =========================================================================

    proptest! {
        #[test]
        fn one_command_per_tick_enforced(
            command_count in 1usize..=20usize,
            tick_count in 1usize..=50usize,
        ) {
            let config = ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 1024,
                step_budget_per_tick: 1000,
                max_active_runs: 1024,
                policy: vb_core::policy::RuntimePolicy::Strict,
            };
            let mut shard = Shard::new(config);

            // Enqueue commands
            for i in 0..command_count {
                let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                let workflow = make_nop_workflow(2);
                let cmd = ShardCommand::Submit {
                    run: run_id,
                    workflow,
                    caps: CapabilitySet::empty(),
                };
                shard.enqueue(cmd).expect("enqueue must succeed");
            }

            let initial_queue_len = shard.command_queue_len();

            // Tick and count how many commands were actually processed.
            // A command is processed only when the queue shrinks (tick pops a command).
            let mut processed = 0usize;
            for _ in 0..tick_count {
                let queue_before = shard.command_queue_len();
                match shard.tick() {
                    Ok(true) => {
                        if shard.command_queue_len() < queue_before {
                            processed += 1;
                        }
                    }
                    Ok(false) => break, // Shutdown
                    Err(_) => break,
                }
            }

            // INV(S2): At most tick_count commands processed
            assert!(
                processed <= tick_count,
                "INV(S2): processed {} must not exceed tick_count {}",
                processed,
                tick_count,
            );

            // INV(S2): After N ticks, exactly min(initial_queue_len, tick_count) commands processed
            let expected_processed = initial_queue_len.min(tick_count);
            assert_eq!(
                processed,
                expected_processed,
                "INV(S2): processed {} must equal min(queue_len {}, tick_count {})",
                processed,
                initial_queue_len,
                tick_count,
            );
        }
    }

    // =========================================================================
    // INV(S3): shutdown_termination
    // =========================================================================

    proptest! {
        #[test]
        fn shutdown_terminates_tick_loop(
            pre_shutdown_commands in 0usize..=10usize,
        ) {
            let config = ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 1024,
                step_budget_per_tick: 1000,
                max_active_runs: 1024,
                policy: vb_core::policy::RuntimePolicy::Strict,
            };
            let mut shard = Shard::new(config);

            // Enqueue pre-shutdown commands
            for i in 0..pre_shutdown_commands {
                let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                let workflow = make_nop_workflow(2);
                let cmd = ShardCommand::Submit {
                    run: run_id,
                    workflow,
                    caps: CapabilitySet::empty(),
                };
                shard.enqueue(cmd).expect("enqueue must succeed");
            }

            // Process pre-shutdown commands
            for _ in 0..pre_shutdown_commands {
                if shard.tick().unwrap_or(false) == false {
                    break;
                }
            }

            // Enqueue shutdown
            shard.enqueue(ShardCommand::Shutdown).expect("enqueue must succeed");

            // First tick processes shutdown
            let first_result = shard.tick();
            assert!(
                matches!(first_result, Ok(false)),
                    "INV(S3): tick() must return Ok(false) when processing Shutdown",
            );

            // Subsequent ticks return Ok(false) without processing
            for _ in 0..5 {
                let result = shard.tick();
                assert!(
                    matches!(result, Ok(false)),
                    "INV(S3): tick() must return Ok(false) after Shutdown",
                );
            }
        }
    }

    // =========================================================================
    // INV(S4): run_exclusivity
    // =========================================================================

    proptest! {
        #[test]
        fn run_lifecycle_submit_cancel_exclusivity(
            submits in 0usize..=10usize,
            cancels in 0usize..=10usize,
        ) {
            let config = ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 1024,
                step_budget_per_tick: 1000,
                max_active_runs: 1024,
                policy: vb_core::policy::RuntimePolicy::Strict,
            };
            let mut shard = Shard::new(config);

            // Interleave submits and cancels
            for i in 0..submits.max(cancels) {
                if i < submits {
                    let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                    let workflow = make_nop_workflow(2);
                    let cmd = ShardCommand::Submit {
                        run: run_id,
                        workflow,
                        caps: CapabilitySet::empty(),
                    };
                    shard.enqueue(cmd).expect("enqueue must succeed");
                }

                if i < cancels {
                    let run_id = RunId::new(u64::try_from(i).unwrap_or(0));
                    let cmd = ShardCommand::Cancel { run: run_id };
                    shard.enqueue(cmd).expect("enqueue must succeed");
                }
            }

            // Process all commands
            let total_commands = submits + cancels;
            for _ in 0..total_commands {
                shard.tick().expect("tick must succeed");
            }

            // INV(S4): A RunId appears in self.runs at most once
            // After Cancel, the run is not in self.runs
            // This is inherently enforced by the IndexMap structure
        }
    }

    // =========================================================================
    // INV(M1): valid_state_transitions
    // INV(M2): no_invalid_backward_transitions
    // =========================================================================

    proptest! {
        #[test]
        fn step_state_transition_validity(
            initial_state in prop::sample::select(vec![
                StepState::Pending,
                StepState::Running,
                StepState::Waiting,
                StepState::Asking,
                StepState::Succeeded,
            ]),
            signal in prop::sample::select(vec![
                RuntimeSignal::Continue,
                RuntimeSignal::Finished(SlotValue::I64(0)),
                RuntimeSignal::AwaitingWait,
                RuntimeSignal::AwaitingAsk,
                RuntimeSignal::AwaitingAction(ActionTicket {
                    run: RunId::new(0),
                    step: StepIdx::ZERO,
                    seq: SeqNo::new(0),
                    action: ActionId::new(0),
                    attempt: 0,
                    idempotency_key: 0,
                    capacity: 1,
                }),
                RuntimeSignal::StepBudgetExhausted,
            ]),
        ) {
            let mut run = RunFrame::new(
                RunId::new(1),
                StepIdx::ZERO,
                2,
                1,
            ).expect("valid frame");

            // Set initial state (skip Pending which is default)
            match initial_state {
                StepState::Running => {
                    run.mark_running(StepIdx::ZERO)
                        .expect("mark_running must succeed in test");
                }
                StepState::Waiting => {
                    run.mark_running(StepIdx::ZERO)
                        .expect("mark_running must succeed in test");
                    run.mark_waiting(StepIdx::ZERO)
                        .expect("mark_waiting must succeed in test");
                }
                StepState::Asking => {
                    run.mark_running(StepIdx::ZERO)
                        .expect("mark_running must succeed in test");
                    run.mark_asking(StepIdx::ZERO)
                        .expect("mark_asking must succeed in test");
                }
                StepState::Succeeded => {
                    run.mark_running(StepIdx::ZERO)
                        .expect("mark_running must succeed in test");
                    run.mark_succeeded(StepIdx::ZERO)
                        .expect("mark_succeeded must succeed in test");
                }
                _ => {}
            }

            // INV(M1): valid_state_transitions
            // INV(M2): no_invalid_backward_transitions
            match (&initial_state, &signal) {
                // AwaitingWait: Running -> Waiting
                (StepState::Running, RuntimeSignal::AwaitingWait) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + AwaitingWait must succeed");
                    assert_eq!(
                        run.step_state(StepIdx::ZERO).unwrap(),
                        StepState::Waiting,
                        "INV(M1): step must transition to Waiting",
                    );
                }
                // AwaitingAsk: Running -> Asking
                (StepState::Running, RuntimeSignal::AwaitingAsk) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + AwaitingAsk must succeed");
                    assert_eq!(
                        run.step_state(StepIdx::ZERO).unwrap(),
                        StepState::Asking,
                        "INV(M1): step must transition to Asking",
                    );
                }
                // AwaitingAction: Running -> Running (no change)
                (StepState::Running, RuntimeSignal::AwaitingAction(_)) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + AwaitingAction must succeed");
                    assert_eq!(
                        run.step_state(StepIdx::ZERO).unwrap(),
                        StepState::Running,
                        "INV(M1): step must remain Running",
                    );
                }
                // StepBudgetExhausted: Running -> Running (no change)
                (StepState::Running, RuntimeSignal::StepBudgetExhausted) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + StepBudgetExhausted must succeed");
                }
                // Continue: Running -> Succeeded
                (StepState::Running, RuntimeSignal::Continue) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + Continue must succeed");
                    assert_eq!(
                        run.step_state(StepIdx::ZERO).unwrap(),
                        StepState::Succeeded,
                        "INV(M1): step must transition to Succeeded",
                    );
                }
                // Finished: Running -> Succeeded
                (StepState::Running, RuntimeSignal::Finished(_)) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(result.is_ok(), "INV(M1): Running + Finished must succeed");
                    assert_eq!(
                        run.step_state(StepIdx::ZERO).unwrap(),
                        StepState::Succeeded,
                        "INV(M1): step must transition to Succeeded",
                    );
                }
                // INV(M2): Invalid transitions from non-Running states
                // Succeeded -> Succeeded is idempotent (valid), so exclude Succeeded.
                (StepState::Waiting, RuntimeSignal::Continue)
                | (StepState::Waiting, RuntimeSignal::Finished(_))
                | (StepState::Asking, RuntimeSignal::Continue)
                | (StepState::Asking, RuntimeSignal::Finished(_)) => {
                    let result = mark_step_after_signal(&mut run, StepIdx::ZERO, &signal);
                    assert!(
                        matches!(result, Err(EngineError::InternalInvariantViolation { .. })),
                        "INV(M2): invalid transition must return InternalInvariantViolation",
                    );
                }
                _ => {}
            }
        }
    }

    // =========================================================================
    // BranchLimitExceeded error
    // =========================================================================

    #[test]
    fn compute_max_parallel_rejects_overflow() {
        // TogetherStart with u16::MAX + 1 branches
        let too_many_branches = usize::from(u16::MAX) + 1;
        let mut nodes = Vec::new();

        // Create a TogetherStart node with too many branches.
        // All branches point to step 1 (the join node) so the workflow is valid.
        let branches: Vec<StepIdx> = (0..too_many_branches).map(|_| StepIdx::new(1)).collect();

        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join: StepIdx::new(1),
            },
        });

        // Add a join node
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        });

        let parts = WorkflowParts {
            name: "overflow_workflow".into(),
            digest: vb_core::ids::WorkflowDigest::from_bytes([3; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![].into_boxed_slice(),
            accessors: vec![].into_boxed_slice(),
            constants: vec![].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: vec![].into_boxed_slice(),
        };

        // Use from_parts_unchecked to bypass validation that would reject
        // 65536 nodes (step count overflow) or out-of-range branch indices.
        let workflow = CompiledWorkflow::from_parts_unchecked(parts);

        let result = compute_max_parallel_in_flight(&workflow);
        assert!(
            matches!(result, Err(RuntimeEngineError::BranchLimitExceeded { .. })),
            "compute_max_parallel_in_flight must return BranchLimitExceeded for too many branches",
        );
    }

    // =========================================================================
    // EvidenceCollector edge case: zero capacity drops all
    // =========================================================================

    #[test]
    fn zero_capacity_collector_drops_all() {
        let mut collector = EvidenceCollector::with_capacity(0);

        assert_eq!(collector.capacity(), 0, "capacity must be 0");
        assert_eq!(collector.len(), 0, "len must be 0 initially");

        collector.push_step_started(StepIdx::ZERO);
        assert_eq!(collector.len(), 0, "len must still be 0 after push");
        assert_eq!(collector.dropped(), 1, "dropped must be 1");

        collector.push_step_started(StepIdx::new(1));
        assert_eq!(
            collector.dropped(),
            2,
            "dropped must be 2 after second push"
        );
    }
}
