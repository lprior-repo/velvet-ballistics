use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{CapabilitySet, RunId, RuntimePolicy};
use vb_runtime::{
    RuntimeError,
    journal::VolatileRuntimeJournal,
    runtime::Runtime,
    shard::{Shard, ShardCommand, ShardConfig},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn unknown_artifact_has_identical_rejection_for_available_and_missing_secrets() -> TestResult {
    let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: unknown_secret_artifact\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: '$answer'}\n")?;
    let expected = RuntimeError::AdmissionArtifactNotFound {
        digest: workflow.digest(),
    };
    for available in [false, true] {
        let names: Box<[Box<str>]> = if available {
            Box::new([Box::from("TOKEN")])
        } else {
            Box::default()
        };
        let config = ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(NonZeroUsize::MIN, config, journal.clone());
        runtime.set_available_secret_names(names.clone());
        let run = RunId::new(1);
        for result in [
            runtime.submit_compiled(run, workflow.clone()),
            runtime.submit_compiled_with_inputs(run, workflow.clone(), Box::default()),
            runtime.submit_direct_with_grants_and_contracts(
                run,
                workflow.clone(),
                CapabilitySet::empty(),
                Box::default(),
            ),
            runtime.submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::default(),
                CapabilitySet::empty(),
                Box::default(),
            ),
        ] {
            assert_eq!(result, Err(expected.clone()));
        }
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(journal.snapshot()?, Vec::new());

        let mut shard = Shard::new_with_journal(config, journal.clone());
        shard.set_available_secret_names(names.into());
        for command in [
            ShardCommand::SubmitPrePersisted {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::SubmitWithInputs {
                run,
                workflow: workflow.clone(),
                inputs: Box::default(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::SubmitWithContracts {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
                action_contracts: Box::default(),
            },
            ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow: workflow.clone(),
                inputs: Box::default(),
                caps: CapabilitySet::empty(),
                action_contracts: Box::default(),
            },
        ] {
            assert_eq!(shard.enqueue(command), Err(expected.clone()));
            assert_eq!(shard.status().command_queue_depth, 0);
        }
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(journal.snapshot()?, Vec::new());
    }
    Ok(())
}

#[test]
fn accepted_artifact_capability_denial_precedes_presence_on_every_submit() -> TestResult {
    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    use vb_core::{ActionId, Capability};
    use vb_runtime::journal::StorageRuntimeJournal;

    let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: denied_secret_capability\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: invoke\n    do: {action: '0', input: '$answer'}\n  - id: done\n    finish: {result: '$step.invoke'}\n")?;
    let required = Capability::new("approved.action".into(), ActionId::new(0));
    let contracts = Box::new([ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("approved-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([required.clone()]),
    }]);
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let artifact = vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        contracts.as_ref(),
    )?;
    assert_eq!(
        vb_storage::admission::verify_accepted_artifact_for_policy(
            &artifact,
            RuntimePolicy::Strict
        )?
        .to_parts(),
        workflow.to_parts()
    );
    let expected = RuntimeError::AdmissionCapabilityDenied {
        action: ActionId::new(0),
        required: required.clone(),
        granted: CapabilitySet::empty(),
    };
    let config = ShardConfig {
        policy: RuntimePolicy::Strict,
        command_queue_capacity: 1,
        ..ShardConfig::default()
    };
    let run = RunId::new(41);
    let before = journal.events_for_run_full(run)?;
    for available in [false, true] {
        let names: Box<[Box<str>]> = if available {
            Box::new([Box::from("TOKEN")])
        } else {
            Box::default()
        };
        let mut runtime = Runtime::new(
            NonZeroUsize::MIN,
            config,
            StorageRuntimeJournal::shared_strict(journal.clone()),
        );
        runtime.set_available_secret_names(names.clone());
        for result in [
            runtime.submit_compiled(run, workflow.clone()),
            runtime.submit_compiled_with_inputs(run, workflow.clone(), Box::default()),
            runtime.submit_direct_with_grants_and_contracts(
                run,
                workflow.clone(),
                CapabilitySet::empty(),
                contracts.clone(),
            ),
            runtime.submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::default(),
                CapabilitySet::empty(),
                contracts.clone(),
            ),
        ] {
            assert_eq!(result, Err(expected.clone()));
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
        assert_eq!(journal.events_for_run_full(run)?, before);
        assert!(matches!(
            runtime.inspect_run_state(run),
            Err(RuntimeError::RunNotFound)
        ));

        let mut shard = Shard::new_with_journal(
            config,
            StorageRuntimeJournal::shared_strict(journal.clone()),
        );
        shard.set_available_secret_names(names.into());
        for command in [
            ShardCommand::SubmitPrePersisted {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::SubmitWithInputs {
                run,
                workflow: workflow.clone(),
                inputs: Box::default(),
                caps: CapabilitySet::empty(),
            },
            ShardCommand::SubmitWithContracts {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty(),
                action_contracts: contracts.clone(),
            },
            ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow: workflow.clone(),
                inputs: Box::default(),
                caps: CapabilitySet::empty(),
                action_contracts: contracts.clone(),
            },
        ] {
            assert_eq!(shard.enqueue(command), Err(expected.clone()));
            assert_eq!(shard.command_queue_len(), 0);
        }
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(journal.events_for_run_full(run)?, before);

        if available {
            runtime.submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::default(),
                CapabilitySet::from_grants(Box::new([required.clone()])),
                contracts.clone(),
            )?;
            runtime.tick_all()?;
            assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
            assert_eq!(
                journal
                    .events_for_run_full(run)?
                    .iter()
                    .filter(|event| matches!(event, vb_storage::JournalEvent::RunAccepted { .. }))
                    .count(),
                1
            );
        }
    }
    Ok(())
}
