use super::*;
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::action::ActionContract;
use vb_core::{RunId, capability::CapabilitySet, ids::ActionId};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_runtime::{RuntimeError, runtime::Runtime};

fn action_contract_abi_digest(
    contract: &ActionContract,
) -> Result<WorkflowDigest, Box<dyn std::error::Error>> {
    let bytes = postcard::to_allocvec(contract)?;
    Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
}

fn config(policy: RuntimePolicy) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy,
    }
}
fn runtime(
    journal: Arc<vb_storage::FjallJournal>,
    policy: RuntimePolicy,
) -> Result<Runtime, Box<dyn std::error::Error>> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("zero shard count")?,
        config(policy),
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
fn replacement_contracts_rejected(policy: RuntimePolicy) -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let workflow = action_workflow()?;
    let original = pure_contract()?;
    vb_storage::submit_artifact_with_contracts(
        journal.as_ref(),
        &workflow,
        policy,
        &[original.clone()],
    )?;
    let expected_abi = action_contract_abi_digest(&original)?;
    let mut runtime = runtime(journal.clone(), policy)?;
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
    // Replacement contracts are detected by the ABI digest check first,
    // producing AdmissionActionAbiDigestMismatch before artifact invalidity.
    let replacement_contract = contract()?;
    let submitted_abi = action_contract_abi_digest(&replacement_contract)?;
    assert_eq!(
        runtime.submit_direct_with_grants_and_contracts(
            RunId::new(32),
            workflow.clone(),
            CapabilitySet::empty(),
            Box::new([replacement_contract])
        ),
        Err(RuntimeError::AdmissionActionAbiDigestMismatch {
            action: ActionId::new(0),
            expected: expected_abi,
            submitted: submitted_abi,
        })
    );
    let replacement_contract2 = contract()?;
    let submitted_abi2 = action_contract_abi_digest(&replacement_contract2)?;
    assert_eq!(
        runtime.submit_direct_with_inputs_grants_and_contracts(
            RunId::new(33),
            workflow.clone(),
            Box::new([]),
            CapabilitySet::empty(),
            Box::new([replacement_contract2])
        ),
        Err(RuntimeError::AdmissionActionAbiDigestMismatch {
            action: ActionId::new(0),
            expected: expected_abi,
            submitted: submitted_abi2,
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
    let mut shard = Shard::new_with_journal(
        config(policy),
        StorageRuntimeJournal::shared_strict(journal),
    );
    let mut replacement = contract()?;
    replacement.idempotency = vb_core::Idempotency::AtLeastOnceExternal;
    replacement.retry_safety = vb_core::RetrySafety::Unsafe;
    let submitted_abi3 = action_contract_abi_digest(&replacement)?;
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run: RunId::new(35),
        workflow: workflow.clone(),
        caps: CapabilitySet::empty(),
        action_contracts: Box::new([replacement]),
    })?;
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::AdmissionActionAbiDigestMismatch {
            action: ActionId::new(0),
            expected: expected_abi,
            submitted: submitted_abi3,
        })
    );
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

fn changed_ir_rejected(policy: RuntimePolicy) -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let workflow = workflow()?;
    vb_storage::submit_artifact(journal.as_ref(), &workflow, policy)?;
    let runtime = runtime(journal.clone(), policy)?;
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
    let mut shard = Shard::new_with_journal(
        config(policy),
        StorageRuntimeJournal::shared_strict(journal),
    );
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

#[test]
fn public_runtime_rejects_replacement_execution_contracts_before_enqueue() -> TestResult {
    replacement_contracts_rejected(RuntimePolicy::Strict)
}

#[test]
fn journaled_runtime_and_processing_reject_replacement_execution_contracts() -> TestResult {
    replacement_contracts_rejected(RuntimePolicy::Journaled)
}

#[test]
fn public_runtime_and_processing_reject_changed_ir_with_retained_claimed_digest() -> TestResult {
    changed_ir_rejected(RuntimePolicy::Strict)
}

#[test]
fn journaled_runtime_and_processing_reject_changed_ir_with_retained_claimed_digest() -> TestResult {
    changed_ir_rejected(RuntimePolicy::Journaled)
}
