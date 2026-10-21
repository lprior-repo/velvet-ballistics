    // Uses imports from earlier chunk_001.rs: Shard, ShardConfig, RuntimeError, etc.
    // No additional use statements needed.

    fn contract_validation_make_contract(
        id: u16,
        name: &str,
    ) -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
        let action_name = vb_core::action::ActionName::new(name)?;
        Ok(vb_core::action::ActionContract {
            id: vb_core::ids::ActionId::new(id),
            name: action_name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            side_effect: vb_core::action::SideEffect::None,
            retry_safety: vb_core::action::RetrySafety::Safe,
            required_capabilities: Box::new([]),
        })
    }

    fn contract_validation_test_shard() -> crate::shard::Shard {
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
    fn validate_contracts_rejects_duplicate_action_ids() -> Result<(), Box<dyn std::error::Error>> {
        let shard = contract_validation_test_shard();

        let run = vb_core::ids::RunId::new(1);
        let digest = vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]);
        let caps = vb_core::capability::CapabilitySet::empty();

        let contract = contract_validation_make_contract(0, "foo")?;
        let contracts = [contract.clone(), contract.clone()];
        let result = shard.validate_submit_admission_with_contracts(
            run,
            digest,
            caps,
            &contracts,
        );
        assert_eq!(result, Err(crate::RuntimeError::ContractValidationError {
            action: vb_core::ids::ActionId::new(0),
            error: "duplicate action ID".to_string(),
        }));
        Ok(())
    }

    #[test]
    fn validate_contracts_accepts_well_formed_contracts() -> Result<(), Box<dyn std::error::Error>> {
        let shard = contract_validation_test_shard();

        let run = vb_core::ids::RunId::new(1);
        let digest = vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]);
        let caps = vb_core::capability::CapabilitySet::empty();

        let contract = contract_validation_make_contract(0, "real_action")?;
        let result = shard.validate_submit_admission_with_contracts(
            run,
            digest,
            caps,
            &[contract],
        );
        assert!(
            result.is_ok(),
            "well-formed contracts must be accepted"
        );
        Ok(())
    }
