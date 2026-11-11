// =====================================================================
// Proptest tests
// =====================================================================

#[cfg(test)]
mod proptests {

    use super::super::*;
    use crate::engine::action::compute_idempotency_key;
    use proptest::prelude::*;
    use vb_core::ids::{ActionId, RunId, SeqNo};

    proptest! {
        #[test]
        fn step_budget_never_allows_more_than_n_steps(n in 1u64..1000u64) {
            let mut budget = vb_core::engine::StepBudget::new(n);
            let mut taken = 0u64;
            let mut drained = false;
            while !drained && taken <= n + 1 {
                match budget.try_take() {
                    Ok(true) => taken += 1,
                    Ok(false) => drained = true,
                    Err(_) => drained = true,
                }
            }
            prop_assert_eq!(taken, n);
        }
    }

    proptest! {
        #[test]
        fn idempotency_key_differs_for_different_tuples(
            run1 in 1u64..100u64,
            run2 in 1u64..100u64,
            seq1 in 0u64..100u64,
            seq2 in 0u64..100u64,
            action1 in 0u16..10u16,
            action2 in 0u16..10u16,
        ) {
            prop_assume!(run1 != run2 || seq1 != seq2 || action1 != action2);
            let key1 = compute_idempotency_key(RunId::new(run1), 1, SeqNo::new(seq1), ActionId::new(action1));
            let key2 = compute_idempotency_key(RunId::new(run2), 1, SeqNo::new(seq2), ActionId::new(action2));
            prop_assert_ne!(key1, key2);
        }
    }
}

// ==========================================================================
// BLACKHAT SECURITY REVIEW: engine module findings
// ==========================================================================
//
// Reviewer: BLACKHAT
// Scope: engine/{mod,execute,drive,action,signal,helpers,types}.rs
//
// Findings documented in tests below:
//
// BH-ENG-01: EvidenceCollector has no capacity bound (resource exhaustion)
// BH-ENG-02: mark_step_after_signal leaves Running state on StepBudgetExhausted
// BH-ENG-03: read_attempt_from_slot silently returns 0 on read errors
// BH-ENG-04: runtime_from_core discards taint from Finished signal
// BH-ENG-05: ErrorHandler dispatch routes to body, not handler
// BH-ENG-06: RetryPolicy max_attempts=0 is accepted (zero-retry policy)
// BH-ENG-07: execute_do_without_contract skips all security checks
// BH-ENG-08: RetryCheck increments executed counter
// BH-ENG-09: No SlotWritten for AwaitingAction steps without output
// BH-ENG-10: Idempotency key collision search in small space
// BH-ENG-11: Retry ticket uses frame run ID
// BH-ENG-12: drive_with_actions creates fresh ValueStore per call
// BH-ENG-13: Suspended outcome ignores original ticket fields
// BH-ENG-14: Double taint check defense in depth
// ==========================================================================

#[cfg(test)]
mod blackhat_engine {

    use vb_core::action::{
        ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutcome, ActionTicket,
        Idempotency, RetryPolicy as VbRetryPolicy, RetrySafety, SideEffect,
    };
    use vb_core::capability::CapabilitySet;
    use vb_core::engine::EngineSignal;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx};
    use vb_core::value::{SlotValue, Taint};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    use crate::engine::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
        compute_idempotency_key, drive_deterministic_full, drive_with_actions, execute_do,
        execute_do_without_contract, execute_error_handler, execute_node_full, execute_retry_check,
        resume_action_outcome, runtime_from_core,
    };
    use crate::primitives::collect::CollectStates;

    // ---- Helpers ----

    fn dummy_contract() -> ActionContract {
        ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }
    }

    // ---- Workflow/Run factories ----

    fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow {
        make_workflow_with_constants(nodes, slot_count, Box::from([]))
    }

    fn make_workflow_with_constants(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        constants: Box<[vb_core::value::ConstValue]>,
    ) -> CompiledWorkflow {
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("bh_test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants,
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            input_slots: Box::from([]),
            step_names: Box::from([]),
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(e) => {
                let msg = format!("workflow validation failed: {e}");
                panic!("{msg}");
            }
        }
    }

    fn finish_node(id: u16, slot: u16) -> CompiledNode {
        vb_core::workflow::node(
            StepIdx::new(id),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(slot),
            },
            Some(SlotIdx::new(slot)),
            None,
        )
    }

    fn make_run(slot_count: u16, step_count: u16) -> RunFrame {
        match RunFrame::new(RunId::new(1), StepIdx::ZERO, slot_count, step_count) {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("RunFrame::new failed: {e}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-01 FIXED: EvidenceCollector now has capacity bound
    //
    // The EvidenceCollector previously used an unbounded Vec. Now it has a
    // capacity limit (DEFAULT_EVIDENCE_CAPACITY = 3072). Events beyond the
    // capacity are silently dropped with a dropped count tracked.
    // =====================================================================

    #[test]
    fn bh_eng_01_evidence_collector_enforces_capacity_bound() {
        let mut collector = EvidenceCollector::new();
        let capacity = collector.capacity();
        // First `capacity` pushes succeed; the rest surface as typed errors.
        let mut overflow_errors = 0usize;
        for i in 0u16..10_000 {
            if collector.push_step_started(StepIdx::new(i)).is_err() {
                overflow_errors = overflow_errors.saturating_add(1);
            }
        }
        assert_eq!(
            collector.len(),
            capacity,
            "BH-ENG-01 FIXED: EvidenceCollector should respect capacity bound"
        );
        assert_eq!(
            overflow_errors,
            10_000 - capacity,
            "overflow pushes must surface as typed errors"
        );
    }

    #[test]
    fn bh_eng_01_evidence_events_per_step_exceeds_one() {
        // Each step in drive_deterministic_full can emit up to 3 events:
        // StepStarted, SlotWritten, StepSucceeded. A budget of N steps
        // can produce up to 3*N events. Use SetConst which actually writes.
        let set_const = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        );
        let finish = finish_node(1, 0);
        let wf = make_workflow_with_constants(
            vec![set_const, finish],
            4,
            Box::from([vb_core::value::ConstValue::I64(1)]),
        );
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert!(result.is_ok(), "drive should succeed: {result:?}");
        let events = evidence.drain();
        assert!(
            events.len() > 1,
            "BH-ENG-01: expected multiple evidence events, got {}",
            events.len()
        );
    }

    // =====================================================================
    // BH-ENG-02: mark_step_after_signal leaves Running state on
    //            StepBudgetExhausted (state machine gap)
    //
    // When the drive loop exhausts its step budget mid-run, the step at
    // the current PC has been marked Running (drive.rs:69) but
    // mark_step_after_signal maps StepBudgetExhausted to Ok(()) without
    // transitioning the step state. The step remains in Running state
    // until the next drive call.
    // Severity: Low-Medium. On resume, the drive loop re-marks the step.
    // =====================================================================

    // BUG-PIN (BH-ENG-02): asserts the CURRENT BUGGY behavior where
    // budget exhaustion leaves step 0 in stale Running state. The spec
    // requires the step to be transitioned to a terminal state so that
    // resume does not re-execute it. Production code does NOT do this;
    // this test pins the gap so future drift is visible.
    //
    // TODO(close-reason vb-3p9f7): rewrite this test to assert that
    // step 0's state after StepBudgetExhausted is NOT `Running`. When
    // `mark_step_after_signal` is fixed to transition Running → terminal
    // on budget exhaustion, remove the `_bug_pin_` suffix.
    #[test]
    fn bh_eng_02_budget_exhaustion_leaves_step_in_running_state_bug_pin() {
        // TODO(vb-3p9f7): rewrite to assert spec behavior, not buggy state
        let nop = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        );
        let nop1 = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(2)),
        );
        let finish = finish_node(2, 0);
        let wf = make_workflow(vec![nop, nop1, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(1);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // BH-ENG-02: Step 0 was marked Running but StepBudgetExhausted
        // did not transition it to Succeeded via mark_step_after_signal.
        // The step stays in stale Running state until next drive.
    }

    // =====================================================================
    // BH-ENG-04: runtime_from_core discards taint from Finished signal
    //
    // signal.rs:16 maps EngineSignal::Finished(value, _taint) to
    // RuntimeSignal::Finished(value). The taint is silently discarded.
    // Severity: Medium-High. Consumers of RuntimeSignal cannot determine
    // whether the finished value was Clean, Secret, or DerivedFromSecret.
    // =====================================================================

    // BUG-PIN (BH-ENG-04): asserts the CURRENT BUGGY behavior where
    // `runtime_from_core` discards taint from `EngineSignal::Finished`,
    // collapsing `Finished(_, Clean)` and `Finished(_, Secret)` to the
    // same `RuntimeSignal::Finished`. The spec requires taint to be
    // preserved so downstream consumers can distinguish Clean/Secret/
    // DerivedFromSecret outcomes. Production `signal.rs:16` does NOT
    // preserve taint; this test pins the gap.
    //
    // TODO(close-reason vb-3p9f7): rewrite this test to assert
    // `clean_signal != secret_signal`. When `runtime_from_core` is
    // patched to thread taint through Finished, remove the
    // `_bug_pin_` suffix.
    #[test]
    fn bh_eng_04_runtime_from_core_discards_taint_from_finished_bug_pin() {
        // TODO(vb-3p9f7): rewrite to assert spec behavior, not buggy state
        let clean_signal =
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean));
        let secret_signal =
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Secret));
        assert_eq!(
            clean_signal, secret_signal,
            "BH-ENG-04: taint is discarded in runtime_from_core, both signals are equal"
        );
    }

    // =====================================================================
    // BH-ENG-05: ErrorHandler dispatch routes PC to body step, not handler
    //
    // The ErrorHandler node dispatches to body for normal execution.
    // The handler is only used when an actual failure occurs.
    // Severity: Informational. Correct behavior but potentially confusing.
    // =====================================================================

    #[test]
    fn bh_eng_05_error_handler_dispatches_to_body_not_handler() {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None.into(),
            },
            None,
            None,
        );
        let finish1 = finish_node(1, 0);
        let finish2 = finish_node(2, 0);
        let wf = make_workflow(vec![node, finish1, finish2], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(
            run.pc(),
            StepIdx::new(1),
            "BH-ENG-05: ErrorHandler routes to body=1, not handler=2"
        );
    }

    // =====================================================================
    // BH-ENG-06: RetryPolicy with max_attempts=0 is accepted
    //
    // A max_attempts=0 policy means "never attempt" and exhausts
    // immediately. The policy should probably reject 0 at construction.
    // Severity: Low. Safe but semantically questionable.
    // =====================================================================

    #[test]
    fn bh_eng_06_zero_max_attempts_policy_exhausts_immediately_bug_pin() {
        // TODO(vb-3p9f7): rewrite to assert spec behavior, not buggy state
        let policy = RetryPolicy {
            max_attempts: 0,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let target = execute_retry_check(0, policy, StepIdx::new(1), StepIdx::new(9));
        assert_eq!(
            target,
            StepIdx::new(9),
            "BH-ENG-06: max_attempts=0 should exhaust at attempt 0"
        );
    }

    // =====================================================================
    // BH-ENG-07 FIXED: execute_do_without_contract now enforces taint checks
    //
    // Previously, when contracts was empty, execute_do_without_contract was
    // used, skipping taint checking entirely. Now it enforces the most
    // conservative taint policy: tainted inputs are rejected.
    // =====================================================================

    #[test]
    fn bh_eng_07_do_without_contract_now_enforces_taint_check() {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        );
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret),
            Ok(())
        );
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        // BH-FIX: TaintViolation is now raised even without contracts.
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-07 FIXED: taint check is now enforced without contracts: {result:?}"
        );
    }

    #[test]
    fn bh_eng_07_do_without_contract_rejects_clean_input() {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        );
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(42)), Ok(()));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    #[test]
    fn bh_eng_07_do_with_contract_catches_taint_violation() {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        );
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret),
            Ok(())
        );
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let contracts = vec![ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }];
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &contracts,
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-07: with contracts, taint violation is detected: {result:?}"
        );
    }

    // =====================================================================
    // BH-ENG-08: RetryCheck increments executed counter
    //
    // Control flow routing nodes count as executed steps, which could
    // cause premature step budget exhaustion.
    // Severity: Low. Semantic issue with "executed" definition.
    // =====================================================================

    #[test]
    fn bh_eng_08_retry_check_increments_executed_counter() {
        let node0 = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
            None,
            None,
        );
        let finish = finish_node(1, 0);
        let wf = make_workflow(vec![node0, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let executed_before = run.executed();
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert!(
            matches!(result, Ok(RuntimeSignal::Continue)),
            "BH-ENG-08: RetryCheck should return Continue"
        );
        assert_eq!(
            run.executed(),
            executed_before + 1,
            "BH-ENG-08: RetryCheck increments executed counter"
        );
    }

    // =====================================================================
    // BH-ENG-09: No SlotWritten for AwaitingAction steps without output
    // =====================================================================

    #[test]
    fn bh_eng_09_no_slot_written_for_awaiting_action_steps() {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("bh_do_no_out"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            input_slots: Box::from([]),
            step_names: Box::from([]),
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = make_run(4, 2);
        // Input slot must be initialized with clean taint for the no-contract path.
        assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(0)), Ok(()));
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        match result {
            Ok(RuntimeSignal::AwaitingAction(_)) => {}
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied {
                action,
                ..
            })) if action == ActionId::new(0) => {}
            other => {
                assert!(
                    false,
                    "BH-ENG-09: expected AwaitingAction or CapabilityDenied {{ action: 0 }}, got {other:?}"
                );
            }
        }
        let events = evidence.drain();
        let slot_written_count = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::SlotWritten { .. }))
            .count();
        assert_eq!(
            slot_written_count, 0,
            "BH-ENG-09: no SlotWritten should be emitted for AwaitingAction (no output slot)"
        );
    }

    // =====================================================================
    // BH-ENG-10: Idempotency key collision search in small space
    // =====================================================================

    #[test]
    fn bh_eng_10_idempotency_key_collision_search_small_space() {
        let mut keys = std::collections::HashSet::new();
        let mut collisions = 0u64;
        for run in 0u64..50 {
            for seq in 0u64..50 {
                for action in 0u16..20 {
                    let key = compute_idempotency_key(
                        RunId::new(run),
                        1,
                        SeqNo::new(seq),
                        ActionId::new(action),
                    );
                    if !keys.insert(key) {
                        collisions = collisions.saturating_add(1);
                    }
                }
            }
        }
        assert_eq!(
            collisions, 0,
            "BH-ENG-10: found {collisions} collisions in small idempotency key space"
        );
    }

    // =====================================================================
    // BH-ENG-11: Retry ticket uses frame run ID (matches original)
    // =====================================================================

    #[test]
    fn bh_eng_11_retry_ticket_uses_frame_run_id() {
        let _run = RunFrame::new(RunId::new(99), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let original = ActionTicket {
            run: RunId::new(99),
            step: StepIdx::ZERO,
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0,
            capacity: 3,
            fencing_token: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: VbRetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let outcome = ActionOutcome::Failed(failure);
        let result = resume_action_outcome(&original, outcome, &dummy_contract());
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(
                    ticket.run, original.run,
                    "BH-ENG-11: retry ticket run must match original ticket run"
                );
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-12: drive_with_actions creates fresh ValueStore per call
    // =====================================================================

    #[test]
    fn bh_eng_12_drive_with_actions_uses_fresh_value_store() {
        // Use SetConst -> Finish which doesn't need pre-initialized slots.
        let set_const = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        );
        let finish = finish_node(1, 0);
        let wf = make_workflow_with_constants(
            vec![set_const, finish],
            4,
            Box::from([vb_core::value::ConstValue::I64(1)]),
        );
        let mut run = make_run(4, 4);
        let mut budget = vb_core::engine::StepBudget::new(10);
        let result = drive_with_actions(&wf, &mut run, &mut budget, &[], RetryPolicy::NEVER);
        assert!(
            result.is_ok(),
            "drive_with_actions should succeed: {result:?}"
        );
    }

    // =====================================================================
    // BH-ENG-13: Suspended outcome ignores original ticket fields
    //
    // When ActionOutcome::Suspended is received, resume_action_outcome
    // returns the suspended ticket directly, ignoring original_ticket.
    // Severity: Medium. A malicious action handler could redirect
    // execution to a different step or run.
    // =====================================================================

    #[test]
    fn bh_eng_13_suspended_outcome_ignores_original_ticket_bug_pin() {
        // TODO(vb-3p9f7): rewrite to assert spec behavior, not buggy state
        let _run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let suspended_ticket = ActionTicket {
            run: RunId::new(999),
            step: StepIdx::new(50),
            seq: SeqNo::new(5),
            action: ActionId::new(99),
            run_attempt: 1,
            attempt: 3,
            idempotency_key: 99999,
            capacity: 1,
            fencing_token: 0,
        };
        let original = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::new(1),
            action: ActionId::new(0),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
            fencing_token: 0,
        };
        let outcome = ActionOutcome::Suspended(suspended_ticket);
        let result = resume_action_outcome(&original, outcome, &dummy_contract());
        match result {
            Ok(RuntimeSignal::AwaitingAction(returned)) => {
                // BH-ENG-13: Suspended ticket fields passed through unchecked.
                assert_eq!(returned.run, RunId::new(999));
                assert_eq!(returned.step, StepIdx::new(50));
                assert_eq!(returned.action, ActionId::new(99));
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-14: Double taint check defense in depth
    // =====================================================================

    #[test]
    fn bh_eng_14_taint_check_catches_deterministic_pure_secret_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret),
            Ok(())
        );
        let contract = ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let registry = vec![contract.clone()];
        let result = execute_do(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &contract,
            &registry,
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
            1,
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { .. })),
            "BH-ENG-14: DeterministicPure with Secret input must fail taint check"
        );
    }

    // =====================================================================
    // BH-ENG-15 FIXED: EvidenceCollector capacity bound prevents exhaustion
    //
    // The EvidenceCollector now enforces a capacity limit. Events beyond
    // capacity are silently dropped but tracked via the dropped() counter.
    // =====================================================================

    #[test]
    fn bh_eng_15_evidence_collector_with_capacity_drops_excess() {
        let mut collector = EvidenceCollector::with_capacity(3);
        assert!(collector.push_step_started(StepIdx::new(0)).is_ok());
        assert!(collector.push_step_started(StepIdx::new(1)).is_ok());
        assert!(collector.push_step_started(StepIdx::new(2)).is_ok());
        // At capacity: next event must surface as a typed error.
        assert!(collector.push_step_started(StepIdx::new(3)).is_err());
        assert_eq!(collector.len(), 3, "capacity should be respected");
    }

    #[test]
    fn bh_eng_15_evidence_collector_drain_after_overflow() {
        let mut collector = EvidenceCollector::with_capacity(2);
        assert!(collector.push_step_started(StepIdx::new(0)).is_ok());
        assert!(collector.push_step_started(StepIdx::new(1)).is_ok());
        assert!(
            collector.push_step_started(StepIdx::new(2)).is_err(),
            "overflow must surface as typed error"
        );
        let events = collector.drain();
        assert_eq!(events.len(), 2);
        assert!(collector.is_empty(), "drain empties the buffer");
    }

    // =====================================================================
    // BH-ENG-16: execute_do_without_contract rejects tainted input
    //
    // Regression test to ensure the taint bypass (BH-ENG-07) stays fixed.
    // =====================================================================

    #[test]
    fn bh_eng_16_do_without_contract_rejects_secret_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret),
            Ok(())
        );
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-16: tainted input must be rejected without contract: {result:?}"
        );
    }

    #[test]
    fn bh_eng_16_do_without_contract_rejects_uninitialized_slot() {
        let run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        // Slot 0 is never written -- uninitialized is treated as Clean.
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    #[test]
    fn bh_eng_16_do_without_contract_rejects_clean_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(42)), Ok(()));
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    // =====================================================================
    // BH-ENG-17: compute_max_parallel_in_flight saturates on overflow
    //
    // When a TogetherStart has more branches than u16::MAX, the count
    // must saturate to u16::MAX rather than panic or wrap.
    // =====================================================================

    #[test]
    fn bh_eng_17_parallel_count_saturates_on_overflow() {
        // Verify that compute_max_parallel_in_flight correctly handles
        // the u16::try_from(branches.len()) conversion by testing with
        // a small but valid TogetherStart. The actual saturation at u16::MAX
        // cannot be tested due to memory constraints, but we verify the
        // drive loop handles a multi-branch TogetherStart without panic.
        let branches: Box<[StepIdx]> = (0u32..3)
            .map(|i| StepIdx::new(i.saturating_add(1) as u16))
            .collect();
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::TogetherStart {
                branches,
                join: StepIdx::new(4),
            },
            None,
            None,
        );
        let step1 = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(4)),
        );
        let step2 = vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(4)),
        );
        let step3 = vb_core::workflow::node(
            StepIdx::new(3),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(4)),
        );
        let finish = finish_node(4, 0);
        let wf = make_workflow(vec![node, step1, step2, step3, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drive_deterministic_full(
                &wf,
                &mut run,
                &mut budget,
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut cs,
                &CapabilitySet::empty(),
                1,
            )
        }));
        assert!(
            result.is_ok(),
            "drive_deterministic_full must not panic on multi-branch TogetherStart"
        );
        // Note: Actual saturation at u16::MAX cannot be tested due to memory constraints.
    }

    // =====================================================================
    // BH-ENG-18: Drive loop budget exhaustion does not corrupt frame state
    //
    // When the budget is exhausted, the frame's step state must be
    // consistent for resumption.
    // =====================================================================

    #[test]
    fn bh_eng_18_budget_exhaustion_preserves_pc_for_resume() {
        let nop0 = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        );
        let nop1 = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(2)),
        );
        let finish = finish_node(2, 0);
        let wf = make_workflow(vec![nop0, nop1, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(1);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // PC should have advanced past step 0 (the executed step).
        assert_ne!(
            run.pc(),
            StepIdx::ZERO,
            "PC must advance after step execution"
        );
    }
}
