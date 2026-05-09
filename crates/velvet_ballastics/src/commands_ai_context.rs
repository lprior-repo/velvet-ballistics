//! AI context CLI command.
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::{Map, Value};

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    Running,
    Finished,
    Failed,
    Cancelled,
}

pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };
    let events = match journal.events_for_run(rid) {
        Ok(events) if !events.is_empty() => events,
        Ok(_) => return report_run_not_found(run_id, output),
        Err(e) => {
            report_journal_read_error("events", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let header = match journal.run_header(rid) {
        Ok(header) => header,
        Err(e) => {
            report_journal_read_error("run header", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let digest = header
        .as_ref()
        .map(|header| header.compiled_digest)
        .or_else(|| workflow_digest_from_events(&events));
    let latest_snapshot = match latest_snapshot_for_run(&journal, rid, &events) {
        Ok(snapshot) => snapshot,
        Err(e) => {
            report_journal_read_error("snapshot", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let workflow = ai_workflow_summary(&journal, digest);
    let status = run_status_from_events(&events);
    let packet = serde_json::json!({
        "schema_version": "1",
        "kind": "AiContextPacket",
        "run_id": rid.get(),
        "workflow": workflow,
        "journal_event_trail": ai_journal_events(&events, latest_snapshot.as_ref()),
        "action_contracts": ai_action_contracts(&events, workflow.get("referenced_actions")),
        "trace_ring_snapshot": trace_ring_snapshot(),
        "suggested_next_cli_commands": suggested_ai_commands(run_id, db, status),
    });
    json_out(&packet, output);
    ExitCode::SUCCESS
}

fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            write_stderr_line(format_args!("invalid run_id '{raw}': {e}"));
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

fn report_storage_open_error(
    e: &vb_storage::JournalError,
    db: &std::path::Path,
    output: OutputFormat,
) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("error opening journal at {}: {e}", db.display())
            }),
            output,
        );
    } else {
        write_stderr_line(format_args!(
            "error opening journal at {}: {e}",
            db.display()
        ));
    }
}

fn report_run_not_found(run_id: &str, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "code": "RUN_NOT_FOUND",
                "run_id": run_id,
            }),
            output,
        );
    } else {
        write_stderr_line(format_args!("RUN_NOT_FOUND: run {run_id}"));
    }
    CliExitCode::ValidationFailed.into()
}

fn report_journal_read_error(
    area: &str,
    run_id: &str,
    e: &vb_storage::JournalError,
    output: OutputFormat,
) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("error reading {area} for run {run_id}: {e}")
            }),
            output,
        );
    } else {
        write_stderr_line(format_args!("error reading {area} for run {run_id}: {e}"));
    }
}

fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn latest_snapshot_for_run(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    events: &[vb_storage::JournalEvent],
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    latest_snapshot_from_events(events, |seq| journal.snapshot(run, seq))
}

fn latest_snapshot_from_events(
    events: &[vb_storage::JournalEvent],
    mut snapshot_at: impl FnMut(
        vb_storage::EventSeq,
    )
        -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError>,
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    events.iter().rev().try_fold(None, |found, event| {
        if found.is_some() {
            Ok(found)
        } else {
            snapshot_at(event.seq())
        }
    })
}

fn ai_workflow_summary(
    journal: &vb_storage::FjallJournal,
    digest: Option<vb_core::WorkflowDigest>,
) -> Value {
    let Some(digest) = digest else {
        return serde_json::json!({
            "digest": null,
            "compiled_ir": {"available": false, "reason": "workflow digest not present in run header or events"},
            "source_included": false,
        });
    };
    let record = match journal.compiled_ir(digest) {
        Ok(Some(record)) => record,
        Ok(None) => return workflow_summary_from_source(journal, digest),
        Err(e) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": format!("compiled IR read error: {e}")},
                "source_included": false,
            });
        }
    };
    match postcard::from_bytes::<vb_core::WorkflowParts>(&record.ir)
        .ok()
        .and_then(|parts| vb_core::CompiledWorkflow::try_from_parts(parts).ok())
    {
        Some(compiled) => compiled_workflow_summary(digest, &compiled),
        None => serde_json::json!({
            "digest": digest_hex(digest),
            "compiled_ir": {"available": false, "reason": "compiled IR decode failed"},
            "source_included": false,
        }),
    }
}

fn workflow_summary_from_source(
    journal: &vb_storage::FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Value {
    let source = match journal.workflow_source(digest) {
        Ok(Some(record)) => record.source,
        Ok(None) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": "compiled IR and workflow source not found"},
                "source_included": false,
            });
        }
        Err(e) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": format!("compiled IR not found; workflow source read error: {e}")},
                "source_included": false,
            });
        }
    };
    match vb_compile::compile_workflow(&source) {
        Ok(compiled) => compiled_workflow_summary(digest, &compiled),
        Err(e) => serde_json::json!({
            "digest": digest_hex(digest),
            "compiled_ir": {"available": false, "reason": format!("compiled IR not found; workflow source compile failed: {e}")},
            "source_included": false,
        }),
    }
}

fn compiled_workflow_summary(
    digest: vb_core::WorkflowDigest,
    compiled: &vb_core::CompiledWorkflow,
) -> Value {
    let nodes: Vec<Value> = (0..compiled.node_count())
        .filter_map(|raw| compiled_node_json(compiled, raw))
        .collect();
    serde_json::json!({
        "digest": digest_hex(digest),
        "compiled_ir": {
            "available": true,
            "name": compiled.name(),
            "entry": compiled.entry().get(),
            "node_count": compiled.node_count(),
            "slot_count": compiled.slot_count(),
            "resource_contract": compiled.resource_contract(),
            "nodes": nodes,
        },
        "referenced_actions": referenced_actions(compiled),
        "source_included": false,
    })
}

fn compiled_node_json(compiled: &vb_core::CompiledWorkflow, raw: u16) -> Option<Value> {
    let step = vb_core::StepIdx::new(raw);
    compiled.node(step).map(|node| {
        serde_json::json!({
            "step": raw,
            "name": compiled.step_name(step),
            "kind": node_kind_name(&node.kind),
            "output": node.output.map(|slot| slot.get()),
            "next": node.next.map(|next| next.get()),
        })
    })
}

fn referenced_actions(compiled: &vb_core::CompiledWorkflow) -> Vec<u32> {
    (0..compiled.node_count())
        .filter_map(|raw| compiled.node(vb_core::StepIdx::new(raw)))
        .filter_map(|node| match &node.kind {
            vb_core::workflow::CompiledNodeKind::Do { action, .. } => Some(u32::from(action.get())),
            _ => None,
        })
        .fold(Vec::<u32>::new(), push_unique_u32)
}

fn workflow_digest_from_events(
    events: &[vb_storage::JournalEvent],
) -> Option<vb_core::WorkflowDigest> {
    events.iter().find_map(|event| match event {
        vb_storage::JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
        _ => None,
    })
}

fn push_unique_u32(mut values: Vec<u32>, value: u32) -> Vec<u32> {
    if !values.contains(&value) {
        values.push(value);
    }
    values
}

fn ai_journal_events(
    events: &[vb_storage::JournalEvent],
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Vec<Value> {
    events
        .iter()
        .map(|event| ai_event_to_json(event, snapshot))
        .collect()
}

fn ai_event_to_json(
    event: &vb_storage::JournalEvent,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    let value = event_to_json(event);
    match (event, value) {
        (
            vb_storage::JournalEvent::SlotWrittenEvent {
                slot, value: bytes, ..
            },
            Value::Object(object),
        ) => Value::Object(Map::from_iter(object.into_iter().chain([
            ("slot".to_string(), Value::from(slot.get())),
            (
                "value".to_string(),
                redacted_slot_value(*slot, bytes.as_ref(), snapshot),
            ),
        ]))),
        (_, value) => value,
    }
}

pub(crate) fn redacted_slot_value(
    slot: vb_core::SlotIdx,
    value: Option<&Vec<u8>>,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    if slot_is_secret_or_derived(slot, snapshot) {
        return Value::String("[REDACTED]".to_string());
    }
    value.map_or(Value::Null, |bytes| {
        postcard::from_bytes::<vb_core::SlotValue>(bytes)
            .map_or(Value::String("[UNDECODED]".to_string()), |slot_value| {
                Value::String(slot_value.to_string())
            })
    })
}

fn slot_is_secret_or_derived(
    slot: vb_core::SlotIdx,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> bool {
    snapshot
        .and_then(|snapshot| snapshot.taint.get(slot.as_usize()))
        .is_some_and(|raw| matches!(*raw, 1 | 2))
}

fn ai_action_contracts(
    events: &[vb_storage::JournalEvent],
    workflow_actions: Option<&Value>,
) -> Value {
    let workflow_ids = workflow_actions
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_u64().and_then(|raw| u32::try_from(raw).ok()));
    let event_ids = events.iter().filter_map(|event| match event {
        vb_storage::JournalEvent::ActionScheduled { action, .. }
        | vb_storage::JournalEvent::ActionCompletedEvent { action, .. }
        | vb_storage::JournalEvent::ActionFailedEvent { action, .. } => {
            Some(u32::from(action.get()))
        }
        _ => None,
    });
    Value::Array(
        workflow_ids
            .chain(event_ids)
            .fold(Vec::<u32>::new(), push_unique_u32)
            .into_iter()
            .map(inferred_action_contract_json)
            .collect(),
    )
}

fn inferred_action_contract_json(action: u32) -> Value {
    serde_json::json!({
        "action": action,
        "contract_status": "inferred_from_compiled_ir_and_journal",
        "contract": {
            "id": action,
            "source": "compiled_ir_do_node_or_action_event",
            "input_slot_count": null,
            "output_slot_count": null,
            "max_input_bytes": null,
            "max_output_bytes": null,
            "timeout_ms": null,
            "idempotency": "unknown_not_embedded",
            "side_effect": "unknown_not_embedded",
            "retry_safety": "unknown_not_embedded",
            "required_capabilities": []
        }
    })
}

fn trace_ring_snapshot() -> Value {
    serde_json::json!({
        "available": false,
        "reason": "TraceRing is volatile in-memory runtime state; this packet does not fabricate a durable trace snapshot",
        "fabricated": false,
        "events": []
    })
}

fn run_status_from_events(events: &[vb_storage::JournalEvent]) -> RunStatus {
    match events.last() {
        Some(vb_storage::JournalEvent::RunFinished { .. }) => RunStatus::Finished,
        Some(vb_storage::JournalEvent::RunFailedEvent { .. }) => RunStatus::Failed,
        Some(vb_storage::JournalEvent::RunCancelled { .. }) => RunStatus::Cancelled,
        _ => RunStatus::Running,
    }
}

pub(crate) fn suggested_ai_commands(
    run_id: &str,
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String> {
    let db_arg = db.display();
    let base = vec![
        format!("velvet-ballastics inspect {run_id} --db {db_arg} --json"),
        format!("velvet-ballastics events {run_id} --db {db_arg} --json"),
    ];
    match status {
        RunStatus::Failed | RunStatus::Cancelled => base
            .into_iter()
            .chain([
                format!("velvet-ballastics incident {run_id} --db {db_arg} --json"),
                format!("velvet-ballastics retry {run_id} --db {db_arg} --json"),
            ])
            .collect(),
        RunStatus::Running => base
            .into_iter()
            .chain([
                format!("velvet-ballastics trace {run_id} --db {db_arg} --json"),
                format!("velvet-ballastics resume {run_id} --db {db_arg} --json"),
            ])
            .collect(),
        RunStatus::Finished => base
            .into_iter()
            .chain([format!(
                "velvet-ballastics replay {run_id} --db {db_arg} --json"
            )])
            .collect(),
    }
}

fn node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        vb_core::workflow::CompiledNodeKind::Copy { .. } => "Copy",
        vb_core::workflow::CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        vb_core::workflow::CompiledNodeKind::BuildObject { .. } => "BuildObject",
        vb_core::workflow::CompiledNodeKind::BuildList { .. } => "BuildList",
        vb_core::workflow::CompiledNodeKind::Do { .. } => "Do",
        vb_core::workflow::CompiledNodeKind::Choose { .. } => "Choose",
        vb_core::workflow::CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        vb_core::workflow::CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        vb_core::workflow::CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        vb_core::workflow::CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        vb_core::workflow::CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        vb_core::workflow::CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        vb_core::workflow::CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        vb_core::workflow::CompiledNodeKind::CollectStart { .. } => "CollectStart",
        vb_core::workflow::CompiledNodeKind::CollectPage { .. } => "CollectPage",
        vb_core::workflow::CompiledNodeKind::CollectNext { .. } => "CollectNext",
        vb_core::workflow::CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        vb_core::workflow::CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        vb_core::workflow::CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        vb_core::workflow::CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        vb_core::workflow::CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        vb_core::workflow::CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        vb_core::workflow::CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        vb_core::workflow::CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        vb_core::workflow::CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        vb_core::workflow::CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        vb_core::workflow::CompiledNodeKind::Ask { .. } => "Ask",
        vb_core::workflow::CompiledNodeKind::AskResume { .. } => "AskResume",
        vb_core::workflow::CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
    }
}

fn event_to_json(event: &vb_storage::JournalEvent) -> Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({"seq": seq.get(), "type": "RunAccepted", "run": run.get(), "workflow": format!("{:?}", workflow)})
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "StepStarted", "step": step.get()})
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "StepSucceeded", "step": step.get(), "output": output.get()})
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionScheduled", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionCompleted", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({"seq": seq.get(), "type": "ActionFailed", "step": step.get(), "action": action.get()})
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "SlotWritten", "slot": slot.get()})
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "WaitScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "AskScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "AskAnswered", "step": step.get()})
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RetryScheduled", "step": step.get()})
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunCancelled"})
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunFinished", "result": result.get()})
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"seq": seq.get(), "type": "RunFailed"})
        }
    }
}

fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
        .is_err()
    {}
}

fn json_out(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            if let Ok(text) = serde_json::to_string(value) {
                match write_stdout_line(format_args!("{text}")) {
                    Ok(()) | Err(_) => {}
                }
            }
        }
        OutputFormat::Text => {
            if let Ok(text) = serde_json::to_string_pretty(value) {
                match write_stdout_line(format_args!("{text}")) {
                    Ok(()) | Err(_) => {}
                }
            }
        }
    }
}

fn json_error(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            if let Ok(text) = serde_json::to_string(value) {
                write_stderr_line(format_args!("{text}"));
            }
        }
        OutputFormat::Text => write_stderr_line(format_args!("{value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::latest_snapshot_from_events;
    use vb_storage::{EventSeq, JournalError, JournalEvent};

    #[test]
    fn ai_context_latest_snapshot_from_events_propagates_snapshot_lookup_error()
    -> Result<(), String> {
        let run = vb_core::RunId::new(9);
        let events = [JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([1; 32]),
        }];

        let result = latest_snapshot_from_events(&events, |_| Err(JournalError::WriteLockPoisoned));

        match result {
            Err(JournalError::WriteLockPoisoned) => Ok(()),
            Err(e) => Err(format!("expected WriteLockPoisoned, got {e:?}")),
            Ok(v) => Err(format!("expected Err(WriteLockPoisoned), got Ok({v:?})")),
        }
    }
}
