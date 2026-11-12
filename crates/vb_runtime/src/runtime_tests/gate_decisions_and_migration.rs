    #[test]
    fn runtime_counters_snapshot_starts_at_zero() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // When taking counters snapshot
        let snap = runtime.counters_snapshot();
        // Then all counters are zero
        assert_eq!(snap.runs_submitted, 0);
        assert_eq!(snap.runs_completed, 0);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.steps_executed, 0);
    }

    #[test]
    fn runtime_submit_compiled_delegates_to_submit_direct() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = vb_core::ids::RunId::new(42);
        // When using submit_compiled
        assert_eq!(runtime.submit_compiled(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is processed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
    }

    #[test]
    fn runtime_fail_action_routes_to_run_shard() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let ticket = ActionTicket {
            run: vb_core::ids::RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_idempotency_key(
                vb_core::ids::RunId::new(1),
                1,
                SeqNo::ZERO,
                ActionId::new(0),
            ),
            capacity: 1,
            fencing_token: 0,
        };
        let failure = ActionFailureCode::Rejected.into();
        let result = runtime.fail_action(ticket, failure);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn runtime_answer_ask_routes_to_run_shard() {
        // Given a 1-shard runtime with an Ask-waiting run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        // Drive the run to the Ask-waiting state.
        assert_eq!(runtime.tick_all(), Ok(true));
        let ticket = runtime
            .lookup_pending_ask_ticket(run, StepIdx::new(2))
            .expect("suspended ask must issue a ticket");
        let answer = AskAnswer {
            ticket,
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        // Then answer_ask routes to the only shard and succeeds
        assert_eq!(runtime.answer_ask(answer), Ok(()));
    }

    #[derive(Debug, Default)]
    struct GateDecisionTestJournal {
        decisions: Mutex<Vec<GateOutcomeRecord>>,
        opened: Mutex<Vec<(ManualGate, u64, ApproverId)>>,
    }

    impl GateDecisionTestJournal {
        fn shared() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn decisions(&self) -> crate::RuntimeResult<Vec<GateOutcomeRecord>> {
            self.decisions
                .lock()
                .map(|calls| calls.clone())
                .map_err(|_| RuntimeError::JournalPoisoned)
        }
    }

    impl RuntimeJournal for GateDecisionTestJournal {
        fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            Ok(())
        }

        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            for event in events {
                self.append(event.clone())?;
            }
            Ok(())
        }

        fn append_gate_decision_strict(
            &self,
            record: GateOutcomeRecord,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.decisions
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push(record);
            Ok(())
        }

        fn append_gate_opened_strict(
            &self,
            gate: ManualGate,
            opened_at_ms: u64,
            requester: ApproverId,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.opened
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push((gate, opened_at_ms, requester));
            Ok(())
        }

        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    fn gate_digest(byte: u8) -> GateDigest {
        GateDigest::from_bytes([byte; 32])
    }

    fn gate_role() -> ApproverRole {
        ApproverRole::new("deploy-approver".to_string()).unwrap()
    }

    fn gate_person(value: &str) -> ApproverId {
        ApproverId::new(value.to_string()).unwrap()
    }

    fn gate_fixture(run: RunId, id: u64) -> ManualGate {
        ManualGate {
            id: GateId::new(id),
            run,
            action: ActionId::new(9),
            risk: RiskClass::High,
            compensatable: false,
            required_role: gate_role(),
            separation_of_duties: SeparationOfDuties::DistinctFromRequester,
            evidence_bundle_digest: gate_digest(1),
            prompt_digest: gate_digest(2),
            timeout_ms: 1_000,
            escalation_policy_digest: gate_digest(3),
            break_glass_allowed: true,
            approval_reason_required: true,
        }
    }

    fn gate_input_fixture(id: u64, decision: GateDecision) -> GateDecisionInput {
        GateDecisionInput {
            gate: GateId::new(id),
            decision,
            approver: gate_person("alice"),
            approver_role: gate_role(),
            requester: gate_person("bob"),
            reason: Some("checked the rollout plan".to_string()),
            evidence_shown: true,
            evidence_bundle_digest: gate_digest(1),
            delegated_to: None,
            break_glass_justification: None,
            decided_at_ms: 500,
        }
    }

    fn gate_chain_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let mut nodes = Vec::new();
        for index in 0..9_u16 {
            nodes.push(vb_core::workflow::node(
                StepIdx::new(index),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(index + 1)),
            ));
        }
        nodes.push(vb_core::workflow::node(
            StepIdx::new(9),
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        ));
        let parts = WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            name: Box::from("chain_then_finish"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn running_runtime_with_open_gate(
        run: RunId,
        gate_id: u64,
    ) -> (Runtime, Arc<GateDecisionTestJournal>) {
        let shard_count = NonZeroUsize::new(1).expect("one shard");
        let journal = GateDecisionTestJournal::shared();
        let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
        let workflow = gate_chain_workflow().expect("chain workflow must compile");
        assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let gate = gate_fixture(run, gate_id);
        let shard = runtime
            .shards
            .first_mut()
            .expect("single-shard runtime must hold one shard");
        assert_eq!(shard.open_gate(gate, 0, gate_person("bob")), Ok(()));
        (runtime, journal)
    }

    #[test]
    fn runtime_submit_gate_decision_approves_open_gate() {
        let run = RunId::new(93001);
        let (mut runtime, journal) = running_runtime_with_open_gate(run, 7);
        assert_eq!(
            runtime.submit_gate_decision(run, gate_input_fixture(7, GateDecision::Approve)),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        let shard = runtime
            .shards
            .first()
            .expect("single-shard runtime must hold one shard");
        assert_eq!(
            shard.open_gate_for(run),
            None,
            "gate must close after the approved decision"
        );
        assert_eq!(
            shard.runtime_state_get(run),
            Some(RuntimeState::Running),
            "run must resume after the approved decision"
        );
        let decisions = journal.decisions().expect("decision journal must read");
        assert_eq!(
            decisions.len(),
            1,
            "approval must journal exactly one decision"
        );
        let decision = decisions
            .into_iter()
            .next()
            .expect("one decision must be journaled");
        assert_eq!(decision.outcome, GateOutcome::Approved);
        assert!(decision.receipt.is_some(), "approval must carry a receipt");
    }

    #[test]
    fn runtime_submit_gate_decision_rejects_unknown_gate() {
        let run = RunId::new(93002);
        let shard_count = NonZeroUsize::new(1).expect("one shard");
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let workflow = gate_chain_workflow().expect("chain workflow must compile");
        assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(
            runtime.submit_gate_decision(run, gate_input_fixture(7, GateDecision::Approve)),
            Err(RuntimeError::GateNotOpen {
                gate: GateId::new(7)
            }),
            "a decision for a gate that was never opened must be rejected"
        );
    }

    #[test]
    fn runtime_submit_gate_decision_rejects_run_mismatch() {
        let run = RunId::new(93003);
        let other = RunId::new(93004);
        let (mut runtime, _journal) = running_runtime_with_open_gate(run, 7);
        let workflow = gate_chain_workflow().expect("chain workflow must compile");
        assert_eq!(runtime.submit_compiled(other, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(
            runtime.submit_gate_decision(other, gate_input_fixture(7, GateDecision::Approve)),
            Err(RuntimeError::GateRunMismatch {
                gate: GateId::new(7),
                expected: run,
                actual: other,
            }),
            "a decision naming another run must be rejected with the typed mismatch"
        );
        let shard = runtime
            .shards
            .first()
            .expect("single-shard runtime must hold one shard");
        assert_eq!(
            shard.open_gate_for(run),
            Some(GateId::new(7)),
            "gate must stay open after a rejected mismatched decision"
        );
    }

    #[test]
    fn runtime_submit_gate_decision_rejects_unknown_run() {
        let shard_count = NonZeroUsize::new(1).expect("one shard");
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        assert_eq!(
            runtime.submit_gate_decision(
                RunId::new(93005),
                gate_input_fixture(7, GateDecision::Approve)
            ),
            Err(RuntimeError::RunNotFound),
            "a decision for a run the runtime does not own must be rejected"
        );
    }

    #[test]
    fn runtime_answer_ask_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime (RA-030 regression test)
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        // Pick a run that actually lives on shard 0 so the migration target
        // (shard 1) differs from the home shard. This guarantees the
        // hash-based shard_for() lookup would miss the run.
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            // skip — cannot construct migration scenario on this seed
            return;
        }
        let destination = 1usize;
        // When submitting an Ask-waiting workflow to put the run into a
        // suspend state where answer_ask is meaningful.
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        // Tick until AskAwaiting (handle_ask returns AwaitingAsk).
        assert_eq!(runtime.tick_all(), Ok(true));
        // Sanity: run is now active on home shard.
        assert!(runtime.shards[home_index].run_state_contains(run));
        let ticket_before = runtime.shards[home_index]
            .lookup_pending_ask_ticket(run, StepIdx::new(2))
            .expect("source shard must issue a ticket before migration");
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Now answer_ask must find the run on the destination shard.
        let ticket = runtime
            .lookup_pending_ask_ticket(run, StepIdx::new(2))
            .expect("migrated suspended ask must issue a ticket");
        assert_eq!(ticket, ticket_before);
        let answer = AskAnswer {
            ticket,
            answer_slot: SlotIdx::new(2),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1u32,
        };
        // Then answer_ask returns Ok (post-fix scans all shards).
        assert_eq!(runtime.answer_ask(answer), Ok(()));
    }

    #[test]
    fn runtime_answer_ask_returns_run_not_found_for_unknown_run() {
        // Given a 2-shard runtime with no submitted runs
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // Unknown runs cannot be used to issue an opaque ticket.
        assert_eq!(
            runtime.lookup_pending_ask_ticket(vb_core::ids::RunId::new(424242), StepIdx::ZERO),
            None
        );
    }

    #[test]
    fn runtime_cancel_run_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0 (vb-906ix).
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // Find a run whose home shard is 0 so migration to shard 1 is real.
        let Some(run) = (1..=64)
            .map(vb_core::ids::RunId::new)
            .find(|candidate| runtime.shard_index(*candidate) == 0)
        else {
            panic!("no run id in 1..=64 homes to shard 0");
        };
        let home_index = 0usize;
        let destination = 1usize;
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(runtime.shards[home_index].run_state_contains(run));
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        assert!(runtime.shards[destination].run_state_contains(run));
        // When cancelling, the command must reach the owning shard.
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the cancel took effect instead of stranding on the home shard:
        // the migrated run must leave the owning shard's active set.
        assert!(!runtime.shards[destination].run_state_contains(run));
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_snapshot_run_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0 (vb-906ix).
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        // Find a run whose home shard is 0 so migration to shard 1 is real.
        let Some(run) = (1..=64)
            .map(vb_core::ids::RunId::new)
            .find(|candidate| runtime.shard_index(*candidate) == 0)
        else {
            panic!("no run id in 1..=64 homes to shard 0");
        };
        let home_index = 0usize;
        let destination = 1usize;
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        assert!(runtime.shards[destination].run_state_contains(run));
        assert!(!runtime.shards[home_index].run_state_contains(run));
        assert_eq!(
            runtime.shards[home_index].snapshot_run(run, 42),
            InspectResponse::NotFound {
                run,
                correlation: 42
            }
        );
        // When snapshotting, the response must come from the owning shard.
        match runtime.snapshot_run(run, 42) {
            Ok(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.run, run);
            }
            other => panic!("expected Found snapshot for migrated run, got {other:?}"),
        }
    }

    // ----------------------------------------------------------------
    // RA-030 wave-15 follow-up (vb-sxkz6) regression tests
    // Each test mirrors runtime_answer_ask_finds_run_on_migrated_shard
    // and exercises the scan-all-shards routing helper.
    // ----------------------------------------------------------------

    /// RA-030 wave-15: list_events must find the run on its owning shard
    /// even after migration.
    #[test]
    fn runtime_list_events_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0.
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            return;
        }
        let destination = 1usize;
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(runtime.shards[home_index].run_state_contains(run));
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Then list_events returns Ok on the destination shard.
        let result = runtime.list_events(run);
        assert!(matches!(result, Ok(_)));
    }

    /// RA-030 wave-15: take_inspect_response must drain the inspect slot
    /// of the owning shard even after migration.
    #[test]
    fn runtime_take_inspect_response_finds_run_on_migrated_shard() {
        // Given a 2-shard runtime; run lives on shard 0.
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
        let run = vb_core::ids::RunId::new(1);
        let home_index = runtime.shard_index(run);
        if home_index != 0 {
            return;
        }
        let destination = 1usize;
        let Some(wf) = ask_waiting_workflow() else {
            return;
        };
        assert_eq!(submit_ask_waiting(&runtime, run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(runtime.shards[home_index].run_state_contains(run));
        // Simulate migration through the invariant-preserving transfer helper.
        assert_eq!(
            migrate_active_run_for_test(&mut runtime, home_index, destination, run),
            Ok(())
        );
        // Then take_inspect_response returns Ok on the destination shard.
        // No inspect was issued, so the slot is None.
        let result = runtime.take_inspect_response(run);
        assert!(matches!(result, Ok(None)));
    }
