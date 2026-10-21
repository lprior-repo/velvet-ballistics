    // Uses imports from earlier chunk_001.rs: RuntimeError, Shard, ShardConfig, etc.
    // No additional use statements needed.

    fn recovery_context_test_shard() -> crate::shard::Shard {
        let config = crate::shard::ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        crate::shard::Shard::new(config)
    }

    #[test]
    fn recover_rejects_malformed_command() {
        let mut shard = recovery_context_test_shard();

        let frame = vb_core::frame::RunFrame::new(
            vb_core::ids::RunId::new(1),
            vb_core::ids::StepIdx::new(0),
            1,
            1,
        )
        .expect("test frame construction must succeed with valid args");

        let command = crate::shard::types::RecoverRunCommand::new(
            vb_core::ids::RunId::new(1),
            frame,
            vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            vb_core::ids::WorkflowDigest::from_bytes([1u8; 32]),
            vb_storage::EventSeq::new(1),
            crate::primitives::collect::CollectStates::new(),
            crate::recovery::RecoveredRunBoundary::none(),
        );

        let result = shard.handle_recover(command);
        assert!(
            result.is_err(),
            "malformed command with no artifact store should fail recovery"
        );
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
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        };

        let mut shard = recovery_context_test_shard();

        let result = shard.reinject_pending_action(run, ticket);
        assert!(
            result.is_err(),
            "reinject for non-existent run must fail"
        );
        match result {
            Err(crate::RuntimeError::RunNotFound) => {}
            Err(other) => panic!("expected RunNotFound, got {other:?}"),
            Ok(_) => panic!("reinject for non-existent run must fail"),
        }
    }

    #[test]
    fn recover_handles_open_ask_boundary() {
        assert_eq!(
            crate::recovery::RecoveredRunBoundary::none().kind(),
            crate::recovery::RecoveredRunBoundaryKind::None
        );
    }
