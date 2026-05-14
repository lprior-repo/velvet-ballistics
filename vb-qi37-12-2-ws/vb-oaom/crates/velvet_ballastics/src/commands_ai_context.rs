//! AI context CLI command.
//!
//! Emits a bounded, redacted AI context packet for a specific run.

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

/// Handle the `ai-context` CLI subcommand.
/// Emits a bounded, redacted AI context packet for the given run.
pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    // Step 1: Parse run ID
    let run = match parse_run_id(run_id) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Step 2: Open journal
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            return report_storage_open_error(&e, db, output);
        }
    };

    // Step 3: Load run header to verify run exists
    let _header = match journal.run_header(run) {
        Ok(Some(h)) => h,
        Ok(None) => {
            return report_run_not_found(run_id, output);
        }
        Err(e) => {
            return report_journal_read_error("run header", run_id, &e, output);
        }
    };

    // Step 4: Load events for the run
    let events = match journal.events_for_run(run) {
        Ok(e) => e,
        Err(e) => {
            return report_journal_read_error("reading events", run_id, &e, output);
        }
    };

    // Step 5: Check PRE-003 - run must have at least one event
    if events.is_empty() {
        return report_run_not_found(run_id, output);
    }

    // Step 6: Determine run status
    let status = run_status_from_events(&events);

    // Step 7: Load latest snapshot
    let snapshot_result = latest_snapshot_from_events(&events, |seq| {
        journal.snapshot(run, seq)
    });

    let snapshot = match snapshot_result {
        Ok(s) => s,
        Err(e) => {
            return report_journal_read_error("reading snapshot", run_id, &e, output);
        }
    };

    // Step 8: Get workflow digest from events
    let workflow_digest = workflow_digest_from_events(&events);

    // Step 9: Build AI context packet
    let schema_version = Value::String("1".to_string());
    let kind = Value::String("AiContextPacket".to_string());
    let run_id_value = Value::Number(serde_json::Number::from(run.get()));

    // Build workflow summary
    let workflow = ai_workflow_summary(&journal, workflow_digest);

    // Build journal event trail
    let journal_event_trail = ai_journal_events(&events, snapshot.as_ref());

    // Build action contracts
    let action_contracts = ai_action_contracts(&events, workflow.get("referenced_actions"));

    // Build trace ring snapshot
    let trace_ring_snapshot = if let Some(ref snap) = snapshot {
        trace_ring_snapshot_json(snap)
    } else {
        Value::Null
    };

    // Build suggested next CLI commands
    let suggested_next_cli_commands: Vec<Value> = suggested_ai_commands(run_id, db, status)
        .into_iter()
        .map(Value::String)
        .collect();

    // Assemble packet
    let mut packet = Map::new();
    packet.insert("schema_version".to_string(), schema_version);
    packet.insert("kind".to_string(), kind);
    packet.insert("run_id".to_string(), run_id_value);
    packet.insert("workflow".to_string(), workflow);
    packet.insert("journal_event_trail".to_string(), Value::Array(journal_event_trail));
    packet.insert("action_contracts".to_string(), action_contracts);
    packet.insert("trace_ring_snapshot".to_string(), trace_ring_snapshot);
    packet.insert(
        "suggested_next_cli_commands".to_string(),
        Value::Array(suggested_next_cli_commands),
    );

    json_out(&Value::Object(packet), output);
    ExitCode::from(CliExitCode::Success)
}

pub(crate) fn redacted_slot_value(
    slot: vb_core::SlotIdx,
    value: Option<&Vec<u8>>,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    // Check if slot is secret or derived first (POST-003)
    if slot_is_secret_or_derived(slot, snapshot) {
        return Value::String("[REDACTED]".to_string());
    }

    // Handle None value case
    let Some(bytes) = value else {
        return Value::Null;
    };

    // Handle empty bytes case (taint=0 and empty vec = Null)
    if bytes.is_empty() {
        return Value::Null;
    }

    // Try to decode as SlotValue
    match postcard::from_bytes::<vb_core::SlotValue>(bytes) {
        Ok(slot_value) => Value::String(slot_value.to_string()),
        Err(_) => Value::String("[UNDECODED]".to_string()),
    }
}

pub(crate) fn suggested_ai_commands(
    run_id: &str,
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String> {
    let db_str = db.to_string_lossy();
    let base = vec![
        format!("velvet-ballastics inspect {run_id} --db {db_str} --json"),
        format!("velvet-ballastics events {run_id} --db {db_str} --json"),
    ];

    let mut commands = base;

    match status {
        RunStatus::Failed | RunStatus::Cancelled => {
            commands.push(format!(
                "velvet-ballastics incident {run_id} --db {db_str} --json"
            ));
            commands.push(format!("velvet-ballastics retry {run_id} --db {db_str} --json"));
        }
        RunStatus::Running => {
            commands.push(format!("velvet-ballastics trace {run_id} --db {db_str} --json"));
            commands.push(format!("velvet-ballastics resume {run_id} --db {db_str} --json"));
        }
        RunStatus::Finished => {
            commands.push(format!("velvet-ballastics replay {run_id} --db {db_str} --json"));
        }
    }

    // INV-002: bounded to max 4 commands
    commands.truncate(4);
    commands
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    // PRE-001: must be valid u64 decimal string
    // Reject empty string and strings with leading whitespace
    if raw.is_empty() || raw.starts_with(' ') || raw.starts_with('\t') {
        return Err(ExitCode::from(CliExitCode::ValidationFailed));
    }

    let parsed: Result<u64, _> = raw.parse();
    match parsed {
        Ok(v) => Ok(vb_core::RunId::new(v)),
        Err(_) => Err(ExitCode::from(CliExitCode::ValidationFailed)),
    }
}

fn report_storage_open_error(
    e: &vb_storage::JournalError,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let mut err_map = Map::new();
    err_map.insert("success".to_string(), Value::Bool(false));
    err_map.insert(
        "error".to_string(),
        Value::String(format!("opening journal at {}: {e}", db.display())),
    );
    json_error(&Value::Object(err_map), output);
    ExitCode::from(CliExitCode::StorageError)
}

fn report_run_not_found(run_id: &str, output: OutputFormat) -> ExitCode {
    let mut err_map = Map::new();
    err_map.insert("success".to_string(), Value::Bool(false));
    err_map.insert("code".to_string(), Value::String("RUN_NOT_FOUND".to_string()));
    err_map.insert(
        "error".to_string(),
        Value::String(format!("run {run_id} not found or has no events")),
    );
    json_error(&Value::Object(err_map), output);
    ExitCode::from(CliExitCode::ValidationFailed)
}

fn report_journal_read_error(
    area: &str,
    run_id: &str,
    e: &vb_storage::JournalError,
    output: OutputFormat,
) -> ExitCode {
    let mut err_map = Map::new();
    err_map.insert("success".to_string(), Value::Bool(false));
    err_map.insert(
        "error".to_string(),
        Value::String(format!("error {area} for run {run_id}: {e}")),
    );
    json_error(&Value::Object(err_map), output);
    ExitCode::from(CliExitCode::StorageError)
}

fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    use std::fmt::Write;
    let bytes = digest.as_bytes();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes.iter() {
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
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
    // Walk events in reverse to find the latest snapshot
    // Events are stored in ascending sequence order, so we scan from the end
    let mut latest_seq: Option<vb_storage::EventSeq> = None;
    let mut latest_snap: Option<vb_storage::RunSnapshot> = None;

    // Scan events backwards to find the highest seq with a snapshot
    for event in events.iter().rev() {
        let seq = event.seq();
        match snapshot_at(seq) {
            Ok(Some(snap)) => {
                // Found a snapshot at this seq
                latest_seq = Some(seq);
                latest_snap = Some(snap);
                break;
            }
            Ok(None) => {
                // No snapshot at this seq, continue looking
            }
            Err(e) => {
                // Propagate error
                return Err(e);
            }
        }
    }

    Ok(latest_snap)
}

fn ai_workflow_summary(
    journal: &vb_storage::FjallJournal,
    digest: Option<vb_core::WorkflowDigest>,
) -> Value {
    let mut summary = Map::new();

    match digest {
        Some(d) => {
            summary.insert("digest".to_string(), Value::String(digest_hex(d)));

            // Try to load compiled IR
            match journal.compiled_ir(d) {
                Ok(Some(record)) => {
                    // Try to decode the compiled workflow as WorkflowParts
                    match postcard::from_bytes::<vb_core::WorkflowParts>(&record.ir) {
                        Ok(parts) => {
                            summary.insert("compiled_ir".to_string(), workflow_parts_summary(&parts));
                            summary.insert("referenced_actions".to_string(), referenced_actions_from_parts(&parts));
                        }
                        Err(_) => {
                            let mut ir_map = Map::new();
                            ir_map.insert("available".to_string(), Value::Bool(false));
                            ir_map.insert("reason".to_string(), Value::String("decode failed".to_string()));
                            summary.insert("compiled_ir".to_string(), Value::Object(ir_map));
                            summary.insert("referenced_actions".to_string(), Value::Array(vec![]));
                        }
                    }
                }
                Ok(None) => {
                    let mut ir_map = Map::new();
                    ir_map.insert("available".to_string(), Value::Bool(false));
                    summary.insert("compiled_ir".to_string(), Value::Object(ir_map));
                    summary.insert("referenced_actions".to_string(), Value::Array(vec![]));
                }
                Err(_) => {
                    let mut ir_map = Map::new();
                    ir_map.insert("available".to_string(), Value::Bool(false));
                    summary.insert("compiled_ir".to_string(), Value::Object(ir_map));
                    summary.insert("referenced_actions".to_string(), Value::Array(vec![]));
                }
            }
        }
        None => {
            summary.insert("digest".to_string(), Value::Null);
            let mut ir_map = Map::new();
            ir_map.insert("available".to_string(), Value::Bool(false));
            summary.insert("compiled_ir".to_string(), Value::Object(ir_map));
            summary.insert("referenced_actions".to_string(), Value::Array(vec![]));
        }
    }

    Value::Object(summary)
}

fn workflow_summary_from_source(
    _journal: &vb_storage::FjallJournal,
    _digest: vb_core::WorkflowDigest,
) -> Value {
    // Source workflow resolution not implemented in ai-context (compiled IR only)
    let mut summary = Map::new();
    summary.insert("digest".to_string(), Value::Null);
    let mut ir_map = Map::new();
    ir_map.insert("available".to_string(), Value::Bool(false));
    summary.insert("compiled_ir".to_string(), Value::Object(ir_map));
    summary.insert("referenced_actions".to_string(), Value::Array(vec![]));
    Value::Object(summary)
}

fn workflow_parts_summary(parts: &vb_core::WorkflowParts) -> Value {
    let mut ir_map = Map::new();
    ir_map.insert("available".to_string(), Value::Bool(true));
    ir_map.insert("name".to_string(), Value::String(parts.name.to_string()));
    ir_map.insert("node_count".to_string(), Value::Number(serde_json::Number::from(u16::try_from(parts.nodes.len()).unwrap_or(u16::MAX))));
    ir_map.insert("slot_count".to_string(), Value::Number(serde_json::Number::from(parts.slot_count)));

    // Build nodes array
    let mut nodes = Vec::new();
    for node in parts.nodes.iter() {
        nodes.push(workflow_node_json(node));
    }
    ir_map.insert("nodes".to_string(), Value::Array(nodes));

    Value::Object(ir_map)
}

fn workflow_node_json(node: &vb_core::CompiledNode) -> Value {
    let mut node_map = Map::new();
    node_map.insert("id".to_string(), Value::Number(serde_json::Number::from(node.id.get())));

    if let Some(output) = node.output {
        node_map.insert("output".to_string(), Value::Number(serde_json::Number::from(output.get())));
    }

    let kind_name = workflow_node_kind_name(&node.kind);
    node_map.insert("kind".to_string(), Value::String(kind_name.to_string()));

    // For Do nodes, extract action ID
    if let vb_core::workflow::CompiledNodeKind::Do { action, .. } = &node.kind {
        node_map.insert("action".to_string(), Value::Number(serde_json::Number::from(action.get())));
    }

    Value::Object(node_map)
}

fn referenced_actions_from_parts(parts: &vb_core::WorkflowParts) -> Value {
    let actions: Vec<Value> = referenced_actions_from_parts_list(parts)
        .into_iter()
        .map(|a| Value::Number(serde_json::Number::from(a)))
        .collect();
    Value::Array(actions)
}

fn referenced_actions_from_parts_list(parts: &vb_core::WorkflowParts) -> Vec<u32> {
    let mut actions = Vec::new();
    for node in parts.nodes.iter() {
        if let vb_core::workflow::CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_u32 = u32::from(action.get());
            if !actions.contains(&action_u32) {
                actions.push(action_u32);
            }
        }
    }
    actions
}

fn workflow_digest_from_events(events: &[vb_storage::JournalEvent]) -> Option<vb_core::WorkflowDigest> {
    // Find the first RunAccepted event which contains the workflow digest
    for event in events {
        if let vb_storage::JournalEvent::RunAccepted { workflow, .. } = event {
            return Some(*workflow);
        }
    }
    None
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
        .map(|e| ai_event_to_json(e, snapshot))
        .collect()
}

fn ai_event_to_json(event: &vb_storage::JournalEvent, snapshot: Option<&vb_storage::RunSnapshot>) -> Value {
    let mut map = Map::new();

    // Add common fields
    map.insert("seq".to_string(), Value::Number(serde_json::Number::from(event.seq().get())));

    match event {
        vb_storage::JournalEvent::RunAccepted { run, seq: _, workflow } => {
            map.insert("kind".to_string(), Value::String("RunAccepted".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("workflow".to_string(), Value::String(digest_hex(*workflow)));
        }
        vb_storage::JournalEvent::RunAdmission { run, seq: _, artifact_digest, granted_capabilities: _, policy: _ } => {
            map.insert("kind".to_string(), Value::String("RunAdmission".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("artifact_digest".to_string(), Value::String(digest_hex(*artifact_digest)));
        }
        vb_storage::JournalEvent::StepStarted { run, seq: _, step } => {
            map.insert("kind".to_string(), Value::String("StepStarted".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
        }
        vb_storage::JournalEvent::StepSucceeded { run, seq: _, step, output } => {
            map.insert("kind".to_string(), Value::String("StepSucceeded".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
            map.insert("output".to_string(), Value::Number(serde_json::Number::from(output.get())));
        }
        vb_storage::JournalEvent::ActionScheduled { run, seq: _, step, action } => {
            map.insert("kind".to_string(), Value::String("ActionScheduled".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
            map.insert("action".to_string(), Value::Number(serde_json::Number::from(action.get())));
        }
        vb_storage::JournalEvent::ActionCompletedEvent { run, seq: _, step, action } => {
            map.insert("kind".to_string(), Value::String("ActionCompleted".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
            map.insert("action".to_string(), Value::Number(serde_json::Number::from(action.get())));
        }
        vb_storage::JournalEvent::ActionFailedEvent { run, seq: _, step, action } => {
            map.insert("kind".to_string(), Value::String("ActionFailed".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
            map.insert("action".to_string(), Value::Number(serde_json::Number::from(action.get())));
        }
        vb_storage::JournalEvent::SlotWrittenEvent { run, seq: _, slot, value, extra: _ } => {
            map.insert("kind".to_string(), Value::String("SlotWritten".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("slot".to_string(), Value::Number(serde_json::Number::from(slot.get())));
            // Apply redaction to slot value
            let redacted = redacted_slot_value(*slot, value.as_ref(), snapshot);
            map.insert("value".to_string(), redacted);
        }
        vb_storage::JournalEvent::WaitScheduledEvent { run, seq: _, step } => {
            map.insert("kind".to_string(), Value::String("WaitScheduled".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
        }
        vb_storage::JournalEvent::AskScheduledEvent { run, seq: _, step } => {
            map.insert("kind".to_string(), Value::String("AskScheduled".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
        }
        vb_storage::JournalEvent::AskAnsweredEvent { run, seq: _, step } => {
            map.insert("kind".to_string(), Value::String("AskAnswered".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
        }
        vb_storage::JournalEvent::RetryScheduledEvent { run, seq: _, step } => {
            map.insert("kind".to_string(), Value::String("RetryScheduled".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("step".to_string(), Value::Number(serde_json::Number::from(step.get())));
        }
        vb_storage::JournalEvent::RunCancelled { run, seq: _ } => {
            map.insert("kind".to_string(), Value::String("RunCancelled".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
        }
        vb_storage::JournalEvent::RunFinished { run, seq: _, result } => {
            map.insert("kind".to_string(), Value::String("RunFinished".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
            map.insert("result".to_string(), Value::Number(serde_json::Number::from(result.get())));
        }
        vb_storage::JournalEvent::RunFailedEvent { run, seq: _ } => {
            map.insert("kind".to_string(), Value::String("RunFailed".to_string()));
            map.insert("run".to_string(), Value::Number(serde_json::Number::from(run.get())));
        }
    }

    Value::Object(map)
}

fn slot_is_secret_or_derived(slot: vb_core::SlotIdx, snapshot: Option<&vb_storage::RunSnapshot>) -> bool {
    let Some(snap) = snapshot else {
        return false;
    };

    let taint = &snap.taint;
    let idx = slot.as_usize();

    // Check bounds - if slot index is beyond taint table, treat as clean
    if idx >= taint.len() {
        return false;
    }

    let raw = taint[idx];
    raw == 1 || raw == 2
}

fn ai_action_contracts(
    events: &[vb_storage::JournalEvent],
    workflow_actions: Option<&Value>,
) -> Value {
    let mut action_ids: Vec<u32> = Vec::new();

    // Extract action IDs from journal events
    for event in events {
        match event {
            vb_storage::JournalEvent::ActionScheduled { action, .. } => {
                action_ids = push_unique_u32(action_ids, u32::from(action.get()));
            }
            vb_storage::JournalEvent::ActionCompletedEvent { action, .. } => {
                action_ids = push_unique_u32(action_ids, u32::from(action.get()));
            }
            vb_storage::JournalEvent::ActionFailedEvent { action, .. } => {
                action_ids = push_unique_u32(action_ids, u32::from(action.get()));
            }
            _ => {}
        }
    }

    // Add action IDs from workflow compiled IR if available
    if let Some(Value::Array(workflow_actions_arr)) = workflow_actions {
        for action_val in workflow_actions_arr {
            if let Value::Number(n) = action_val {
                if let Some(id) = n.as_u64() {
                    action_ids = push_unique_u32(action_ids, id as u32);
                }
            }
        }
    }

    // Build contract JSON for each action
    let contracts: Vec<Value> = action_ids
        .into_iter()
        .map(|action| inferred_action_contract_json(action))
        .collect();

    Value::Array(contracts)
}

fn inferred_action_contract_json(action: u32) -> Value {
    let mut map = Map::new();
    map.insert("action".to_string(), Value::Number(serde_json::Number::from(action)));
    map.insert(
        "contract_status".to_string(),
        Value::String("inferred_from_compiled_ir_and_journal".to_string()),
    );
    Value::Object(map)
}

fn trace_ring_snapshot_json(snapshot: &vb_storage::RunSnapshot) -> Value {
    let mut map = Map::new();
    map.insert("run".to_string(), Value::Number(serde_json::Number::from(snapshot.run.get())));
    map.insert("seq".to_string(), Value::Number(serde_json::Number::from(snapshot.seq.get())));
    map.insert("workflow".to_string(), Value::String(digest_hex(snapshot.workflow)));
    map.insert("slot_count".to_string(), Value::Number(serde_json::Number::from(snapshot.slots.len())));
    map.insert("taint_count".to_string(), Value::Number(serde_json::Number::from(snapshot.taint.len())));
    Value::Object(map)
}

pub(crate) fn run_status_from_events(events: &[vb_storage::JournalEvent]) -> RunStatus {
    // INV-001: read-only - no mutation
    // Scan events in order and return status based on the LAST terminal event
    let mut status = RunStatus::Running;

    for event in events {
        match event {
            vb_storage::JournalEvent::RunFinished { .. } => {
                status = RunStatus::Finished;
            }
            vb_storage::JournalEvent::RunFailedEvent { .. } => {
                status = RunStatus::Failed;
            }
            vb_storage::JournalEvent::RunCancelled { .. } => {
                status = RunStatus::Cancelled;
            }
            _ => {
                // Other events don't change terminal status
            }
        }
    }

    status
}

fn workflow_node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
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
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
    }
}

fn event_to_json(event: &vb_storage::JournalEvent) -> Value {
    ai_event_to_json(event, None)
}

fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    print!("{args}");
    io::stdout().flush()
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    eprint!("{args}");
    let _ = io::stderr().flush();
}

fn json_out(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let line = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
            let _ = write_stdout_line(format_args!("{line}\n"));
        }
        OutputFormat::Jsonl => {
            let line = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
            let _ = write_stdout_line(format_args!("{line}\n"));
        }
        OutputFormat::Text => {
            // For text format, just print the JSON in a readable way
            let line = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
            let _ = write_stdout_line(format_args!("{line}\n"));
        }
    }
}

fn json_error(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            let line = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
            let _ = write_stderr_line(format_args!("{line}\n"));
        }
        OutputFormat::Text => {
            let line = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
            let _ = write_stderr_line(format_args!("{line}\n"));
        }
    }
}

// ============================================================================
// TESTS — RED PHASE
// These tests will FAIL until the actual implementation replaces the stubs.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use vb_storage::{EventSeq, JournalEvent};
    use vb_core::WorkflowDigest;

    // ------------------------------------------------------------------
    // parse_run_id tests
    // ------------------------------------------------------------------

    #[test]
    fn parse_run_id_returns_run_id_when_valid_decimal_u64() {
        // Given: a valid decimal u64 string
        let input = "12345";

        // When: parse_run_id is called
        // Then: it returns Ok(RunId::new(12345))
        // RED PHASE: This will panic with todo!()
        let result = parse_run_id(input);
        assert!(result.is_ok(), "parse_run_id should succeed for valid u64");
        assert_eq!(result.unwrap().get(), 12345);
    }

    #[test]
    fn parse_run_id_returns_validation_failed_when_input_is_non_numeric() {
        // Given: a non-numeric string
        let input = "not-a-number";

        // When: parse_run_id is called
        // Then: it returns Err(CliExitCode::ValidationFailed)
        // RED PHASE: This will panic with todo!()
        let result = parse_run_id(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExitCode::from(CliExitCode::ValidationFailed));
    }

    #[test]
    fn parse_run_id_returns_validation_failed_when_input_is_out_of_range() {
        // Given: a string that exceeds u64::MAX
        let input = "99999999999999999999";

        // When: parse_run_id is called
        // Then: it returns Err(CliExitCode::ValidationFailed)
        // RED PHASE: This will panic with todo!()
        let result = parse_run_id(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExitCode::from(CliExitCode::ValidationFailed));
    }

    #[test]
    fn parse_run_id_returns_validation_failed_when_input_is_empty() {
        // Given: an empty string
        let input = "";

        // When: parse_run_id is called
        // Then: it returns Err(CliExitCode::ValidationFailed)
        // RED PHASE: This will panic with todo!()
        let result = parse_run_id(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExitCode::from(CliExitCode::ValidationFailed));
    }

    #[test]
    fn parse_run_id_returns_validation_failed_when_input_has_whitespace_prefix() {
        // Given: a string with whitespace prefix
        let input = " 12345";

        // When: parse_run_id is called
        // Then: it returns Err(CliExitCode::ValidationFailed)
        // RED PHASE: This will panic with todo!()
        let result = parse_run_id(input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // redacted_slot_value tests
    // ------------------------------------------------------------------

    #[test]
    fn redacted_slot_value_returns_redacted_when_slot_is_secret() {
        // Given: a slot with taint=1 (Secret)
        let slot = vb_core::SlotIdx::new(0);
        let value = vec![1, 2, 3];
        // Construct a snapshot with taint[0] = 1 (Secret)
        let taint = vec![1]; // taint at slot 0 is Secret
        let snapshot = vb_storage::RunSnapshot {
            run: vb_core::RunId::new(1),
            seq: vb_storage::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
            slots: vec![],
            taint,
        };

        let result = redacted_slot_value(slot, Some(&value), Some(&snapshot));
        assert_eq!(result, Value::String("[REDACTED]".to_string()));
    }

    #[test]
    fn redacted_slot_value_returns_redacted_when_slot_is_derived_from_secret() {
        // Given: a slot with taint=2 (DerivedFromSecret)
        let slot = vb_core::SlotIdx::new(5);
        let value = vec![9, 8, 7];
        // Construct a snapshot with taint[5] = 2 (DerivedFromSecret)
        let mut taint = vec![0; 6]; // indices 0-5
        taint[5] = 2; // taint at slot 5 is DerivedFromSecret
        let snapshot = vb_storage::RunSnapshot {
            run: vb_core::RunId::new(1),
            seq: vb_storage::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
            slots: vec![],
            taint,
        };

        let result = redacted_slot_value(slot, Some(&value), Some(&snapshot));
        assert_eq!(result, Value::String("[REDACTED]".to_string()));
    }

    #[test]
    fn redacted_slot_value_returns_decoded_string_when_slot_is_clean() {
        // Given: a clean slot (taint=0) with valid SlotValue bytes
        let slot = vb_core::SlotIdx::new(0);
        // Encode a valid SlotValue
        let valid_bytes = postcard::to_allocvec(&vb_core::SlotValue::I64(42)).unwrap();

        // RED PHASE: This will panic with todo!()
        let result = redacted_slot_value(slot, Some(&valid_bytes), None);
        // Should return the decoded string representation, not raw bytes
        assert_eq!(result, Value::String("42".to_string()));
    }

    #[test]
    fn redacted_slot_value_returns_undecoded_when_bytes_fail_postcard_decode() {
        // Given: invalid bytes that can't be decoded as SlotValue
        let slot = vb_core::SlotIdx::new(0);
        let invalid_bytes = vec![0xff, 0xfe, 0x00];

        // RED PHASE: This will panic with todo!()
        let result = redacted_slot_value(slot, Some(&invalid_bytes), None);
        assert_eq!(result, Value::String("[UNDECODED]".to_string()));
    }

    #[test]
    fn redacted_slot_value_returns_null_when_value_is_none_and_slot_is_clean() {
        // Given: no value and a clean slot
        let slot = vb_core::SlotIdx::new(0);

        // RED PHASE: This will panic with todo!()
        let result = redacted_slot_value(slot, None, None);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn redacted_slot_value_returns_null_when_taint_is_zero_and_value_is_empty_vec() {
        // Given: empty vec and taint=0
        let slot = vb_core::SlotIdx::new(0);
        let empty_bytes = vec![];

        // RED PHASE: This will panic with todo!()
        let result = redacted_slot_value(slot, Some(&empty_bytes), None);
        assert_eq!(result, Value::Null);
    }

    // ------------------------------------------------------------------
    // slot_is_secret_or_derived tests
    // ------------------------------------------------------------------

    #[test]
    fn slot_is_secret_or_derived_returns_true_when_taint_entry_is_1() {
        // Given: a snapshot with taint[0] = 1
        let slot = vb_core::SlotIdx::new(0);
        let taint = vec![1]; // taint at slot 0 is Secret
        let snapshot = vb_storage::RunSnapshot {
            run: vb_core::RunId::new(1),
            seq: vb_storage::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
            slots: vec![],
            taint,
        };

        let result = slot_is_secret_or_derived(slot, Some(&snapshot));
        assert!(result);
    }

    #[test]
    fn slot_is_secret_or_derived_returns_true_when_taint_entry_is_2() {
        let slot = vb_core::SlotIdx::new(5);
        let mut taint = vec![0; 6]; // indices 0-5
        taint[5] = 2; // taint at slot 5 is DerivedFromSecret
        let snapshot = vb_storage::RunSnapshot {
            run: vb_core::RunId::new(1),
            seq: vb_storage::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
            slots: vec![],
            taint,
        };

        let result = slot_is_secret_or_derived(slot, Some(&snapshot));
        assert!(result);
    }

    #[test]
    fn slot_is_secret_or_derived_returns_false_when_taint_entry_is_0() {
        let slot = vb_core::SlotIdx::new(0);
        // RED PHASE: This will panic with todo!()
        let result = slot_is_secret_or_derived(slot, None);
        assert!(!result);
    }

    #[test]
    fn slot_is_secret_or_derived_returns_false_when_snapshot_is_none() {
        let slot = vb_core::SlotIdx::new(0);
        // RED PHASE: This will panic with todo!()
        let result = slot_is_secret_or_derived(slot, None);
        assert!(!result);
    }

    // ------------------------------------------------------------------
    // suggested_ai_commands tests
    // ------------------------------------------------------------------

    #[test]
    fn suggested_ai_commands_returns_inspect_and_events_for_all_statuses() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let statuses = [
            RunStatus::Running,
            RunStatus::Finished,
            RunStatus::Failed,
            RunStatus::Cancelled,
        ];

        for status in statuses {
            // RED PHASE: This will panic with todo!()
            let result = suggested_ai_commands(run_id, db, status);
            assert!(result.len() >= 2, "Should have at least inspect and events");
            assert!(result[0].contains("velvet-ballastics inspect"));
            assert!(result[1].contains("velvet-ballastics events"));
        }
    }

    #[test]
    fn suggested_ai_commands_adds_incident_and_retry_when_status_is_failed() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let status = RunStatus::Failed;

        // RED PHASE: This will panic with todo!()
        let result = suggested_ai_commands(run_id, db, status);
        assert_eq!(result.len(), 4);
        assert!(result[2].contains("velvet-ballastics incident"));
        assert!(result[3].contains("velvet-ballastics retry"));
    }

    #[test]
    fn suggested_ai_commands_adds_incident_and_retry_when_status_is_cancelled() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let status = RunStatus::Cancelled;

        // RED PHASE: This will panic with todo!()
        let result = suggested_ai_commands(run_id, db, status);
        assert_eq!(result.len(), 4);
        assert!(result[2].contains("velvet-ballastics incident"));
        assert!(result[3].contains("velvet-ballastics retry"));
    }

    #[test]
    fn suggested_ai_commands_adds_trace_and_resume_when_status_is_running() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let status = RunStatus::Running;

        // RED PHASE: This will panic with todo!()
        let result = suggested_ai_commands(run_id, db, status);
        assert_eq!(result.len(), 4);
        assert!(result[2].contains("velvet-ballastics trace"));
        assert!(result[3].contains("velvet-ballastics resume"));
    }

    #[test]
    fn suggested_ai_commands_adds_replay_when_status_is_finished() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let status = RunStatus::Finished;

        // RED PHASE: This will panic with todo!()
        let result = suggested_ai_commands(run_id, db, status);
        assert_eq!(result.len(), 3);
        assert!(result[2].contains("velvet-ballastics replay"));
    }

    #[test]
    fn suggested_ai_commands_returns_max_4_commands() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let statuses = [
            RunStatus::Running,
            RunStatus::Finished,
            RunStatus::Failed,
            RunStatus::Cancelled,
        ];

        for status in statuses {
            // RED PHASE: This will panic with todo!()
            let result = suggested_ai_commands(run_id, db, status);
            assert!(result.len() <= 4, "Max 4 commands allowed");
        }
    }

    #[test]
    fn suggested_ai_commands_all_commands_start_with_velvet_ballastics() {
        let run_id = "1";
        let db = std::path::Path::new("/tmp/db");
        let status = RunStatus::Running;

        // RED PHASE: This will panic with todo!()
        let result = suggested_ai_commands(run_id, db, status);
        for cmd in &result {
            assert!(
                cmd.starts_with("velvet-ballastics "),
                "Command should start with 'velvet-ballastics ': {}",
                cmd
            );
        }
    }

    // ------------------------------------------------------------------
    // run_status_from_events tests
    // ------------------------------------------------------------------

    #[test]
    fn run_status_from_events_returns_finished_when_last_event_is_run_finished() {
        let run = vb_core::RunId::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(1),
                result: vb_core::SlotIdx::ZERO,
            },
        ];

        // RED PHASE: This will panic with todo!()
        let result = run_status_from_events(&events);
        assert_eq!(result, RunStatus::Finished);
    }

    #[test]
    fn run_status_from_events_returns_failed_when_last_event_is_run_failed() {
        let run = vb_core::RunId::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(1),
            },
        ];

        // RED PHASE: This will panic with todo!()
        let result = run_status_from_events(&events);
        assert_eq!(result, RunStatus::Failed);
    }

    #[test]
    fn run_status_from_events_returns_cancelled_when_last_event_is_run_cancelled() {
        let run = vb_core::RunId::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
            },
        ];

        // RED PHASE: This will panic with todo!()
        let result = run_status_from_events(&events);
        assert_eq!(result, RunStatus::Cancelled);
    }

    #[test]
    fn run_status_from_events_returns_running_when_events_list_is_empty() {
        let events: Vec<JournalEvent> = vec![];

        // RED PHASE: This will panic with todo!()
        let result = run_status_from_events(&events);
        assert_eq!(result, RunStatus::Running);
    }

    // ------------------------------------------------------------------
    // report_run_not_found tests
    // ------------------------------------------------------------------

    #[test]
    fn report_run_not_found_outputs_json_with_code_run_not_found() {
        let run_id = "999";
        let output = OutputFormat::Json;

        // RED PHASE: This will panic with todo!()
        let exit_code = report_run_not_found(run_id, output);

        // Should return ValidationFailed exit code
        assert_eq!(exit_code, ExitCode::from(CliExitCode::ValidationFailed));
    }

    // ------------------------------------------------------------------
    // latest_snapshot_from_events tests
    // ------------------------------------------------------------------

    #[test]
    fn latest_snapshot_from_events_propagates_snapshot_lookup_error() {
        let run = vb_core::RunId::new(9);
        let events = vec![JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        }];

        // RED PHASE: This will panic with todo!()
        let result =
            latest_snapshot_from_events(&events, |_| Err(vb_storage::JournalError::KeyCapacity));

        assert!(result.is_err());
    }
}

// ============================================================================
// PROPTEST INVARIANTS
// ============================================================================

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    // Property-based test: parse_run_id either succeeds or returns ValidationFailed
    proptest! {
        #[test]
        fn parse_run_id_either_succeeds_or_validation_failed(input: String) {
            let result = parse_run_id(&input);

            // Then: either it succeeds (valid u64) or returns ValidationFailed
            if result.is_ok() {
                let rid = result.unwrap();
                prop_assert!(rid.get() <= u64::MAX);
            } else {
                prop_assert_eq!(result.unwrap_err(), ExitCode::from(CliExitCode::ValidationFailed));
            }
        }
    }

    // Property-based test: suggested_ai_commands length is bounded by max 4
    proptest! {
        #[test]
        fn suggested_ai_commands_length_bounded(
            run_id: String,
            status: u8,
        ) {
            let status = match status % 4 {
                0 => RunStatus::Running,
                1 => RunStatus::Finished,
                2 => RunStatus::Failed,
                _ => RunStatus::Cancelled,
            };
            let db = std::path::Path::new("/tmp/test");

            let result = suggested_ai_commands(&run_id, db, status);

            prop_assert!(
                result.len() <= 4,
                "suggested commands length {} exceeds max 4",
                result.len()
            );
        }
    }

    // Property-based test: suggested_ai_commands all start with velvet-ballastics
    proptest! {
        #[test]
        fn suggested_ai_commands_all_start_with_velvet_ballastics(
            run_id: String,
            status: u8,
        ) {
            let status = match status % 4 {
                0 => RunStatus::Running,
                1 => RunStatus::Finished,
                2 => RunStatus::Failed,
                _ => RunStatus::Cancelled,
            };
            let db = std::path::Path::new("/tmp/test");

            let result = suggested_ai_commands(&run_id, db, status);

            for cmd in &result {
                prop_assert!(
                    cmd.starts_with("velvet-ballastics "),
                    "Command should start with 'velvet-ballastics ': {}",
                    cmd
                );
            }
        }
    }
}
