#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{RunId, RuntimePolicy, SlotValue, ValueStore, WorkflowDigest};
use vb_runtime::runtime::completion_observation::{
    CompletionObservationSlot, CompletionValueObservation,
};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{FjallJournal, RunHeaderRecord, RunSubmission, WorkflowSourceRecord};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(94_401);
const KEY: &str = "VB_F4_RECOVERY_PRESENCE_ONLY";
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: retained_secret\nwhen: {manual: {}}\nsecrets: {token: VB_F4_RECOVERY_PRESENCE_ONLY}\nsteps:\n  - id: done\n    finish: {result: '$secrets.token'}\n";

#[path = "vb_f4_retained_recovery/hostile.rs"]
mod hostile;
#[path = "vb_f4_retained_recovery/owned.rs"]
mod owned;
#[path = "vb_f4_retained_recovery/wire.rs"]
mod wire;

fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    );
    runtime.set_available_secret_names(Box::new([KEY.into()]));
    runtime
}

fn commit_secret(path: &std::path::Path, owned: bool) -> TestResult {
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
        source: SOURCE.to_vec(),
    };
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    let submission = if owned {
        let bundle = vb_core::root_bundle::OwnedRootBundle::new(
            Box::new([]),
            ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1(),
            &workflow,
        )?;
        let inputs = vb_storage::OwnedRunInputsRecordV1::new(
            RUN,
            artifact.digest,
            source.digest,
            &bundle,
            &workflow,
        )?;
        RunSubmission::owned(source, artifact, inputs, header)?
    } else {
        let binding = workflow
            .source_metadata()
            .ok_or("metadata")?
            .secret_requirements()
            .first()
            .ok_or("binding")?;
        let inputs = vb_storage::RunInputsRecord::new(
            RUN,
            artifact.digest,
            source.digest,
            &workflow,
            Box::new([(
                binding.slot().ok_or("slot")?,
                SlotValue::Symbol(binding.requirement()),
            )]),
        )?;
        RunSubmission::new(source, artifact, inputs, header)?
    };
    let journal = FjallJournal::open(path, None)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(journal.events_for_run_full(RUN)?.len(), 3);
    Ok(())
}

fn reopen_and_finish_secret(owned: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    commit_secret(directory.path(), owned)?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut runtime = runtime(journal.clone());
    let product = runtime.recover_product(RUN)?;
    assert!(matches!(
        product,
        vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let mut observations = [CompletionObservationSlot::default()];
    let outcome = runtime
        .tick_all_observed(&mut observations)
        .map_err(|e| format!("observation: {e:?}"))?;
    outcome.execution?;
    outcome
        .observation
        .map_err(|e| format!("observation conflict: {e:?}"))?;
    assert_eq!(
        observations
            .first()
            .and_then(|slot| slot.first)
            .ok_or("completion")?
            .result,
        CompletionValueObservation::Restricted
    );
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    assert_eq!(journal.events_for_run_full(RUN)?.iter().filter(|event|
        matches!(event, vb_storage::JournalEvent::RunFinished { run, .. } if *run == RUN)).count(), 1);
    Ok(())
}

#[test]
fn pristine_scalar_secret_survives_real_fjall_close_reopen() -> TestResult {
    reopen_and_finish_secret(false)
}

#[test]
fn pristine_owned_secret_survives_real_fjall_close_reopen() -> TestResult {
    reopen_and_finish_secret(true)
}
