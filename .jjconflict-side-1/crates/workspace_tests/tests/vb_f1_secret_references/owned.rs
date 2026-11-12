use super::*;
use std::{num::NonZeroUsize, sync::Arc};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};

#[test]
fn strict_owned_activation_rechecks_host_presence_and_preserves_canonical_reference() -> TestResult
{
    let source = b"version: velvet-ballistics/v1\nname: owned_secret\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: done\n    finish: {result: '$secrets.token'}\n";
    let workflow = vb_compile::compile_workflow(source)?;
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::new([]),
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1(),
        &workflow,
    )?;
    assert_eq!(
        bundle.roots(),
        &[(
            binding.slot().ok_or("slot")?,
            SlotValue::Symbol(binding.requirement()),
            Taint::Secret
        )]
    );
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(source).into()),
        source: source.to_vec(),
    };
    let run = RunId::new(71);
    let record = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &bundle,
        &workflow,
    )?;
    let submission = vb_storage::RunSubmission::owned(
        source.clone(),
        artifact.clone(),
        record,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)?;
    let initial = journal.events_for_run_full(run)?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    assert_eq!(
        runtime.activate_owned_submission(run, source.digest),
        Err(RuntimeError::SecretUnavailable)
    );
    assert_eq!(journal.events_for_run_full(run)?, initial);
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    runtime.activate_owned_submission(run, source.digest)?;
    runtime.set_available_secret_names(Box::new([]));
    assert_eq!(runtime.tick_all(), Err(RuntimeError::SecretUnavailable));
    assert_eq!(journal.events_for_run_full(run)?, initial);
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    runtime.activate_owned_submission(run, source.digest)?;
    runtime.tick_all()?;
    let retained = journal
        .owned_run_inputs(run, artifact.digest, source.digest, &workflow)?
        .ok_or("owned record")?;
    assert_eq!(retained.roots(), bundle.roots());
    assert_eq!(retained.checkpoint(), bundle.checkpoint());
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}
