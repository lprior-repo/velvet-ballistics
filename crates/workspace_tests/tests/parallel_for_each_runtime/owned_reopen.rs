use super::*;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::Taint;
use vb_core::root_bundle::OwnedRootBundle;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, RunHeaderRecord, RunSubmission, WorkflowSourceRecord};

#[test]
fn owned_strict_submission_drop_reopen_resumes_for_each() -> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: owned_parallel_foreach\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n";
    let workflow = vb_compile::compile_workflow(source.as_bytes())
        .map_err(|error| format!("compile: {error:?}"))?;
    let mut input_store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = input_store
        .insert_list(vec![
            SlotValue::I64(13),
            SlotValue::I64(7),
            SlotValue::I64(29),
        ])
        .map_err(|error| format!("input list: {error:?}"))?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing items input")?
        .slot;
    let roots = Box::from([(input, SlotValue::List(list), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots, input_store.checkpoint_v1(), &workflow)
        .map_err(|error| format!("root bundle: {error:?}"))?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .map_err(|error| format!("artifact: {error:?}"))?;
    let source_record = WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        vb_core::RunId::new(71_203),
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )
    .map_err(|error| format!("owned inputs: {error:?}"))?;
    let submission = RunSubmission::owned(
        source_record,
        artifact,
        inputs,
        RunHeaderRecord {
            run: vb_core::RunId::new(71_203),
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )
    .map_err(|error| format!("submission: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("tempdir: {error}"))?;
    let run = vb_core::RunId::new(71_203);
    {
        let journal = Arc::new(
            FjallJournal::open(directory.path(), None)
                .map_err(|error| format!("open journal: {error:?}"))?,
        );
        journal
            .commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)
            .map_err(|error| format!("commit submission: {error:?}"))?;
        let mut runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: vb_core::RuntimePolicy::Strict,
                step_budget_per_tick: 1,
                ..ShardConfig::default()
            },
            StorageRuntimeJournal::shared_strict(journal),
        );
        runtime
            .activate_owned_submission(run, source_digest)
            .map_err(|error| format!("activate: {error:?}"))?;
        for _ in 0..3 {
            runtime
                .tick_all()
                .map_err(|error| format!("pre-restart tick: {error:?}"))?;
        }
        if runtime.counters_snapshot().runs_completed != 0 {
            return Err("owned ForEach unexpectedly finished before cold restart".into());
        }
    }
    let journal = Arc::new(
        FjallJournal::open(directory.path(), None)
            .map_err(|error| format!("cold reopen: {error:?}"))?,
    );
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime
        .recover_and_resume(run)
        .map_err(|error| format!("recover and resume: {error:?}"))?;
    let finish = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            if let vb_core::CompiledNodeKind::Finish { result } = node.kind {
                Some((node.id, result))
            } else {
                None
            }
        })
        .ok_or("Finish missing")?;
    let mut observed = None;
    for _ in 0..128 {
        runtime
            .tick_all()
            .map_err(|error| format!("recovered tick: {error:?}"))?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        let state = runtime
            .inspect_run_state(run)
            .map_err(|error| format!("inspect recovered run: {error:?}"))?;
        if state.frame.pc() == finish.0 {
            let SlotValue::List(list) = *state
                .frame
                .read_slot(finish.1)
                .map_err(|error| error.to_string())?
            else {
                return Err("ForEach output is not a list".into());
            };
            observed = Some(
                state
                    .store
                    .list(list)
                    .map_err(|error| error.to_string())?
                    .to_vec(),
            );
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        observed.ok_or("preterminal recovered output was not observed")?,
        vec![SlotValue::I64(13), SlotValue::I64(7), SlotValue::I64(29)]
    );
    drop(runtime);
    drop(journal);
    let cold = FjallJournal::open(directory.path(), None)
        .map_err(|error| format!("terminal cold reopen: {error:?}"))?;
    let events = cold
        .events_for_run_full(run)
        .map_err(|error| format!("read terminal events: {error:?}"))?;
    assert!(matches!(
        events.last(),
        Some(vb_storage::JournalEvent::RunFinished { .. })
    ));
    Ok(())
}
