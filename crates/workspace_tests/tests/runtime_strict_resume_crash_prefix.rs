#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{CapabilitySet, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{
    RuntimeError,
    journal::{SharedRuntimeJournal, StorageRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalEvent};

#[path = "runtime_strict_resume_crash_prefix/cut.rs"]
mod cut;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(47241);
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: strict_resume_prefix\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: work\n    do: {action: '0', input: '$seed'}\n  - id: done\n    finish: {result: '$step.work'}\n";

fn runtime(journal: SharedRuntimeJournal) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        journal,
    )?)
}

fn rows(path: &Path) -> TestResult<Vec<(String, Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

#[test]
fn durable_completion_and_interrupted_resume_recover_without_reissuing_action() -> TestResult {
    crash_prefix(true)
}

#[test]
fn durable_completion_before_continuation_recovers_without_reissuing_action() -> TestResult {
    crash_prefix(false)
}

fn crash_prefix(interrupted: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    let action = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }).then_some(node.id)
        })
        .ok_or("compiled action absent")?;
    let output = workflow
        .node(action)
        .and_then(|node| node.output.slot())
        .ok_or("action output absent")?;
    let contract = vb_core::ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("strict-resume-prefix")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1_000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
    )?;
    let mut live = runtime(Arc::new(cut::CutJournal::new(journal.clone())))?;
    live.submit_direct_with_grants_and_contracts(
        RUN,
        workflow,
        CapabilitySet::empty(),
        Box::from([contract]),
    )?;
    for _ in 0..12 {
        live.tick_all()?;
        if live
            .lookup_pending_action_ticket(RUN, u64::from(action.get()))
            .is_some()
        {
            break;
        }
    }
    let ticket = live
        .lookup_pending_action_ticket(RUN, u64::from(action.get()))
        .ok_or("issued ticket absent")?;
    let value = SlotValue::I64(73);
    let ready = vb_core::ActionOutputReady {
        output_slot: output,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
    };
    live.complete_action_with_output(ticket, ready)?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    let mut state = live.inspect_run_state(RUN)?.clone();
    assert_eq!(state.frame.read_slot(output)?, &value);
    assert_eq!(
        live.lookup_pending_action_ticket(RUN, u64::from(action.get())),
        None
    );
    if interrupted {
        live.resume_run(RUN)?;
        assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    }
    let prefix = journal.events_for_run_full(RUN)?;
    assert_eq!(
        prefix
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunResumed { .. }))
            .count(),
        usize::from(interrupted)
    );
    if interrupted {
        let resumed = prefix
            .iter()
            .find(|event| matches!(event, JournalEvent::RunResumed { .. }))
            .ok_or("durable resume absent")?;
        state.journal_bytes_written = state
            .journal_bytes_written
            .checked_add(u64::try_from(postcard::to_allocvec(resumed)?.len())?)
            .ok_or("journal accounting overflow")?;
    }
    assert_eq!(live.inspect_run_state(RUN)?, &state);
    assert_eq!(prefix.iter().filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { ticket: actual, .. } if *actual == ticket)).count(), 1);
    let receipt = journal.run_inputs(RUN)?;
    drop(live);
    drop(journal);
    let before = rows(directory.path())?;

    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let product = recovered.recover_product(RUN)?;
    assert!(
        matches!(
            product,
            vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_)
        ),
        "{product:?}"
    );
    recovered.recover_and_resume(RUN)?;
    for _ in 0..12 {
        recovered.tick_all()?;
        if recovered.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(
        recovered.counters_snapshot().runs_completed,
        1,
        "state={:?} pending={:?} events={:?}",
        recovered.inspect_run_state(RUN),
        recovered.pending_boundary_snapshot(8),
        journal.events_for_run_full(RUN)?
    );
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(journal.run_inputs(RUN)?, receipt);
    drop(recovered);
    drop(journal);
    let terminal_rows = rows(directory.path())?;
    assert_ne!(terminal_rows, before);
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let counters = recovered.counters_snapshot();
    let boundary = recovered.pending_boundary_snapshot(8);
    assert_eq!(
        recovered.recover_and_resume(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("terminal_state")
        })
    );
    recovered.complete_action_with_output(ticket, ready)?;
    assert_eq!(recovered.tick_all(), Err(RuntimeError::RunNotFound));
    assert_eq!(recovered.counters_snapshot(), counters);
    assert_eq!(recovered.pending_boundary_snapshot(8), boundary);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(journal.run_inputs(RUN)?, receipt);
    drop(recovered);
    drop(journal);
    assert_eq!(rows(directory.path())?, terminal_rows);
    Ok(())
}
