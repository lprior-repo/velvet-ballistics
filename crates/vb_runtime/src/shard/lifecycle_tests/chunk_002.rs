
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

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    fn make_ticket(run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
        let seq = SeqNo::ZERO;
        let action = ActionId::new(0);
        ActionTicket {
            run,
            step,
            seq,
            action,
            attempt,
            idempotency_key: vb_core::action::compute_action_idempotency_key(run, seq, action),
            capacity: 1,
        }
    }

    fn pending_ticket(shard: &Shard, run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
        let Some(ticket) = shard.pending_action_get(run) else {
            panic!("pending action ticket must exist for run {run:?}");
        };
        assert_eq!(ticket.step, step);
        assert_eq!(ticket.attempt, attempt);
        ticket
    }

    fn non_retryable_failure() -> ActionFailure {
        ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: VbRetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        }
    }

    fn retryable_failure() -> ActionFailure {
        ActionFailure {
            retry_policy: VbRetryPolicy::Retryable,
            ..non_retryable_failure()
        }
    }

    fn require_workflow(
        name: &str,
        workflow: Option<vb_core::workflow::CompiledWorkflow>,
    ) -> Result<vb_core::workflow::CompiledWorkflow, String> {
        match workflow {
            Some(wf) => Ok(wf),
            None => Err(format!("{name} fixture workflow must compile")),
        }
    }

    fn require_snapshot(
        journal: &crate::journal::VolatileRuntimeJournal,
    ) -> Result<Vec<RuntimeJournalEvent>, String> {
        journal
            .snapshot()
            .map_err(|error| format!("journal snapshot failed: {error:?}"))
    }

    fn action_failed_count(events: &[RuntimeJournalEvent], run: RunId, step: StepIdx) -> usize {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::ActionFailed { run: event_run, step: event_step, .. }
                        if *event_run == run && *event_step == step
                )
            })
            .count()
    }

    fn retry_workflow() -> Result<vb_core::workflow::CompiledWorkflow, String> {
        let set_policy = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let retry = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(3),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::from("retry"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: Box::from([set_policy, action, retry, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(2)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        });
        workflow.map_err(|error| format!("retry fixture workflow must compile: {error:?}"))
    }

    fn submit_run(shard: &mut Shard, run: RunId, workflow: CompiledWorkflow) {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    fn enqueue_action_failure(shard: &mut Shard, run: RunId, step: StepIdx, attempt: u16) {
        let ticket = pending_ticket(shard, run, step, attempt);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    fn event_position(
        events: &[RuntimeJournalEvent],
        expected: &RuntimeJournalEvent,
    ) -> Option<usize> {
        events.iter().position(|event| event == expected)
    }

    fn assert_event_order(
        events: &[RuntimeJournalEvent],
        first: RuntimeJournalEvent,
        second: RuntimeJournalEvent,
    ) {
        let first_position = event_position(events, &first);
        let second_position = event_position(events, &second);
        assert!(
            matches!((first_position, second_position), (Some(a), Some(b)) if a < b),
            "events out of order: {events:?}"
        );
    }

    fn assert_retry_exhaustion_journal(events: &[RuntimeJournalEvent], run: RunId) {
        assert_eq!(action_failed_count(events, run, StepIdx::new(1)), 2);
        assert_event_order(
            events,
            RuntimeJournalEvent::ActionFailed {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(0),
                attempt: 1,
            },
            RuntimeJournalEvent::RunFailed { run },
        );
    }

    #[test]
    fn submit_finished_workflow_completes_immediately() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(1);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.active_run_count(), 0);
    }
