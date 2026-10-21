    use crate::journal::VolatileRuntimeJournal;
    use crate::recovery::RecoveredRunBoundary;
    use crate::shard::types::RecoverRunCommand;

    fn action_recovery_test_contract() -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
        let name = vb_core::action::ActionName::new("recovery-action")?;
        Ok(vb_core::action::ActionContract {
            id: vb_core::ids::ActionId::new(0),
            name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            side_effect: vb_core::action::SideEffect::None,
            retry_safety: vb_core::action::RetrySafety::Safe,
            required_capabilities: Box::new([]),
        })
    }

    fn action_recovery_contract_store(
        digest: vb_core::ids::WorkflowDigest,
        workflow_ir: Vec<u8>,
        contract: vb_core::action::ActionContract,
    ) -> crate::admission::SharedAcceptedArtifactStore {
        #[derive(Debug)]
        struct ActionRecoveryContractStore {
            digest: vb_core::ids::WorkflowDigest,
            workflow_ir: Vec<u8>,
            contract: vb_core::action::ActionContract,
        }

        impl crate::admission::AcceptedArtifactStore for ActionRecoveryContractStore {
            fn load_accepted_artifact(
                &self,
                artifact_digest: vb_core::ids::WorkflowDigest,
            ) -> Result<
                vb_storage::admission::AcceptedArtifact,
                crate::admission::ArtifactEnvelopeError,
            > {
                if artifact_digest != self.digest {
                    return Err(crate::admission::ArtifactEnvelopeError::ArtifactNotFound {
                        digest: artifact_digest,
                    });
                }
                Ok(vb_storage::admission::AcceptedArtifact {
                    digest: self.digest,
                    source_digest: self.digest,
                    policy_digest: self.digest,
                    ir: self.workflow_ir.clone(),
                    verification: vb_storage::admission::VerificationProof {
                        digest: self.digest,
                        gate_count: 8,
                        durable: true,
                        bounded_claimed: true,
                        taint_safe_claimed: true,
                        retry_safe_claimed: true,
                        idempotency_verified_claimed: true,
                        replayable_claimed: true,
                        idempotency_keyed: Box::new([]),
                        idempotency_attested: Box::new([]),
                        warnings: Vec::new(),
                    },
                    accepted_at_seq: vb_storage::types::EventSeq::new(0),
                    required_capabilities: Box::new([]),
                    action_contracts: Box::from([self.contract.clone()]),
                })
            }
        }

        std::sync::Arc::new(ActionRecoveryContractStore {
            digest,
            workflow_ir,
            contract,
        })
    }

    #[test]
    fn recover_restores_action_contracts_schedules_completes_and_finishes() -> Result<(), Box<dyn std::error::Error>> {
        let run_id = vb_core::ids::RunId::new(1);
        let workflow_digest = vb_core::ids::WorkflowDigest::from_bytes([42u8; 32]);

        let do_node = vb_core::workflow::CompiledNode {
            id: vb_core::ids::StepIdx::ZERO,
            output: Some(vb_core::ids::SlotIdx::new(1)),
            next: Some(vb_core::ids::StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(0),
                input: vb_core::ids::SlotIdx::new(0),
            },
        };
        let finish_node = vb_core::workflow::CompiledNode {
            id: vb_core::ids::StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("recovery-test"),
            digest: workflow_digest,
            nodes: Box::from([do_node, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            symbols_count: 0,
            entry: vb_core::ids::StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };

        let workflow_ir = postcard::to_allocvec(&parts)?;
        let contract = action_recovery_test_contract()?;

        let config = crate::shard::ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let store = action_recovery_contract_store(workflow_digest, workflow_ir, contract.clone());
        let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
        let shared_journal = journal.clone();
        let mut shard = crate::shard::Shard::new_with_journal_and_artifact_store(
            config,
            shared_journal,
            store.clone(),
        );

        let mut frame = vb_core::frame::RunFrame::new(run_id, vb_core::ids::StepIdx::ZERO, 2, 2)?;
        frame.write_slot(vb_core::ids::SlotIdx::new(0), vb_core::value::SlotValue::I64(0))?;

        let command = RecoverRunCommand::new(
            run_id,
            frame,
            workflow_digest,
            workflow_digest,
            vb_storage::types::EventSeq::new(1),
            crate::primitives::collect::CollectStates::new(),
            RecoveredRunBoundary::none(),
        );

        // Phase 1: Drive recovery — restores action contracts and schedules Do
        shard.handle_recover(command)?;

        // Verify action contracts were restored from the artifact (exact match)
        let state = shard.run_state_get(run_id).ok_or(
            "run state not found after recovery; recovery failed",
        )?;
        let expected_artifact = store.load_accepted_artifact(workflow_digest)?;
        assert_eq!(
            state.action_contracts,
            expected_artifact.action_contracts,
            "recovered run action_contracts must exactly match artifact"
        );

        // Verify the Do action was scheduled with the correct ticket fields
        let ticket = shard.pending_action_get(run_id).ok_or(
            "pending action not found after recovery; Do action was not scheduled",
        )?;
        assert_eq!(ticket.run, run_id);
        assert_eq!(ticket.step, vb_core::ids::StepIdx::ZERO);
        assert_eq!(ticket.action, vb_core::ids::ActionId::new(0));
        assert_eq!(ticket.attempt, 1);

        // Phase 2: Complete the action using the REAL API (ShardCommand::ActionCompleted)
        let output = vb_core::action::ActionOutputReady {
            output_slot: vb_core::ids::SlotIdx::new(1),
            value: vb_core::value::SlotValue::I64(42),
            taint: vb_core::value::Taint::Clean,
            encoded_len: 2,
        };
        shard.enqueue(crate::shard::ShardCommand::ActionCompleted {
            ticket,
            output,
        })?;

        // Drive to completion: ActionCompleted → Finish → RunFinished
        let ticked = shard.tick()?;
        assert!(ticked, "tick should return true after processing the action completion");

        // Verify run finished
        assert!(
            shard.terminal_runs_contains(run_id),
            "run should be in terminal state after action completion and finish"
        );
        assert!(
            !shard.run_state_contains(run_id),
            "run state should be removed after terminal completion"
        );

        // Phase 3: Verify real journal evidence
        let events = journal.snapshot()?;

        // When action contracts are present, the runtime uses the new
        // ActionCompletedEnvelope format (atomic durable envelope).
        let action_completed_count = events
            .iter()
            .filter(|e| matches!(
                e,
                crate::journal::RuntimeJournalEvent::ActionCompleted { .. }
                    | crate::journal::RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            ))
            .count();
        assert_eq!(
            action_completed_count,
            1,
            "exactly one action completion journal event expected (Completed or Envelope)"
        );

        // Exactly one RunFinished event
        let run_finished_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
            .collect();
        assert_eq!(
            run_finished_events.len(),
            1,
            "exactly one RunFinished journal event expected"
        );
        // Verify the RunFinished event has the correct run id and result slot
        let run_finished = &run_finished_events[0];
        match run_finished {
            crate::journal::RuntimeJournalEvent::RunFinished { run, result } => {
                assert_eq!(*run, run_id, "RunFinished run id must match");
                assert_eq!(
                    *result,
                    vb_core::ids::SlotIdx::new(1),
                    "RunFinished result slot must match workflow finish node"
                );
            }
            _ => panic!("run_finished_events filter is incorrect"),
        }

        // Phase 4: Duplicate completion must NOT produce a second RunFinished
        // After the run is terminal, a duplicate ActionCompleted is rejected
        // with an error (InvalidActionCompletion or RunNotFound), not silently
        // accepted. Verify the error is thrown and journal is unchanged.
        shard.enqueue(crate::shard::ShardCommand::ActionCompleted {
            ticket,
            output: vb_core::action::ActionOutputReady {
                output_slot: vb_core::ids::SlotIdx::new(1),
                value: vb_core::value::SlotValue::I64(99),
                taint: vb_core::value::Taint::Clean,
                encoded_len: 2,
            },
        })?;
        // tick() should return Err because the duplicate completion targets
        // a run that has already finished (terminal)
        let tick_result = shard.tick();
        assert!(
            tick_result.is_err(),
            "duplicate action completion for terminal run should produce an error"
        );

        let events2 = journal.snapshot()?;
        let run_finished_count_after_dup = events2
            .iter()
            .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
            .count();
        assert_eq!(
            run_finished_count_after_dup,
            1,
            "duplicate completion must not add second RunFinished"
        );

        Ok(())
    }
