
    use vb_core::action::{
        ActionContract, Idempotency, RetrySafety, SideEffect,
    };

    use crate::shard::lifecycle::MAX_ACTION_OUTPUT_BYTES;

    /// Builds a single Do-node workflow that suspends until the action at
    /// `ActionId(0)` completes.  The input slot and output slot are both
    /// `SlotIdx::ZERO`, matching the default test helpers.
    fn single_do_workflow() -> Result<CompiledWorkflow, String> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("action_output_policy"),
            digest: WorkflowDigest::from_bytes([0xA0; 32]),
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
        CompiledWorkflow::try_from_parts(parts).map_err(|error| {
            format!("action output policy fixture must compile: {error:?}")
        })
    }

    /// Builds a workflow that drives a Do action and then a `Finish` node.
    /// After the action completes successfully, the runtime advances the
    /// program counter to the Finish node, allowing a happy-path completion
    /// test to observe a terminated run.
    fn do_then_finish_workflow() -> Result<CompiledWorkflow, String> {
        let do_node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("do_then_finish"),
            digest: WorkflowDigest::from_bytes([0xA1; 32]),
            nodes: Box::from([do_node, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts)
            .map_err(|error| format!("do_then_finish fixture must compile: {error:?}"))
    }

    /// Builds a single action contract with caller-controlled bounds.  Only
    /// the fields that influence the preflight checks are exposed; everything
    /// else is set to the test-friendly defaults.
    fn test_action_contract(max_output_bytes: u32) -> ActionContract {
        ActionContract {
            id: ActionId::new(0),
            name: vb_core::action::ActionName::new("policy-test")
                .expect("policy-test name is a valid ActionName"),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::from([]),
        }
    }

    /// Encodes `value` via postcard and returns the encoded length.  Used to
    /// derive the `encoded_len` field that the preflight checks against the
    /// declared and the contract bounds.
    fn encoded_byte_len(value: &SlotValue) -> u32 {
        let bytes = postcard::to_allocvec(value).expect("postcard encoding is total");
        u32::try_from(bytes.len()).expect("encoded length fits in u32 for the test corpus")
    }

    /// `test_action_output_oversize_rejected` — when a caller supplies an
    /// output whose encoded byte length exceeds the contract's
    /// `max_output_bytes`, the preflight must reject with
    /// `RuntimeError::ActionOutputTooLarge` and must not mutate the run
    /// counters or journal.  This guards the master §19 (action ABI) and
    /// §44 point 14 invariants: an oversized output is never allowed to
    /// advance the in-memory frame or persist as durable evidence.
    #[test]
    fn test_action_output_oversize_rejected() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = single_do_workflow()?;
        let run = RunId::new(60_001);
        // `I64(7)` encodes to 2 bytes via postcard (1-byte variant tag +
        // 1-byte varint payload), so a contract ceiling of 1 forces the
        // preflight to reject the completion.  Using a small value avoids
        // any reliance on the absolute 64 KiB cap while still proving the
        // per-contract size gate is enforced.
        let contract = test_action_contract(1);
        let contracts: Box<[ActionContract]> = Box::from([contract]);
        // The action engine reads the input slot's taint on dispatch, so the
        // input must be initialized even though we are testing the output
        // size cap.
        let inputs: Box<[(SlotIdx, SlotValue)]> =
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]);
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow: wf,
                inputs,
                caps: CapabilitySet::empty(),
                action_contracts: contracts,
            }),
            Ok(())
        );
        // The Do step suspends; tick should report progress without error.
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.active_run_count(),
            1,
            "Do step must suspend with one active run"
        );
        let counters_before = shard.counters().snapshot();
        // `I64(7)` encodes to 2 bytes via postcard, which is strictly larger
        // than the contract ceiling of 1.  The preflight must reject it
        // with `ActionOutputTooLarge`.
        let oversized = SlotValue::I64(7);
        let declared_len = encoded_byte_len(&oversized);
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: oversized,
            taint: Taint::Clean,
            encoded_len: declared_len,
        };
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: vb_core::action::compute_action_idempotency_key(
                run,
                vb_core::ids::SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        let result = shard.tick();
        let err = result.expect_err("oversize completion must be rejected");
        let expected = RuntimeError::ActionOutputTooLarge { size: 2, max: 1 };
        assert_eq!(err, expected, "oversize must surface ActionOutputTooLarge");
        // Counters and state must remain untouched.
        assert_eq!(shard.counters().snapshot(), counters_before);
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    /// `test_action_output_taint_mismatch_rejected` — when a caller
    /// completes a deterministic-pure action whose input slot carries
    /// `Taint::Secret`, supplying `Taint::Clean` as the output taint must
    /// be rejected with `RuntimeError::ActionTaintDowngrade`.  This proves
    /// the runtime enforces master §19 (taint propagation) and §44 point
    /// 11 (idempotency class propagation): an action cannot "declassify"
    /// its own output, regardless of what the caller declares.
    #[test]
    fn test_action_output_taint_mismatch_rejected() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = single_do_workflow()?;
        let run = RunId::new(60_002);
        let contract = test_action_contract(1024);
        let contracts: Box<[ActionContract]> = Box::from([contract]);
        // Seed the input slot with a value so the slot is initialized;
        // `write_taint` requires a populated slot.
        let inputs: Box<[(SlotIdx, SlotValue)]> =
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]);
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow: wf,
                inputs,
                caps: CapabilitySet::empty(),
                action_contracts: contracts,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Mark the input slot as Secret after submission so the propagated
        // required taint is `Taint::Secret`.  This is the hostile state a
        // malicious or buggy completion handler might try to downgrade.
        {
            let Some(state) = shard.run_state_get_mut(run) else {
                return Err("run must remain active after submit".to_string());
            };
            state
                .frame
                .write_taint(SlotIdx::ZERO, Taint::Secret)
                .map_err(|e| format!("write_taint failed: {e:?}"))?;
        }
        let counters_before = shard.counters().snapshot();
        let value = SlotValue::I64(7);
        let declared_len = encoded_byte_len(&value);
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value,
            // Downgrade attempt: Secret input but Clean output taint.
            taint: Taint::Clean,
            encoded_len: declared_len,
        };
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: vb_core::action::compute_action_idempotency_key(
                run,
                vb_core::ids::SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        let result = shard.tick();
        let err = result.expect_err("taint downgrade must be rejected");
        let expected = RuntimeError::ActionTaintDowngrade {
            required: Taint::Secret,
            supplied: Taint::Clean,
        };
        assert_eq!(
            err, expected,
            "taint downgrade must surface ActionTaintDowngrade"
        );
        // Counters and state must remain untouched.
        assert_eq!(shard.counters().snapshot(), counters_before);
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    /// `test_action_output_within_limits_accepted` — when a caller
    /// supplies an output that is within the absolute cap, the contract
    /// cap, the resource cap, and propagates the required taint, the
    /// preflight must accept the completion and the run must advance to
    /// its terminal state.  This is the positive companion to the
    /// oversize and taint-downgrade rejection tests.
    #[test]
    fn test_action_output_within_limits_accepted() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = do_then_finish_workflow()?;
        let run = RunId::new(60_003);
        let contract = test_action_contract(1024);
        let contracts: Box<[ActionContract]> = Box::from([contract]);
        // The action engine reads the input slot's taint on dispatch, so
        // initialize the input slot up front.
        let inputs: Box<[(SlotIdx, SlotValue)]> =
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]);
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow: wf,
                inputs,
                caps: CapabilitySet::empty(),
                action_contracts: contracts,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.active_run_count(),
            1,
            "Do step must suspend with one active run"
        );
        // `I64(7)` is well within every cap: 2 bytes (postcard variant
        // tag + varint payload) < 1024 (contract), < 64 KiB (absolute),
        // and < 16 MiB (resource).
        let value = SlotValue::I64(7);
        let declared_len = encoded_byte_len(&value);
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value,
            taint: Taint::Clean,
            encoded_len: declared_len,
        };
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: vb_core::action::compute_action_idempotency_key(
                run,
                vb_core::ids::SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        // The action completion plus the Finish step both run in the
        // same tick; the run must terminate cleanly.
        assert_eq!(shard.tick(), Ok(true));
        let counters = shard.counters().snapshot();
        assert_eq!(counters.runs_submitted, 1);
        assert_eq!(counters.runs_completed, 1);
        assert_eq!(shard.active_run_count(), 0);
        // The absolute cap constant is part of the public module API
        // and must be exactly 64 KiB.
        assert_eq!(MAX_ACTION_OUTPUT_BYTES, 64 * 1024);
        Ok(())
    }
