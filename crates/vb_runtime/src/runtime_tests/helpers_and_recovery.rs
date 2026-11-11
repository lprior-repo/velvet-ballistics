    #[derive(Debug)]
    struct RejectCompletionJournal {
        events: Mutex<Vec<RuntimeJournalEvent>>,
    }

    impl RejectCompletionJournal {
        fn shared() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
            })
        }

        fn snapshot(&self) -> crate::RuntimeResult<Vec<RuntimeJournalEvent>> {
            self.events
                .lock()
                .map(|events| events.clone())
                .map_err(|_| RuntimeError::JournalPoisoned)
        }
    }

    impl RuntimeJournal for RejectCompletionJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            if matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }) {
                return Err(RuntimeError::JournalFull { capacity: 0 });
            }
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push(event);
            Ok(())
        }

        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }))
            {
                return Err(RuntimeError::JournalFull { capacity: 0 });
            }
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .extend(events.iter().cloned());
            Ok(())
        }

        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
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
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn action_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let do_node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
        );
        let finish = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("action_then_finish"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([do_node, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn runtime_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    fn contract_required_capability(action: ActionId) -> Capability {
        Capability::new("__contract_required__".into(), action)
    }

    fn action_contract(action: ActionId, input_slots: u16, output_slots: u16) -> ActionContract {
        ActionContract {
            id: action,
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: input_slots,
            output_slot_count: output_slots,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::from([contract_required_capability(action)]),
        }
    }

    fn action_contracts_through(
        action: ActionId,
        input_slots: u16,
        output_slots: u16,
    ) -> Box<[ActionContract]> {
        let target = action.get();
        let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
        let mut id = 0u16;
        loop {
            let current = ActionId::new(id);
            if id == target {
                contracts.push(action_contract(current, input_slots, output_slots));
                break;
            }
            contracts.push(action_contract(current, 0, 0));
            id = id.saturating_add(1);
        }
        contracts.into_boxed_slice()
    }

    fn action_grants(action: ActionId) -> CapabilitySet {
        let mut grants = Vec::new();
        for i in 0..=action.get() {
            grants.push(contract_required_capability(ActionId::new(i)));
        }
        CapabilitySet::from_grants(grants.into_boxed_slice())
    }

    fn encoded_len(value: &SlotValue) -> u32 {
        match postcard::to_allocvec(value) {
            Ok(bytes) => match u32::try_from(bytes.len()) {
                Ok(len) => len,
                Err(_) => u32::MAX,
            },
            Err(_) => u32::MAX,
        }
    }

    fn active_frame(runtime: &Runtime, run: vb_core::ids::RunId) -> Option<vb_core::RunFrame> {
        runtime
            .shards
            .get(runtime.shard_index(run))
            .and_then(|shard| shard.run_state_get(run))
            .map(|state| state.frame.clone())
    }

    fn shard_error_index(index: usize) -> u32 {
        u32::try_from(index).map_or(u32::MAX, |value| value)
    }

    fn migrate_active_run_for_test(
        runtime: &mut Runtime,
        source: usize,
        destination: usize,
        run: vb_core::ids::RunId,
    ) -> crate::RuntimeResult<()> {
        if source == destination {
            return Err(RuntimeError::MigrateSelf);
        }
        let shard_count = runtime.shards.len();
        if source >= shard_count || destination >= shard_count {
            return Err(RuntimeError::ShardNotFound {
                shard: shard_error_index(destination),
            });
        }
        if source < destination {
            let (left, right) = runtime.shards.split_at_mut(destination);
            let source_shard = left.get_mut(source).ok_or(RuntimeError::ShardNotFound {
                shard: shard_error_index(source),
            })?;
            let target_shard = right.get_mut(0).ok_or(RuntimeError::ShardNotFound {
                shard: shard_error_index(destination),
            })?;
            return source_shard.transfer_active_run_to(target_shard, run);
        }
        let (left, right) = runtime.shards.split_at_mut(source);
        let target_shard = left
            .get_mut(destination)
            .ok_or(RuntimeError::ShardNotFound {
                shard: shard_error_index(destination),
            })?;
        let source_shard = right.get_mut(0).ok_or(RuntimeError::ShardNotFound {
            shard: shard_error_index(source),
        })?;
        source_shard.transfer_active_run_to(target_shard, run)
    }

    fn submit_suspended(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
        )
    }

    fn ask_waiting_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        );
        let set_timeout = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(2)),
        );
        let ask = vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)).into(),
            },
            None,
            Some(StepIdx::new(3)),
        );
        let resume = vb_core::workflow::node(
            StepIdx::new(3),
            CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
            None,
            Some(StepIdx::new(4)),
        );
        let finish = vb_core::workflow::node(
            StepIdx::new(4),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("ask_waiting_test"),
            digest: WorkflowDigest::from_bytes([42; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0), ConstValue::I64(0)]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn ask_without_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        );
        let ask = vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None.into(),
            },
            None,
            Some(StepIdx::new(2)),
        );
        let resume = vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::AskResume {
                answer: SlotIdx::new(1),
            },
            None,
            Some(StepIdx::new(3)),
        );
        let finish = vb_core::workflow::node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("ask_without_timeout_test"),
            digest: WorkflowDigest::from_bytes([43; 32]),
            nodes: Box::from([set_prompt, ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn asking_recovery_summary(
        run: RunId,
        workflow: &vb_core::workflow::CompiledWorkflow,
    ) -> vb_storage::recovery::RecoveryRuntimeSummary {
        vb_storage::recovery::RecoveryRuntimeSummary {
            run,
            first_seq: vb_storage::EventSeq::new(0),
            last_seq: vb_storage::EventSeq::new(2),
            workflow: Some(workflow.digest()),
            steps_started: 1,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 1,
            slots_written: 0,
            terminal: None,
        }
    }

    fn asking_recovery_seed(
        run: RunId,
        workflow: &vb_core::workflow::CompiledWorkflow,
    ) -> vb_storage::recovery::RecoveryFrameSeed {
        vb_storage::recovery::RecoveryFrameSeed {
            summary: asking_recovery_summary(run, workflow),
            first_step: StepIdx::ZERO,
            step_count: 4,
            slot_count: 2,
            pc: StepIdx::new(1),
            steps: asking_recovery_steps(),
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: vb_storage::recovery::UnsupportedRecoveryState::SUPPORTED,
        }
    }

    fn asking_recovery_steps() -> Vec<vb_storage::recovery::RecoveredStepEntry> {
        vec![
            vb_storage::recovery::RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: vb_storage::recovery::RecoveredStepState::Succeeded,
            },
            vb_storage::recovery::RecoveredStepEntry {
                step: StepIdx::new(1),
                state: vb_storage::recovery::RecoveredStepState::Asking,
            },
        ]
    }

    fn asking_recovery_prefix(
        run: RunId,
        workflow: &vb_core::workflow::CompiledWorkflow,
    ) -> RecoveryPrefixEvidence {
        RecoveryPrefixEvidence {
            events: Vec::new(),
            admission: crate::admission::RunAdmission::new(
                WorkflowDigest::from_bytes([0x41; 32]),
                run,
                CapabilitySet::empty(),
                RuntimePolicy::Relaxed,
            ),
            workflow_digest: workflow.digest(),
            next_seq: vb_storage::EventSeq::new(3),
        }
    }

    fn runtime_product_without_ask_schedule(
        run: RunId,
    ) -> Result<crate::recovery::RuntimeRecoveryProduct, String> {
        let workflow = ask_without_timeout_workflow().ok_or("ask workflow must compile")?;
        let seed = vb_storage::recovery::RecoveryFrameSeedProduct::from_seed(asking_recovery_seed(
            run, &workflow,
        ));
        let boundary =
            crate::recovery::recovered_run_boundary_from_seed(seed.seed(), None, &workflow, &[]);
        Runtime::product_from_recovered_seed(
            run,
            asking_recovery_prefix(run, &workflow),
            seed,
            boundary,
            crate::recovery::RecoveredRetainedInputs::empty(),
        )
        .map_err(|error| error.to_string())
    }

    #[test]
    fn runtime_recovery_without_ask_schedule_is_not_resumable() -> Result<(), String> {
        let run = RunId::new(26_001);
        let product = runtime_product_without_ask_schedule(run)?;
        assert_eq!(product.cannot_resume_reason(), Some("pending_asks"));
        Ok(())
    }

    fn submit_ask_waiting(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        runtime.submit_compiled(run, wf)
    }

    fn submit_action_then_finish(
        runtime: &Runtime,
        run: vb_core::ids::RunId,
        wf: vb_core::workflow::CompiledWorkflow,
    ) -> crate::RuntimeResult<()> {
        let action = ActionId::new(7);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run,
            wf,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            action_grants(action),
            action_contracts_through(action, 1, 2),
        )
    }

    #[test]
    fn snapshot_run_reports_missing_run_without_command_queue() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime =
            Runtime::new_for_tests_and_benchmarks_only(shard_count, ShardConfig::default());
        let result = runtime.snapshot_run(vb_core::ids::RunId::new(1), 7);
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: vb_core::ids::RunId::new(1),
                correlation: 7,
            })
        );
    }

    #[test]
    fn list_events_is_non_destructive() {
        // RA-030 wave-15 (vb-sxkz6): list_events now scans all shards and
        // returns Err(RunNotFound) when the run is not on any shard.
        // The "non-destructive" property is preserved for runs that ARE on
        // a shard; for unknown runs we expect RunNotFound.
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, config);
        let first = runtime.list_events(vb_core::ids::RunId::new(1));
        let second = runtime.list_events(vb_core::ids::RunId::new(1));
        assert_eq!(first, Err(RuntimeError::RunNotFound));
        assert_eq!(second, Err(RuntimeError::RunNotFound));
    }
