use super::*;
use std::{num::NonZeroUsize, sync::Arc};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};

fn open(journal: Arc<vb_storage::FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Journaled,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

#[test]
fn durable_ask_recovery_rechecks_presence_before_any_new_journal_event() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: recover_secret\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: request\n    ask: {prompt: '$secrets.token'}\n  - id: done\n    finish: {result: '$step.request'}\n")?;
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let ask = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .ok_or("ask")?
        .id;
    let answer = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::AskResume { answer } => Some(answer),
            _ => None,
        })
        .ok_or("answer")?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[],
    )?;
    let run = RunId::new(70);
    {
        let mut runtime = open(journal.clone());
        runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
        runtime.submit_compiled(run, workflow.clone())?;
        runtime.tick_all()?;
        let ticket = runtime
            .lookup_pending_ask_ticket(run, ask)
            .ok_or("live ask")?;
        assert_eq!(ticket.run(), run);
    }
    let before = journal.events_for_run_full(run)?;
    let mut runtime = open(journal.clone());
    assert_eq!(
        runtime.recover_and_resume(run),
        Err(RuntimeError::SecretUnavailable)
    );
    assert_eq!(journal.events_for_run_full(run)?, before);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    runtime.recover_and_resume(run)?;
    runtime.tick_all()?;
    let state = runtime.inspect_run_state(run)?;
    let slot = binding.slot().ok_or("slot")?;
    assert_eq!(
        *state.frame.read_slot(slot)?,
        SlotValue::Symbol(binding.requirement())
    );
    assert_eq!(state.frame.read_taint(slot)?, Taint::Secret);
    assert_eq!(state.store.symbol(binding.requirement())?, "TOKEN");
    let ticket = runtime
        .lookup_pending_ask_ticket(run, ask)
        .ok_or("recovered ask")?;
    runtime.answer_ask(vb_runtime::shard::AskAnswer::new(
        ticket,
        answer,
        SlotValue::I64(9),
        Taint::Clean,
    ))?;
    runtime.tick_all()?;
    let events = journal.events_for_run_full(run)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}
