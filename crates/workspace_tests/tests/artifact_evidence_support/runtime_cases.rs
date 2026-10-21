use super::*;
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{RunId, capability::CapabilitySet};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_runtime::{RuntimeError, runtime::Runtime};

fn config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: RuntimePolicy::Strict,
    }
}
fn runtime(journal: Arc<vb_storage::FjallJournal>) -> Result<Runtime, Box<dyn std::error::Error>> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("zero shard count")?,
        config(),
        StorageRuntimeJournal::shared_strict(journal),
    ))
}
fn pure_contract() -> Result<vb_core::ActionContract, Box<dyn std::error::Error>> {
    let mut contract = contract()?;
    contract.side_effect = vb_core::SideEffect::None;
    contract.idempotency = vb_core::Idempotency::DeterministicPure;
    contract.retry_safety = vb_core::RetrySafety::Safe;
    Ok(contract)
}

#[test]
fn public_runtime_rejects_replacement_execution_contracts_before_enqueue() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let workflow = action_workflow()?;
    let original = pure_contract()?;
    vb_storage::submit_artifact_with_contracts(
        journal.as_ref(),
        &workflow,
        RuntimePolicy::Strict,
        &[original.clone()],
    )?;
    let mut runtime = runtime(journal.clone())?;
    // Matching declarations establish this is a valid production submit path.
    assert_eq!(
        runtime.submit_direct_with_grants_and_contracts(
            RunId::new(31),
            workflow.clone(),
            CapabilitySet::empty(),
            Box::new([original])
        ),
        Ok(())
    );
    let mut replacement = contract()?;
    replacement.idempotency = vb_core::Idempotency::AtLeastOnceExternal;
    replacement.retry_safety = vb_core::RetrySafety::Unsafe;
    assert_eq!(
        runtime.submit_direct_with_grants_and_contracts(
            RunId::new(32),
            workflow.clone(),
            CapabilitySet::empty(),
            Box::new([replacement.clone()])
        ),
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: workflow.digest()
        })
    );
    assert_eq!(
        runtime.submit_direct_with_inputs_grants_and_contracts(
            RunId::new(33),
            workflow.clone(),
            Box::new([]),
            CapabilitySet::empty(),
            Box::new([replacement])
        ),
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: workflow.digest()
        })
    );
    assert_eq!(
        runtime.submit_direct(RunId::new(34), workflow.clone()),
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: workflow.digest()
        })
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    let mut shard =
        Shard::new_with_journal(config(), StorageRuntimeJournal::shared_strict(journal));
    let mut replacement = contract()?;
    replacement.idempotency = vb_core::Idempotency::AtLeastOnceExternal;
    replacement.retry_safety = vb_core::RetrySafety::Unsafe;
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run: RunId::new(35),
        workflow: workflow.clone(),
        caps: CapabilitySet::empty(),
        action_contracts: Box::new([replacement]),
    })?;
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: workflow.digest()
        })
    );
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn public_runtime_and_processing_reject_changed_ir_with_retained_claimed_digest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let workflow = workflow()?;
    vb_storage::submit_artifact(journal.as_ref(), &workflow, RuntimePolicy::Strict)?;
    let runtime = runtime(journal.clone())?;
    assert_eq!(
        runtime.submit_direct(RunId::new(40), workflow.clone()),
        Ok(())
    );
    let mut parts = workflow.to_parts();
    parts.constants = Box::new([vb_core::ConstValue::I64(99)]);
    let altered = vb_core::CompiledWorkflow::try_from_parts(parts)?;
    let expected = Err(RuntimeError::AdmissionArtifactInvalid {
        digest: workflow.digest(),
    });
    assert_eq!(
        runtime.submit_direct(RunId::new(41), altered.clone()),
        expected
    );
    assert_eq!(
        runtime.submit_compiled_with_inputs(RunId::new(42), altered.clone(), Box::new([])),
        expected
    );
    let mut shard =
        Shard::new_with_journal(config(), StorageRuntimeJournal::shared_strict(journal));
    shard.enqueue(ShardCommand::Submit {
        run: RunId::new(43),
        workflow: altered,
        caps: CapabilitySet::empty(),
    })?;
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: workflow.digest()
        })
    );
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn compiler_and_storage_reject_sparse_and_duplicate_dispatch_registries() -> TestResult {
    let workflow = action_workflow()?;
    let duplicate = [contract()?, contract()?];
    let mut parts = workflow.to_parts();
    for node in parts.nodes.iter_mut() {
        if let vb_core::CompiledNodeKind::Do { action, .. } = &mut node.kind {
            *action = vb_core::ActionId::new(7);
        }
    }
    let sparse_workflow = canonicalize(parts)?;
    let mut sparse = contract()?;
    sparse.id = vb_core::ActionId::new(7);
    for (workflow, contracts, index, actual, expected) in [
        (&workflow, duplicate.to_vec(), 1, 0, 1),
        (&sparse_workflow, vec![sparse], 0, 7, 0),
    ] {
        match vb_compile::verify_compiled_artifact(workflow, &contracts) {
            Err(ArtifactVerificationError::ActionRegistry(
                vb_core::action::ActionRegistryError::IndexMismatch {
                    index: i,
                    actual: a,
                    expected: e,
                },
            )) => assert_eq!((i, a.get(), e.get()), (index, actual, expected)),
            other => return Err(format!("compiler accepted invalid registry: {other:?}").into()),
        }
        match prepare_compiled_artifact(workflow, RuntimePolicy::Journaled, &contracts) {
            Err(JournalError::ArtifactActionRegistry(
                vb_core::action::ActionRegistryError::IndexMismatch {
                    index: i,
                    actual: a,
                    expected: e,
                },
            )) => assert_eq!((i, a.get(), e.get()), (index, actual, expected)),
            other => return Err(format!("storage accepted invalid registry: {other:?}").into()),
        }
    }
    Ok(())
}
