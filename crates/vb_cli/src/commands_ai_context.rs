//! AI context CLI command.
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::{Map, Value};

use crate::args::OutputFormat;
use crate::cli_envelope;
use crate::exit_code::CliExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    Running,
    Finished,
    Failed,
    Cancelled,
}

pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
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
    let payload = serde_json::json!({
        "run_id": rid.get(),
        "workflow": workflow,
        "journal_event_trail": ai_journal_events(&events, latest_snapshot.as_ref()),
        "action_contracts": ai_action_contracts(&events, workflow.get("referenced_actions")),
        "trace_ring_snapshot": trace_ring_snapshot(),
        "suggested_next_cli_commands": suggested_ai_commands(run_id, db, status),
    });
    let envelope =
        cli_envelope::serialize_with_version(&payload, cli_envelope::Kind::AiContextPacket);
    match crate::json_out(&envelope, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            write_stderr_line(format_args!("output failed: {error}"));
            CliExitCode::StorageError.into()
        }
    }
}

fn parse_run_id(raw: &str, output: OutputFormat) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(0) => {
            write_run_id_error(raw, "run_id must be non-zero", output);
            Err(CliExitCode::ValidationFailed.into())
        }
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            write_run_id_error(raw, &e.to_string(), output);
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

fn write_run_id_error(raw: &str, reason: &str, output: OutputFormat) {
    let message = format!("invalid run_id '{raw}': {reason}");
    if output == OutputFormat::Text {
        write_stderr_line(format_args!("{message}"));
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
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
    match decode_compiled_workflow_from_ir(&record.ir) {
        Ok(compiled) => compiled_workflow_summary(digest, &compiled),
        Err(_) => serde_json::json!({
            "digest": digest_hex(digest),
            "compiled_ir": {"available": false, "reason": "compiled IR decode failed"},
            "source_included": false,
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeCompiledWorkflowError {
    DirectWorkflowPartsDecode,
    DirectWorkflowCompile,
    AcceptedArtifactDecode,
    AcceptedArtifactWorkflowPartsDecode,
    AcceptedArtifactWorkflowCompile,
}

fn decode_compiled_workflow_from_ir(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    decode_direct_compiled_workflow(ir).or_else(|_| decode_accepted_artifact_workflow(ir))
}

fn decode_direct_compiled_workflow(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    let parts = postcard::from_bytes::<vb_core::WorkflowParts>(ir)
        .map_err(|_| DecodeCompiledWorkflowError::DirectWorkflowPartsDecode)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| DecodeCompiledWorkflowError::DirectWorkflowCompile)
}

fn decode_accepted_artifact_workflow(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    let artifact = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(ir)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactDecode)?;
    let parts = postcard::from_bytes::<vb_core::WorkflowParts>(&artifact.ir)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactWorkflowPartsDecode)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactWorkflowCompile)
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
        .filter_map(|value| match value.as_u64() {
            None => None,
            Some(raw) => match u32::try_from(raw) {
                Ok(id) => Some(id),
                Err(_) => {
                    // u64 value does not fit in u32; drop it gracefully.
                    None
                }
            },
        });
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
        format!("velvet-ballistics inspect {run_id} --db {db_arg} --emit yaml"),
        format!("velvet-ballistics events {run_id} --db {db_arg} --emit yaml"),
    ];
    match status {
        RunStatus::Failed | RunStatus::Cancelled => base
            .into_iter()
            .chain([
                format!("velvet-ballistics incident {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics retry {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Running => base
            .into_iter()
            .chain([
                format!("velvet-ballistics trace {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics resume {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Finished => base
            .into_iter()
            .chain([format!(
                "velvet-ballistics replay {run_id} --db {db_arg} --emit yaml"
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
        _ => "Unknown",
    }
}

fn event_to_json(event: &vb_storage::JournalEvent) -> Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({"seq": seq.get(), "type": "RunAccepted", "run": run.get(), "workflow": format!("{:?}", workflow)})
        }
        vb_storage::JournalEvent::RunAdmission {
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => serde_json::json!({
            "seq": seq.get(),
            "type": "RunAdmission",
            "artifact_digest": format!("{artifact_digest:?}"),
            "granted_capabilities": format!("{granted_capabilities:?}"),
            "policy": format!("{policy:?}")
        }),
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
        vb_storage::JournalEvent::RunResumed {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({"type": "RunResumed", "run": run.get(), "timestamp": timestamp.to_rfc3339()})
        }
        vb_storage::JournalEvent::RunRetried {
            run,
            seq: _,
            timestamp,
        } => {
            serde_json::json!({"type": "RunRetried", "run": run.get(), "timestamp": timestamp.to_rfc3339()})
        }
        vb_storage::JournalEvent::RunAnswered {
            run,
            seq: _,
            slot_idx,
            answer,
            timestamp,
        } => {
            serde_json::json!({"type": "RunAnswered", "run": run.get(), "slot_idx": slot_idx.get(), "answer": format!("{:?}", answer), "timestamp": timestamp.to_rfc3339()})
        }
        _ => serde_json::json!({"type": "Unknown"}),
    }
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {
        write_stderr_best_effort(format_args!("stderr write failed: {error}"));
    }
}

fn write_stderr_best_effort(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_write_error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {}
}

fn json_error(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            if let Err(error) = crate::app_impl::write_structured_stderr(value, format) {
                write_stderr_best_effort(format_args!("stderr write failed: {error}"));
            }
        }
        OutputFormat::Text => write_stderr_line(format_args!("{value}")),
    }
}

#[cfg(test)]
#[path = "commands_ai_context/tests.rs"]
mod tests;
