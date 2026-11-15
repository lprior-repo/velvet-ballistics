#![cfg(test)]

use super::{support::TestResult, wire};
use serde_json::{Value, json};
use std::process::Output;
use vb_storage::JournalEvent;

pub(super) fn report(events: &[JournalEvent], selected: &[usize]) -> TestResult<Value> {
    let events = selected
        .iter()
        .map(|index| event_value(events.get(*index).ok_or("expected event index")?))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"schema_version":"velvet-ballistics/cli-output/v1","kind":"events_report","run_id":"1","total":events.len(),"events":events}),
    )
}

fn event_value(event: &JournalEvent) -> TestResult<Value> {
    let mut value = event_payload(event).ok_or("unexpected event in events fixture")?;
    let Value::Object(fields) = &mut value else {
        return Err("event projection must be an object".into());
    };
    fields.insert("run".into(), json!(event.run_id().get()));
    fields.insert("seq".into(), json!(event.seq().get()));
    if let Some(attempt) = event.attempt() {
        fields.insert("attempt".into(), json!(attempt));
    }
    Ok(value)
}

fn event_payload(event: &JournalEvent) -> Option<Value> {
    accepted_or_authority_payload(event)
        .or_else(|| admission_payload(event))
        .or_else(|| step_payload(event))
        .or_else(|| terminal_payload(event))
}

fn accepted_or_authority_payload(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            Some(json!({"type":"RunAccepted","workflow":format!("{workflow:?}")}))
        }
        JournalEvent::CompatibilityDecisionEvent { decision, .. } => {
            Some(json!({"type":"CompatibilityDecisionEvent","decision":format!("{decision:?}")}))
        }
        JournalEvent::RunBudgetOriginEvent { started_millis, .. } => {
            Some(json!({"type":"RunBudgetOriginEvent","started_millis":started_millis}))
        }
        _ => None,
    }
}

fn admission_payload(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunAdmissionBudget {
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            budget,
            ..
        } => Some(json!({
            "type":"RunAdmissionBudget",
            "artifact_digest":format!("{artifact_digest:?}"),
            "input_digest":format!("{input_digest:?}"),
            "policy_pin":format!("{policy_pin:?}"),
            "granted_capabilities":format!("{granted_capabilities:?}"),
            "policy":format!("{policy:?}"),
            "budget":format!("{budget:?}")
        })),
        _ => None,
    }
}

fn step_payload(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::StepStarted { step, .. } => {
            Some(json!({"type":"StepStarted","step":step.get()}))
        }
        JournalEvent::SlotWrittenEvent {
            slot, value, extra, ..
        } => Some(json!({"type":"SlotWritten","slot":slot.get(),"value":value,"extra":extra})),
        JournalEvent::StepSucceeded { step, output, .. } => {
            Some(json!({"type":"StepSucceeded","step":step.get(),"output":output.get()}))
        }
        JournalEvent::AskScheduledEvent { step, .. } => {
            Some(json!({"type":"AskScheduled","step":step.get()}))
        }
        _ => None,
    }
}

fn terminal_payload(event: &JournalEvent) -> Option<Value> {
    match event {
        JournalEvent::RunCancelled { reason, .. } => {
            Some(json!({"type":"RunCancelled","reason":reason}))
        }
        JournalEvent::RunFailedEvent { .. } => Some(json!({"type":"RunFailed"})),
        _ => None,
    }
}

pub(super) fn text(selected: &[usize]) -> TestResult<String> {
    let all = [
        "  seq=0: RunAccepted\n",
        "  Unknown\n",
        "  Unknown\n",
        "  Unknown\n",
        "  seq=4: StepStarted step=0\n",
        "  seq=5: SlotWritten slot=0\n",
        "  seq=6: StepSucceeded step=0 output=0\n",
        "  seq=7: StepStarted step=1\n",
        "  seq=8: AskScheduled step=1\n",
        "  seq=9: RunCancelled\n",
        "  seq=10: RunFailed\n",
    ];
    let lines = selected
        .iter()
        .map(|index| all.get(*index).copied().ok_or("expected text index"))
        .collect::<Result<String, _>>()?;
    let count = selected.len();
    Ok(format!(
        "{lines}{count} event(s) total\nBDD-KYYF-002 command=events run_id=1 evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events={count}\n"
    ))
}

pub(super) fn assert_no_data(bytes: &[u8], emit: &str) -> TestResult {
    if emit == "text" {
        assert_eq!(std::str::from_utf8(bytes)?, "run 77: no events found\n");
    } else {
        assert_eq!(
            wire::decode(bytes, emit)?,
            json!({"success":false,"run_id":"77","status":"not_found","events":[],"total":0,"error":"run 77: no events found"})
        );
    }
    Ok(())
}

pub(super) fn assert_error(
    output: Output,
    emit: &str,
    code: i32,
    kind: &str,
    message: &str,
) -> TestResult {
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty());
    if emit == "text" {
        assert_eq!(String::from_utf8(output.stderr)?, format!("{message}\n"));
    } else {
        assert_eq!(
            wire::decode(&output.stderr, emit)?,
            json!({"success":false,"error":kind,"exit_code":code,"message":message})
        );
    }
    Ok(())
}

pub(super) fn assert_read_error(output: Output, emit: &str, message: &str) -> TestResult {
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    if emit == "text" {
        assert_eq!(String::from_utf8(output.stderr)?, format!("{message}\n"));
    } else {
        assert_eq!(
            wire::decode(&output.stderr, emit)?,
            json!({"success":false,"error":message})
        );
    }
    Ok(())
}

pub(super) fn assert_process_lock(output: Output, emit: &str) -> TestResult {
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    if emit == "text" {
        assert_eq!(
            String::from_utf8(output.stdout)?,
            "events run 1: storage is held by an active writer; public CLI surface is available\nBDD-KYYF-002 command=events run_id=1 evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events=0\n"
        );
    } else {
        assert_eq!(
            wire::decode(&output.stdout, emit)?,
            json!({"run_id":"1","command":"events","status":"writer_lock_held","surface":"available"})
        );
    }
    Ok(())
}
