use super::{RUN, TestResult, cut, runtime};
use std::{path::Path, sync::Arc};
use vb_core::{
    ActionOutputReady, ActionTicket, CapabilitySet, CompiledNodeKind, RuntimePolicy, SlotIdx,
    SlotValue, StepIdx, Taint,
};
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal, runtime::Runtime};
use vb_storage::{FjallJournal, JournalEvent};

const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: strict_resume_prefix\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: work\n    do: {action: '0', input: '$seed'}\n  - id: observe\n    ask: {prompt: '$step.work'}\n  - id: done\n    finish: {result: '$step.work'}\n";

#[derive(Clone, Copy)]
pub(super) struct Issued {
    pub ticket: ActionTicket,
    pub output: SlotIdx,
    pub ask: StepIdx,
    pub ready: ActionOutputReady,
}

fn contract() -> TestResult<vb_core::ActionContract> {
    Ok(vb_core::ActionContract {
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
    })
}

fn pending(path: &Path, crashing: bool) -> TestResult<(Arc<FjallJournal>, Runtime, Issued)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    let action = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| matches!(node.kind, CompiledNodeKind::Do { .. }).then_some(node.id))
        .ok_or("compiled action absent")?;
    let ask = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }).then_some(node.id))
        .ok_or("compiled Ask absent")?;
    let output = workflow
        .node(action)
        .and_then(|node| node.output.slot())
        .ok_or("action output absent")?;
    let contract = contract()?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
    )?;
    let adapter: vb_runtime::journal::SharedRuntimeJournal = if crashing {
        Arc::new(cut::CutJournal::new(journal.clone()))
    } else {
        StorageRuntimeJournal::shared_strict(journal.clone())
    };
    let mut live = runtime(adapter)?;
    live.submit_direct_with_grants_and_contracts(
        RUN,
        workflow,
        CapabilitySet::empty(),
        Box::from([contract]),
    )?;
    let ticket = pending_action(&mut live, action)?;
    let index = vb_storage::keys::index_action_key(ticket.action, ticket.run, ticket.step)?;
    assert!(journal.has_action_index_entry(&index)?);
    let value = SlotValue::I64(73);
    let ready = ActionOutputReady {
        output_slot: output,
        value,
        taint: Taint::Random,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
    };
    Ok((
        journal,
        live,
        Issued {
            ticket,
            output,
            ask,
            ready,
        },
    ))
}

fn pending_action(live: &mut Runtime, action: StepIdx) -> TestResult<ActionTicket> {
    for _ in 0..12 {
        live.tick_all()?;
        if let Some(ticket) = live.lookup_pending_action_ticket(RUN, u64::from(action.get())) {
            return Ok(ticket);
        }
    }
    Err("issued ticket absent".into())
}

pub(super) fn pending_ask(live: &mut Runtime, ask: StepIdx) -> TestResult {
    for _ in 0..12 {
        live.tick_all()?;
        if live.lookup_pending_ask_ticket(RUN, ask).is_some() {
            return Ok(());
        }
    }
    Err("live Ask boundary absent".into())
}

pub(super) fn crashed(
    path: &Path,
    interrupted: bool,
) -> TestResult<(Arc<FjallJournal>, Runtime, Issued)> {
    let (journal, mut live, issued) = pending(path, true)?;
    live.complete_action_with_output(issued.ticket, issued.ready)?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    let mut state = live.inspect_run_state(RUN)?.clone();
    assert_eq!(state.frame.read_slot(issued.output)?, &SlotValue::I64(73));
    assert_eq!(state.frame.read_taint(issued.output)?, Taint::Random);
    assert_eq!(
        live.lookup_pending_action_ticket(RUN, u64::from(issued.ticket.step.get())),
        None
    );
    if interrupted {
        live.resume_run(RUN)?;
        assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
        let events = journal.events_for_run_full(RUN)?;
        let resumed = events
            .iter()
            .find(|event| matches!(event, JournalEvent::RunResumed { .. }))
            .ok_or("durable resume absent")?;
        state.journal_bytes_written = state
            .journal_bytes_written
            .checked_add(u64::try_from(postcard::to_allocvec(resumed)?.len())?)
            .ok_or("journal accounting overflow")?;
    }
    assert_eq!(live.inspect_run_state(RUN)?, &state);
    Ok((journal, live, issued))
}

pub(super) fn uninterrupted(path: &Path) -> TestResult<Runtime> {
    let (_journal, mut live, issued) = pending(path, false)?;
    live.complete_action_with_output(issued.ticket, issued.ready)?;
    pending_ask(&mut live, issued.ask)?;
    Ok(live)
}
