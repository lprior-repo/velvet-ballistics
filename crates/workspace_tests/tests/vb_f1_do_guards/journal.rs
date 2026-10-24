use vb_core::Taint;
use vb_core::ids::{RunId, SlotIdx};
use vb_runtime::journal::RuntimeJournalEvent;

use crate::guard_support::decode_slot_value;
use vb_core::SlotValue;

fn expected_scheduled_pairs(smoke_passed: bool) -> Vec<(SlotIdx, SlotIdx)> {
    let mut pairs = vec![
        (SlotIdx::new(0), SlotIdx::new(1)),
        (SlotIdx::new(1), SlotIdx::new(2)),
    ];
    if !smoke_passed {
        pairs.push((SlotIdx::new(1), SlotIdx::new(3)));
    }
    pairs
}

fn verify_scheduled(
    events: &[RuntimeJournalEvent],
    tickets: &[vb_core::action::ActionTicket],
    smoke_passed: bool,
) -> Result<(), String> {
    let expected_count = if smoke_passed { 2 } else { 3 };
    if tickets.len() != expected_count {
        return Err(format!(
            "tickets.len() {} != expected {}",
            tickets.len(),
            expected_count
        ));
    }

    let scheduled: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                ..
            } => Some((ticket.clone(), *input, *output)),
            _ => None,
        })
        .collect();

    if scheduled.len() != expected_count {
        return Err(format!(
            "expected {} scheduled, got {}",
            expected_count,
            scheduled.len()
        ));
    }

    let slot_pairs = expected_scheduled_pairs(smoke_passed);
    let expected: Vec<_> = tickets
        .iter()
        .zip(slot_pairs.iter())
        .map(|(t, (inp, out))| (t.clone(), *inp, *out))
        .collect();

    if scheduled != expected {
        return Err("scheduled tickets mismatch".to_string());
    }

    Ok(())
}

fn expected_completion_values(smoke_passed: bool) -> Vec<(SlotIdx, SlotValue)> {
    let mut vals = vec![
        (SlotIdx::new(1), SlotValue::I64(42)),
        (SlotIdx::new(2), SlotValue::Bool(smoke_passed)),
    ];
    if !smoke_passed {
        vals.push((SlotIdx::new(3), SlotValue::I64(7)));
    }
    vals
}

fn verify_completed(
    events: &[RuntimeJournalEvent],
    tickets: &[vb_core::action::ActionTicket],
    smoke_passed: bool,
) -> Result<(), String> {
    let expected_count = if smoke_passed { 2 } else { 3 };

    let completed: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                taint,
                ..
            } => Some((ticket.clone(), *output, value.clone(), *taint)),
            _ => None,
        })
        .collect();

    if completed.len() != expected_count {
        return Err(format!(
            "expected {} completed, got {}",
            expected_count,
            completed.len()
        ));
    }

    let slot_vals = expected_completion_values(smoke_passed);
    let expected_tickets: Vec<_> = tickets.iter().cloned().collect();

    for (i, (actual, (expected_slot, expected_val))) in
        completed.iter().zip(slot_vals.iter()).enumerate()
    {
        let expected_ticket = expected_tickets
            .get(i)
            .ok_or_else(|| format!("missing expected ticket at {}", i))?;
        if actual.0 != *expected_ticket || actual.1 != *expected_slot || actual.3 != Taint::Clean {
            return Err(format!("completed envelope {} mismatch", i));
        }
        let decoded =
            decode_slot_value(&actual.2).map_err(|e| format!("decode error {}: {}", i, e))?;
        if decoded != *expected_val {
            return Err(format!("completed value {} mismatch", i));
        }
    }

    Ok(())
}

fn verify_seed_write(events: &[RuntimeJournalEvent], run: RunId) -> Result<(), String> {
    let seed_writes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value,
                taint,
                ..
            } if *slot == SlotIdx::new(0) => Some((*run, value.clone(), *taint)),
            _ => None,
        })
        .collect();

    if seed_writes.len() != 1 {
        return Err(format!("expected 1 seed write, got {}", seed_writes.len()));
    }

    let seed = seed_writes
        .get(0)
        .ok_or_else(|| "seed_writes is empty".to_string())?;
    if seed.0 != run || seed.2 != Taint::Clean {
        return Err("seed write mismatch".to_string());
    }
    let decoded = decode_slot_value(&seed.1).map_err(|e| format!("seed decode error: {}", e))?;
    if decoded != SlotValue::I64(42) {
        return Err("seed value mismatch".to_string());
    }

    Ok(())
}

fn verify_run_finished(events: &[RuntimeJournalEvent], run: RunId) -> Result<(), String> {
    let finished: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RuntimeJournalEvent::RunFinished {
                run,
                result,
                attempt,
            } => Some((*run, *result, *attempt)),
            _ => None,
        })
        .collect();

    if finished.len() != 1 {
        return Err(format!("expected 1 run finished, got {}", finished.len()));
    }

    let fin = finished
        .get(0)
        .ok_or_else(|| "finished is empty".to_string())?;
    if fin.0 != run || fin.1 != SlotIdx::new(1) || fin.2 != 1u16 {
        return Err("run finished mismatch".to_string());
    }

    Ok(())
}

fn verify_no_failures(events: &[RuntimeJournalEvent], smoke_passed: bool) -> Result<(), String> {
    if events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::RunFailed { .. }))
    {
        return Err("unexpected RunFailed event".to_string());
    }
    if smoke_passed && events.iter().any(|e| {
        matches!(e, RuntimeJournalEvent::SlotWritten { slot, .. } if *slot == SlotIdx::new(3))
    }) {
        return Err("unexpected slot3 write on smoke pass".to_string());
    }
    Ok(())
}

pub(super) fn verify(
    events: &[RuntimeJournalEvent],
    run: RunId,
    tickets: &[vb_core::action::ActionTicket],
    smoke_passed: bool,
) -> Result<(), String> {
    verify_scheduled(events, tickets, smoke_passed)?;
    verify_completed(events, tickets, smoke_passed)?;
    verify_seed_write(events, run)?;
    verify_run_finished(events, run)?;
    verify_no_failures(events, smoke_passed)?;
    Ok(())
}
