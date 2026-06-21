    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    use vb_core::frame::RunFrame;

    use crate::primitives::collect::CollectStates;
    use crate::shard::types::RunState;

    use super::reject_taint_downgrade;

    fn test_action_contract(idempotency: Idempotency) -> ActionContract {
        let name = ActionName::new("test_action").expect("valid action name");
        ActionContract {
            id: ActionId::new(0),
            name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5_000,
            idempotency,
            side_effect: SideEffect::Pure,
            retry_safety: RetrySafety::Idempotent,
            required_capabilities: Box::new([]),
        }
    }

    fn test_workflow() -> Option<CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("test_wf"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
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
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn test_run_state(taint: Taint) -> RunState {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, 1)
            .expect("frame creation");
        frame
            .write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), taint)
            .expect("slot write");
        let workflow = test_workflow().expect("workflow creation");
        let contract = test_action_contract(Idempotency::DeterministicPure);
        RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::new(),
            action_attempts: crate::shard::helpers::new_action_attempts(1),
            admission: None,
            collect_states: CollectStates::new(),
            action_contracts: Box::from([contract]),
            last_snapshot_executed: 0,
        }
    }

    // ── DeterministicPure taint rejection ─────────────────────────────────

    #[test]
    fn deterministicpure_with_clean_input_passes_taint_check() {
        let state = test_run_state(Taint::Clean);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::Clean);
        assert_eq!(
            result,
            Ok(()),
            "DeterministicPure with Clean input must pass taint check"
        );
    }

    #[test]
    fn deterministicpure_with_secret_input_returns_taintviolation() {
        let state = test_run_state(Taint::Secret);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::Secret);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(supplied, Taint::Secret, "supplied must be Secret");
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, Secret), got {other:?}"
            ),
        }
    }

    #[test]
    fn deterministicpure_with_derivedfromsecret_input_returns_taintviolation() {
        let state = test_run_state(Taint::DerivedFromSecret);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::DerivedFromSecret);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(
                    supplied,
                    Taint::DerivedFromSecret,
                    "supplied must be DerivedFromSecret"
                );
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, DerivedFromSecret), got {other:?}"
            ),
        }
    }

    #[test]
    fn deterministicpure_with_random_input_returns_taintviolation() {
        let state = test_run_state(Taint::Secret);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::Secret);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(supplied, Taint::Secret, "supplied must be Random");
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, Random), got {other:?}"
            ),
        }
    }

    #[test]
    fn deterministicpure_with_timedependent_input_returns_taintviolation() {
        let state = test_run_state(Taint::Secret);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::Secret);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(
                    supplied,
                    Taint::Secret,
                    "supplied must be TimeDependent"
                );
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, TimeDependent), got {other:?}"
            ),
        }
    }

    // ── Regression: other idempotency levels ──────────────────────────────

    #[test]
    fn atleastonceexternal_with_secret_input_passes_taint_check() {
        // AtLeastOnceExternal actions may receive non-Clean input
        // and should not be rejected by the downgrade check.
        let mut frame = RunFrame::new(RunId::new(2), StepIdx::ZERO, 1, 1)
            .expect("frame creation");
        frame
            .write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(99), Taint::Secret)
            .expect("slot write");
        let workflow = test_workflow().expect("workflow creation");
        let contract = test_action_contract(Idempotency::AtLeastOnceExternal);
        let state = RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::new(),
            action_attempts: crate::shard::helpers::new_action_attempts(1),
            admission: None,
            collect_states: CollectStates::new(),
            action_contracts: Box::from([contract.clone()]),
            last_snapshot_executed: 0,
        };
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::DerivedFromSecret);
        assert_eq!(
            result,
            Ok(()),
            "AtLeastOnceExternal with Secret input must pass taint check"
        );
    }

    #[test]
    fn atleastonceexternal_taint_downgrade_rejected_by_join_path() {
        // When input_taint=DerivedFromSecret but supplied=Clean,
        // the join_taint downgrade path (lines 144-149 of chunk_003.rs)
        // must reject the completion.
        let mut frame = RunFrame::new(RunId::new(3), StepIdx::ZERO, 1, 1)
            .expect("frame creation");
        frame
            .write_slot_with_taint(
                SlotIdx::ZERO,
                SlotValue::I64(1),
                Taint::DerivedFromSecret,
            )
            .expect("slot write");
        let workflow = test_workflow().expect("workflow creation");
        let contract = test_action_contract(Idempotency::AtLeastOnceExternal);
        let state = RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::new(),
            action_attempts: crate::shard::helpers::new_action_attempts(1),
            admission: None,
            collect_states: CollectStates::new(),
            action_contracts: Box::from([contract.clone()]),
            last_snapshot_executed: 0,
        };
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::Clean);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(
                    required,
                    Taint::DerivedFromSecret,
                    "required must be DerivedFromSecret"
                );
                assert_eq!(supplied, Taint::Clean, "supplied must be Clean");
            }
            other => panic!(
                "expected ActionTaintDowngrade(DerivedFromSecret, Clean), got {other:?}"
            ),
        }
    }

    #[test]
    fn deterministicpure_reports_frame_input_taint_not_supplied_param() {
        // When supplied != input_taint, the error's `supplied` field must
        // reflect the frame's taint (the violation source), not the parameter.
        let state = test_run_state(Taint::Secret);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let input_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
        let result = reject_taint_downgrade(input_taint, &contract, Taint::DerivedFromSecret);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(
                    supplied, Taint::Secret,
                    "supplied should be the frame's input_taint (Secret), not the parameter"
                );
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, Secret), got {other:?}"
            ),
        }
    }

    // ── Proptest properties ──────────────────────────────────────────────

    mod proptests {
        use proptest::prelude::*;
        use vb_core::value::Taint;
        use vb_core::action::Idempotency;

        use super::super::reject_taint_downgrade;
        use super::super::super::types::RunState;
        use super::{test_action_contract, test_workflow};
        use vb_core::frame::RunFrame;
        use vb_core::ids::{RunId, SlotIdx, StepIdx};
        use vb_core::value::SlotValue;
        use crate::primitives::collect::CollectStates;
        use crate::RuntimeError;

        fn all_taints() -> impl Strategy<Value = Taint> {
            prop_oneof![
                Just(Taint::Clean),
                Just(Taint::DerivedFromSecret),
                Just(Taint::Secret),
                Just(Taint::Secret),
                Just(Taint::Secret),
            ]
        }

        fn all_idempotencies() -> impl Strategy<Value = Idempotency> {
            prop_oneof![
                Just(Idempotency::DeterministicPure),
                Just(Idempotency::IdempotentExternal),
                Just(Idempotency::AtLeastOnceExternal),
            ]
        }

        fn make_proptest_run_state(input_taint: Taint) -> RunState {
            let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, 1)
                .expect("frame creation");
            frame
                .write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), input_taint)
                .expect("slot write");
            let workflow = test_workflow().expect("workflow creation");
            let contract = test_action_contract(Idempotency::DeterministicPure);
            RunState {
                frame,
                workflow,
                store: vb_core::value_store::ValueStore::new(),
                action_attempts: crate::shard::helpers::new_action_attempts(1),
                admission: None,
                collect_states: CollectStates::new(),
                action_contracts: Box::from([contract]),
            last_snapshot_executed: 0,
            }
        }

        proptest! {
            #[test]
            fn prop_deterministicpure_rejects_all_non_clean(
                input_taint in all_taints(),
            ) {
                prop_assume!(input_taint != Taint::Clean);
                let state = make_proptest_run_state(input_taint);
                let contract = test_action_contract(Idempotency::DeterministicPure);
                let frame_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
                let result = reject_taint_downgrade(frame_taint, &contract, Taint::Clean);
                match result {
                    Ok(()) => prop_assert!(false, "DeterministicPure + {input_taint:?} input must reject, got Ok(())"),
                    Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                        prop_assert_eq!(required, Taint::Clean);
                        prop_assert_eq!(supplied, input_taint);
                    }
                    other => panic!("unexpected error variant: {other:?}"),
                }
            }

            #[test]
            fn prop_clean_input_passes_for_all_idempotencies(
                idempotency in all_idempotencies(),
                supplied in all_taints(),
            ) {
                let state = make_proptest_run_state(Taint::Clean);
                let contract = test_action_contract(idempotency);
                let frame_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
                let result = reject_taint_downgrade(frame_taint, &contract, supplied);
                if let Err(RuntimeError::ActionTaintDowngrade { required, .. }) = &result {
                    prop_assert_ne!(*required, Taint::Clean,
                        "guard must not fire on Clean input (idem={:?})", idempotency);
                }
            }

            #[test]
            fn prop_atleastonceexternal_nonclean_input_does_not_require_clean_output(
                input_taint in all_taints(),
                supplied in all_taints(),
            ) {
                let state = make_proptest_run_state(input_taint);
                let contract = test_action_contract(Idempotency::AtLeastOnceExternal);
                let frame_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
                let result = reject_taint_downgrade(frame_taint, &contract, supplied);
                if let Err(RuntimeError::ActionTaintDowngrade { required, .. }) = &result {
                    prop_assert_ne!(*required, Taint::Clean,
                        "AtLeastOnceExternal guard must not fire for Clean-required (input={:?})", input_taint);
                }
            }

            #[test]
            fn prop_guard_reports_input_taint_not_supplied_param(
                input_taint in all_taints(),
                supplied in all_taints(),
            ) {
                prop_assume!(input_taint != Taint::Clean);
                let state = make_proptest_run_state(input_taint);
                let contract = test_action_contract(Idempotency::DeterministicPure);
                let frame_taint = state.frame.read_taint(SlotIdx::ZERO).unwrap();
                let result = reject_taint_downgrade(frame_taint, &contract, supplied);
                match result {
                    Err(RuntimeError::ActionTaintDowngrade { supplied: err_supplied, .. }) => {
                        prop_assert_eq!(err_supplied, input_taint,
                            "supplied in error must be frame's input_taint, not the parameter \
                             (input={:?}, param={:?})", input_taint, supplied);
                    }
                    _ => panic!("guard should have fired"),
                }
            }
        }
    }

// =========================================================================
// vb-u09ai: 4-variant RetrySafety shard-lifecycle test (Tier 1).
// =========================================================================

/// Tier 1: `vb_core::action::is_idempotent(RetrySafety::Unknown) == false`
/// per the master §65 contract (C8: Unknown collapses to non-idempotent).
/// The `is_idempotent(RetrySafety)` const fn is a TDD target State 11 will
/// add — on 3-variant code this test fails to compile (preserves the
/// failing-first signal).
#[test]
fn chunk_008_unknown_retry_safety_recognized() {
    use vb_core::action::{is_idempotent, RetrySafety};
    assert!(
        !is_idempotent(RetrySafety::Unknown),
        "Unknown must NOT be considered idempotent (C8 collapses to non-idempotent)"
    );
}
