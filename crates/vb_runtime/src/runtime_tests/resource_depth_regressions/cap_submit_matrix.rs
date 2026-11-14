fn review_command_shapes(
    run: RunId,
    workflow: vb_core::CompiledWorkflow,
) -> Vec<crate::shard::ShardCommand> {
    use crate::shard::ShardCommand as Command;
    let caps = CapabilitySet::empty();
    let tenant = vb_core::ids::TenantId::new(963);
    vec![
        Command::Submit {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
        },
        Command::SubmitWithLifetimeCap {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
            max_run_time_seconds: std::num::NonZeroU64::MIN,
        },
        Command::SubmitPrePersisted {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
        },
        Command::SubmitWithInputs {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
            inputs: Box::default(),
        },
        Command::SubmitWithContracts {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
            action_contracts: Box::default(),
        },
        Command::SubmitWithInputsAndContracts {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
            inputs: Box::default(),
            action_contracts: Box::default(),
        },
        Command::SubmitTenantScoped {
            run,
            workflow: workflow.clone(),
            caps: caps.clone(),
            tenant,
        },
        Command::SubmitTenantWithInputsAndContracts {
            run,
            workflow,
            caps,
            tenant,
            inputs: Box::default(),
            action_contracts: Box::default(),
        },
        Command::Resume { run },
        Command::Cancel { run, reason: None },
        Command::Kill { run, reason: None },
    ]
}

#[test]
fn review_all_submit_and_control_shapes_keep_active_cap() -> Result<(), Box<dyn std::error::Error>>
{
    let low = review_low_wait_workflow(1)?;
    let mut high_parts = low.to_parts();
    high_parts.resource_contract.max_queue_depth = ResourceContract::DEFAULT.max_queue_depth;
    high_parts.digest = WorkflowDigest::from_bytes([0; 32]);
    high_parts.digest =
        WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&high_parts)?).into());
    let high = vb_core::CompiledWorkflow::try_from_parts(high_parts)?;
    let run = RunId::new(963);
    for command in review_command_shapes(run, high) {
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut shard = crate::shard::Shard::new_with_journal(runtime_config(), journal.clone());
        shard.enqueue(crate::shard::ShardCommand::Submit {
            run,
            workflow: low.clone(),
            caps: CapabilitySet::empty(),
        })?;
        assert_eq!(shard.tick(), Ok(true));
        let before = journal.snapshot()?;
        let frame = shard
            .run_state_get(run)
            .ok_or("active run")?
            .frame
            .checkpoint_v1();
        let quota = shard.global_cost_usage();
        shard.enqueue(command.clone())?;
        assert_eq!(
            shard.enqueue(command),
            Err(RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: 1,
                limit: 1
            })
        );
        assert_eq!(shard.command_queue_pending_for(run), 1);
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.global_cost_usage(), quota);
        assert_eq!(
            shard
                .run_state_get(run)
                .ok_or("active run")?
                .frame
                .checkpoint_v1(),
            frame
        );
        assert_eq!(journal.snapshot()?, before);
    }
    Ok(())
}
