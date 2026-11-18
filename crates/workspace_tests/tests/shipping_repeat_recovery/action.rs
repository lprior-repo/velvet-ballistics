//! Repeat bodies that suspend on recorded actions: one outcome per attempt,
//! durable action receipts, action retry authority left untouched.

use std::{num::NonZeroUsize, sync::Arc};

use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::{ActionId, SlotValue, StepIdx, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent as E};

use crate::support::{RUN, TestResult, counter_sequence, repeat_nodes, replay, started};

const ACTION_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_action\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: base, value: '10'}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: act\n          do: {action: '0', input: '$base'}\n  - id: done\n    finish: {result: '$step.r'}\n";

fn contract(action: ActionId, required: bool) -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("repeat-action").map_err(|error| format!("{error:?}"))?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: if required {
            Box::from([Capability::new(Box::from("repeat.action.required"), action)])
        } else {
            Box::new([])
        },
    })
}

fn contracts(target: ActionId) -> TestResult<Box<[ActionContract]>> {
    (0..=target.get())
        .map(|id| contract(ActionId::new(id), id == target.get()))
        .collect()
}

fn grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        Box::from("repeat.action.required"),
        action,
    )]))
}

fn output(slot: vb_core::SlotIdx, value: SlotValue) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady {
        output_slot: slot,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(encoded.len()).map_err(|_| "output too large")?,
    })
}

#[test]
fn repeat_action_outcomes_are_recorded_once_per_attempt() -> TestResult {
    let workflow = vb_compile::compile_workflow(ACTION_SOURCE.as_bytes())?;
    let nodes = repeat_nodes(&workflow, 2)?;
    let action = ActionId::new(0);
    let parts = workflow.to_parts();
    let action_step = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }))
        .ok_or("repeat Do node")?;
    let action_step_id = action_step.id;
    let action_slot = action_step.output.slot().unwrap_or(nodes.result);
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let contracts = contracts(action)?;
    vb_storage::admission::submit_artifact_with_contracts(
        journal.as_ref(),
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &contracts,
    )?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.submit_direct_with_inputs_grants_and_contracts(
        RUN,
        workflow.clone(),
        Box::new([]),
        grants(action),
        contracts,
    )?;

    for attempt in 0..2 {
        let ticket = loop_action(&mut runtime, action_step_id)?;
        assert_eq!(ticket.action, action);
        assert_eq!(ticket.step, action_step_id);
        runtime.complete_action_with_output(
            ticket,
            output(action_slot, SlotValue::I64(100 + attempt))?,
        )?;
    }
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(started(&events, nodes.body), 2);
    assert_eq!(started(&events, nodes.control), 2);
    assert_eq!(started(&events, nodes.finish), 1);
    assert_eq!(
        counter_sequence(&events, nodes.attempt)?,
        vec![(2, 0), (2, 1), (2, 2)]
    );
    let scheduled = events
        .iter()
        .filter(|event| match event {
            E::ActionScheduled { step, .. } => *step == action_step_id,
            E::ActionScheduledTicket { ticket, .. } => ticket.step == action_step_id,
            _ => false,
        })
        .count();
    let completed = events
        .iter()
        .filter(|event| match event {
            E::ActionCompletedEvent { step, .. } => *step == action_step_id,
            E::ActionCompletedEnvelope { ticket, .. } => ticket.step == action_step_id,
            _ => false,
        })
        .count();
    assert_eq!((scheduled, completed), (2, 2));
    let last_output = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::ActionCompletedEnvelope {
                ticket,
                value,
                taint,
                ..
            } if ticket.step == action_step_id => {
                Some((postcard::from_bytes::<SlotValue>(value).ok()?, *taint))
            }
            _ => None,
        })
        .ok_or("durable action completion envelope")?;
    assert_eq!(last_output, (SlotValue::I64(101), Taint::Clean));
    let retries = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                E::RetryCheckpoint { .. } | E::RetryTimerScheduled { .. }
            )
        })
        .count();
    assert_eq!(retries, 0, "repeat must not consume action retry authority");
    let results = events
        .iter()
        .filter_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(results, vec![nodes.result]);
    assert!(
        replay(&events, &workflow)?.is_none(),
        "action-bearing repeat traces stay outside scalar replay"
    );
    Ok(())
}

fn loop_action(runtime: &mut Runtime, step: StepIdx) -> TestResult<vb_core::action::ActionTicket> {
    for _ in 0..128 {
        runtime.tick_all().map_err(|error| format!("{error:?}"))?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(step.get())) {
            return Ok(ticket);
        }
    }
    Err("repeat action boundary not reached".into())
}
