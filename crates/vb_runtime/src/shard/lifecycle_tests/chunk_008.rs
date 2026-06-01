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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
        }
    }

    // ── DeterministicPure taint rejection ─────────────────────────────────

    #[test]
    fn deterministicpure_with_clean_input_passes_taint_check() {
        let state = test_run_state(Taint::Clean);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let result = reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::Clean);
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
        let result = reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::Secret);
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
        let result = reject_taint_downgrade(
            &state,
            SlotIdx::ZERO,
            &contract,
            Taint::DerivedFromSecret,
        );
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
        let state = test_run_state(Taint::Random);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let result = reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::Random);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(supplied, Taint::Random, "supplied must be Random");
            }
            other => panic!(
                "expected ActionTaintDowngrade(Clean, Random), got {other:?}"
            ),
        }
    }

    #[test]
    fn deterministicpure_with_timedependent_input_returns_taintviolation() {
        let state = test_run_state(Taint::TimeDependent);
        let contract = test_action_contract(Idempotency::DeterministicPure);
        let result =
            reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::TimeDependent);
        match result {
            Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => {
                assert_eq!(required, Taint::Clean, "required must be Clean");
                assert_eq!(
                    supplied,
                    Taint::TimeDependent,
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
        };
        let result = reject_taint_downgrade(
            &state,
            SlotIdx::ZERO,
            &contract,
            Taint::DerivedFromSecret,
        );
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
        };
        let result = reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::Clean);
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
        let result =
            reject_taint_downgrade(&state, SlotIdx::ZERO, &contract, Taint::DerivedFromSecret);
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
