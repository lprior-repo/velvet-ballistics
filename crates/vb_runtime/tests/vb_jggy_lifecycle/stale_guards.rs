// tests/ticket_stale_guards.rs
// Tests that stale (superseded) action tickets are rejected on delivery.

use super::*;
use std::sync::Arc;

#[derive(Debug)]
enum StaleTamper {
    Step,
    Action,
    Capacity,
    Key,
    Sequence,
}

#[derive(Debug)]
enum Delivery {
    Completion,
    Failure,
}

fn check(tamper: &StaleTamper, delivery: &Delivery) -> Result<(), Box<dyn std::error::Error>> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());

    let workflow = retry_workflow().ok_or("no retry workflow")?;
    let run = RunId::new(100);
    submit_with_contracts(&shard, run, workflow);
    assert_eq!(shard.tick(), Ok(true));
    let ticket1 = shard.pending_action_get(run).ok_or("missing ticket1")?;
    assert_eq!(ticket1.attempt, 1);

    shard.enqueue(ShardCommand::ActionFailed {
        ticket: ticket1,
        failure: retryable_failure(),
    })?;
    assert_eq!(shard.tick(), Ok(true));
    let ticket2 = shard.pending_action_get(run).ok_or("missing ticket2")?;

    shard.enqueue(ShardCommand::ActionFailed {
        ticket: ticket2,
        failure: retryable_failure(),
    })?;
    assert_eq!(shard.tick(), Ok(true));
    let ticket3 = shard.pending_action_get(run).ok_or("missing ticket3")?;

    assert_eq!(ticket2.attempt, 2);
    assert_eq!(ticket3.attempt, 3);

    let current_ticket = ticket3;
    let snap_before = journal.snapshot()?;

    let frame_before = &shard.run_state_get(run).ok_or("frame missing")?.frame;
    let pc_before = frame_before.pc();
    let slot0_before = *frame_before.read_slot(SlotIdx::ZERO)?;
    let slot1_before = *frame_before.read_slot(SlotIdx::new(1))?;
    let taint0_before = frame_before.read_taint(SlotIdx::ZERO)?;
    let taint1_before = frame_before.read_taint(SlotIdx::new(1))?;

    let mut stale = ticket2;
    match tamper {
        StaleTamper::Step => {
            stale.step = StepIdx::new(99);
        }
        StaleTamper::Action => {
            stale.action = ActionId::new(99);
            stale.idempotency_key =
                vb_core::action::compute_action_idempotency_key(stale.run, stale.seq, stale.action);
        }
        StaleTamper::Capacity => {
            stale.capacity = stale.capacity.checked_add(1).ok_or("cap overflow")?;
        }
        StaleTamper::Key => {
            stale.idempotency_key ^= 1;
        }
        StaleTamper::Sequence => {
            let seq_plus = current_ticket
                .seq
                .get()
                .checked_add(1)
                .ok_or("seq overflow")?;
            stale.seq = SeqNo::new(seq_plus);
            stale.idempotency_key =
                vb_core::action::compute_action_idempotency_key(stale.run, stale.seq, stale.action);
        }
    }

    let command = match delivery {
        Delivery::Completion => ShardCommand::ActionCompleted {
            ticket: stale,
            output: ActionOutputReady {
                output_slot: SlotIdx::ZERO,
                value: SlotValue::I64(7),
                taint: Taint::Clean,
                encoded_len: 3,
            },
        },
        Delivery::Failure => ShardCommand::ActionFailed {
            ticket: stale,
            failure: non_retryable_failure(),
        },
    };
    shard.enqueue(command)?;
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::InvalidActionCompletion),
        "{tamper:?} via {delivery:?}"
    );

    let snap_after = journal.snapshot()?;
    assert_eq!(snap_before, snap_after);

    let current_pending = shard.pending_action_get(run).ok_or("current ticket gone")?;
    assert_eq!(current_pending, current_ticket);

    let frame_after = &shard.run_state_get(run).ok_or("frame gone after")?.frame;
    assert_eq!(pc_before, frame_after.pc());
    assert_eq!(slot0_before, *frame_after.read_slot(SlotIdx::ZERO)?);
    assert_eq!(slot1_before, *frame_after.read_slot(SlotIdx::new(1))?);
    assert_eq!(taint0_before, frame_after.read_taint(SlotIdx::ZERO)?);
    assert_eq!(taint1_before, frame_after.read_taint(SlotIdx::new(1))?);

    Ok(())
}

#[test]
fn stale_completions_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for tamper in [
        StaleTamper::Step,
        StaleTamper::Action,
        StaleTamper::Capacity,
        StaleTamper::Key,
        StaleTamper::Sequence,
    ] {
        check(&tamper, &Delivery::Completion)?;
    }
    Ok(())
}

#[test]
fn stale_failures_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for tamper in [
        StaleTamper::Step,
        StaleTamper::Action,
        StaleTamper::Capacity,
        StaleTamper::Key,
        StaleTamper::Sequence,
    ] {
        check(&tamper, &Delivery::Failure)?;
    }
    Ok(())
}
