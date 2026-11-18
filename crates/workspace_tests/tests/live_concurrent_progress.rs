#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;
use tempfile::TempDir;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::{CompiledNodeKind, RunId, WorkflowDigest};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, RunHeaderRecord, RunSubmission, WorkflowSourceRecord};

const SOURCE: &str = "version: velvet-ballistics/v1\nname: live_concurrent_progress\nwhen: {manual: {}}\ninputs:\n  items: {is: list, of: any, max: 3}\nsteps:\n  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      at_once: 2\n      steps:\n        - id: leaf\n          set: {output: picked, value: '$entry'}\n  - id: done\n    finish: {result: '$outer'}\n";
const RUN: RunId = RunId::new(41_702);

fn submission()
-> Result<(TempDir, WorkflowDigest, vb_core::CompiledWorkflow), Box<dyn std::error::Error>> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let input = workflow.input_slots().first().ok_or("missing input")?.slot;
    let mut store = vb_core::ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list_with_taint(
        Box::new([SlotValue::I64(11), SlotValue::I64(22), SlotValue::I64(33)]),
        Box::new([Taint::Secret, Taint::Clean, Taint::DerivedFromSecret]),
    )?;
    let bundle = OwnedRootBundle::new(
        Box::new([(input, SlotValue::List(list), Taint::Secret)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )?;
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into());
    let source = WorkflowSourceRecord {
        digest: source_digest,
        source: SOURCE.as_bytes().to_vec(),
    };
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let submission = RunSubmission::owned(
        source,
        artifact,
        inputs,
        RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: u64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis(),
            )?,
        },
    )?;
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    journal.commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)?;
    Ok((directory, source_digest, workflow))
}

#[test]
fn live_owned_foreach_completes_across_small_runtime_ticks()
-> Result<(), Box<dyn std::error::Error>> {
    for step_budget_per_tick in [1, 2, 4] {
        let (directory, source_digest, workflow) = submission()?;
        let (body, join) = workflow
            .to_parts()
            .nodes
            .iter()
            .find_map(|node| {
                if let CompiledNodeKind::ForEachConcurrentStart { body, join, .. } = &node.kind {
                    Some((*body, *join))
                } else {
                    None
                }
            })
            .ok_or("missing compiled ForEachConcurrent")?;
        let output = workflow
            .node(join)
            .and_then(|node| node.output.slot())
            .ok_or("missing ForEach join output")?;
        let input = workflow
            .input_slots()
            .first()
            .ok_or("missing input slot")?
            .slot;
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let runtime_journal = StorageRuntimeJournal::shared_strict(journal.clone());
        let mut runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: vb_core::RuntimePolicy::Strict,
                max_active_runs: 1,
                step_budget_per_tick,
                ..ShardConfig::default()
            },
            runtime_journal,
        );
        runtime.activate_owned_submission(RUN, source_digest)?;
        let mut completed_output = None;
        for _ in 0..256 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_completed == 1 {
                break;
            }
            let state = runtime.inspect_run_state(RUN)?;
            if let Ok(SlotValue::List(list)) = state.frame.read_slot(output).copied() {
                let items = state.store.list_items_with_taint(list)?.collect::<Vec<_>>();
                if items.len() == 1 {
                    assert_eq!(
                        state.frame.step_state(body)?,
                        vb_core::StepState::Succeeded,
                        "budget={step_budget_per_tick} cut pc={:?}, frame={:?}, partial={items:?}",
                        state.frame.pc(),
                        state.frame
                    );
                    let SlotValue::List(input_list) = *state.frame.read_slot(input)? else {
                        return Err("owned input slot is not a list".into());
                    };
                    assert_eq!(
                        state
                            .store
                            .list_items_with_taint(input_list)?
                            .collect::<Vec<_>>(),
                        vec![
                            (SlotValue::I64(11), Taint::Secret),
                            (SlotValue::I64(22), Taint::Clean),
                            (SlotValue::I64(33), Taint::DerivedFromSecret),
                        ],
                        "input list taints remain intact at the cut"
                    );
                }
                if state.frame.pc() == join {
                    completed_output = Some(items);
                }
            }
        }
        assert_eq!(
            runtime.counters_snapshot().runs_completed,
            1,
            "budget={step_budget_per_tick} owned ForEach did not complete"
        );
        assert_eq!(
            completed_output.ok_or("completed ForEach output was not observed")?,
            vec![
                (SlotValue::I64(11), Taint::Secret),
                (SlotValue::I64(22), Taint::Secret),
                (SlotValue::I64(33), Taint::Secret),
            ]
        );
        let events = journal.events_for_run_full(RUN)?;
        assert_eq!(
            events.iter().filter(|event| matches!(event, vb_storage::JournalEvent::StepStarted { step, .. } if *step == body)).count(),
            3
        );
        assert_eq!(
            events.iter().filter(|event| matches!(event, vb_storage::JournalEvent::StepSucceeded { step, .. } if *step == body)).count(),
            3
        );
    }
    Ok(())
}
