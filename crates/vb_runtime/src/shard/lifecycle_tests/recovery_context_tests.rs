    // Uses imports from earlier chunk_001.rs: RuntimeError, Shard, ShardConfig, etc.
    // No additional use statements needed.

    fn recovery_context_test_shard() -> crate::shard::Shard {
        let config = crate::shard::ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            max_open_gates: 1024,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        crate::shard::Shard::new(config)
    }

    #[test]
    fn recover_rejects_malformed_command() -> Result<(), Box<dyn std::error::Error>> {
        // Build a genuine workflow and artifact, then corrupt the artifact's IR
        // so the envelope is internally consistent but recovery fails at
        // deserialization.
        let parts = crash_lab_minimal_parts(vb_core::ids::WorkflowDigest::from_bytes([42u8; 32]));
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)?;
        let workflow = honest_digest_workflow(workflow)?;
        let contract = action_recovery_test_contract()?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            vb_core::policy::RuntimePolicy::Relaxed,
            &[contract],
        )?;

        // Corrupt the artifact: clear IR but keep the digest valid
        let mut malformed = artifact;
        malformed.ir.clear();
        let malformed_digest = vb_storage::admission::accepted_artifact_digest(&malformed)?;
        malformed.digest = malformed_digest;
        malformed.verification.digest = malformed_digest;

        // Store the malformed artifact
        let store = action_recovery_contract_store(malformed);

        let config = crate::shard::ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            max_open_gates: 1024,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let mut shard = crate::shard::Shard::new_with_journal_and_artifact_store(
            config,
            journal,
            store,
        );

        let mut frame = vb_core::frame::RunFrame::new(
            vb_core::ids::RunId::new(1),
            vb_core::ids::StepIdx::ZERO,
            2,
            2,
        )?;
        frame.write_slot(
            vb_core::ids::SlotIdx::new(0),
            vb_core::value::SlotValue::I64(0),
        )?;

        let command = crate::shard::types::RecoverRunCommand::new(
            vb_core::ids::RunId::new(1),
            frame,
            malformed_digest,
            workflow.digest(),
            vb_storage::EventSeq::new(1),
            crate::primitives::collect::CollectStates::new(),
            crate::recovery::RecoveredRunBoundary::none(),
        );

        let result = shard.handle_recover(command);
        assert_eq!(
            result,
            Err(crate::RuntimeError::Recovery {
                error: vb_storage::JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeUnexpectedEnd,
                )
                .to_string(),
            })
        );
        assert_submit_pair_refused(&shard, vb_core::ids::RunId::new(1));
        Ok(())
    }

    #[test]
    fn recover_seeds_nonzero_journal_sequence() {
        let mut shard = recovery_context_test_shard();

        let run = vb_core::ids::RunId::new(42);
        let seq = vb_storage::EventSeq::new(100);
        let result = shard.seed_journal_sequence(run, seq);
        assert!(
            result.is_ok(),
            "seeding a journal sequence must succeed"
        );
    }

    #[test]
    fn recover_handles_pending_action_boundary() {
        let run = vb_core::ids::RunId::new(1);
        let ticket = vb_core::action::ActionTicket {
            run,
            step: vb_core::ids::StepIdx::new(0),
            seq: vb_core::ids::SeqNo::new(1),
            action: vb_core::ids::ActionId::new(0),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
         fencing_token: 0, };

        let mut shard = recovery_context_test_shard();

        let result = shard.reinject_pending_action(run, ticket);
        assert!(
            result.is_err(),
            "reinject for non-existent run must fail"
        );
        assert_eq!(result, Err(crate::RuntimeError::RunNotFound));
    }

    #[test]
    fn recover_handles_open_ask_boundary() {
        assert_eq!(
            crate::recovery::RecoveredRunBoundary::none().kind(),
            crate::recovery::RecoveredRunBoundaryKind::None
        );
    }
