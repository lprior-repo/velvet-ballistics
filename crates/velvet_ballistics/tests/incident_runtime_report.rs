#![forbid(unsafe_code)]

use std::{path::Path, process::Command};

use vb_core::{ActionTicket, RunId, WorkflowDigest};
use vb_storage::{FjallJournal, JournalEvent};

#[path = "../src/cli_postcard.rs"]
mod cli_postcard;
#[path = "incident_runtime_report/text.rs"]
mod incident_text;
#[path = "incident_runtime_report/runtime_fixture.rs"]
mod runtime_fixture;

fn ticket_json(ticket: ActionTicket) -> serde_json::Value {
    serde_json::json!({
        "run": ticket.run.get(),
        "step": ticket.step.get(),
        "seq": ticket.seq.get(),
        "action": ticket.action.get(),
        "run_attempt": ticket.run_attempt,
        "attempt": ticket.attempt,
        "idempotency_key": ticket.idempotency_key.to_string(),
        "capacity": ticket.capacity,
        "fencing_token": ticket.fencing_token,
    })
}

fn digest_json(digest: WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn action_row(
    event: &JournalEvent,
    ticket: ActionTicket,
    certainty: &str,
    digest: Option<WorkflowDigest>,
    completion_durable: bool,
) -> serde_json::Value {
    serde_json::json!({
        "run_id": event.run_id().get(),
        "seq": event.seq().get(),
        "attempt": event.attempt(),
        "step": ticket.step.get(),
        "action": ticket.action.get(),
        "certainty": certainty,
        "ticket": ticket_json(ticket),
        "action_abi_digest": digest.map_or_else(|| serde_json::Value::Null, |value| serde_json::Value::String(digest_json(value))),
        "scheduled_durable": true,
        "completion_durable": completion_durable.then_some(serde_json::Value::Bool(true)).map_or_else(|| serde_json::Value::Null, std::convert::identity),
        "external_effect": "uncertain",
        "safe_to_retry": serde_json::Value::Null,
        "reason": "retry_contract_not_loaded",
    })
}

fn expected_payload(run: RunId, events: &[JournalEvent]) -> serde_json::Value {
    let failed_at_step = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::StepStarted { step, .. } => Some(step.get()),
            _ => None,
        })
        .last();
    let rows = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket,
                action_abi_digest,
                ..
            } => Some(action_row(
                event,
                *ticket,
                "scheduled",
                Some(*action_abi_digest),
                has_completion(events, *ticket),
            )),
            JournalEvent::ActionCompletedEnvelope {
                ticket,
                action_abi_digest,
                ..
            } => Some(action_row(
                event,
                *ticket,
                "confirmed",
                Some(*action_abi_digest),
                true,
            )),
            JournalEvent::ActionAbandoned { ticket, .. } => {
                Some(action_row(event, *ticket, "abandoned", None, false))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let tail = events
        .iter()
        .map(|event| {
            serde_json::json!({
                "seq": event.seq().get(),
                "kind": format!("{:?}", event.record_kind()),
                "run_id": event.run_id().get(),
                "attempt": event.attempt(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "run_id": run.get().to_string(),
        "failure_code": "RunCancelled",
        "failed_at_step": failed_at_step,
        "side_effects": rows,
        "repair_hints": [
            "run was cancelled; check if cancellation was intentional",
            "review recorded action outcomes for partial cleanup needs",
        ],
        "journal_tail": tail,
    })
}

fn has_completion(events: &[JournalEvent], ticket: ActionTicket) -> bool {
    events.iter().any(|event| {
        matches!(event,
        JournalEvent::ActionCompletedEnvelope { ticket: found, .. } if *found == ticket)
    })
}

fn raw_event_bytes(
    path: &Path,
    run: RunId,
    events: &[JournalEvent],
) -> Result<Vec<Vec<u8>>, String> {
    let journal = FjallJournal::open(path, None).map_err(|error| error.to_string())?;
    events
        .iter()
        .map(|event| {
            journal
                .get_event_bytes(run, event.seq())
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("missing retained bytes for sequence {}", event.seq().get()))
        })
        .collect()
}

fn retained_events(path: &Path, run: RunId) -> Result<Vec<JournalEvent>, String> {
    FjallJournal::open(path, None)
        .map_err(|error| error.to_string())?
        .events_for_run_full(run)
        .map_err(|error| error.to_string())
}

fn run_cli(db: &Path, run: RunId, format: &str) -> Result<std::process::Output, String> {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("incident")
        .arg(run.get().to_string())
        .arg("--db")
        .arg(db)
        .args(["--emit", format])
        .output()
        .map_err(|error| error.to_string())
}

#[test]
fn runtime_issued_completion_and_abandonment_are_exact_in_all_public_formats() -> Result<(), String>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/incident-runtime-report");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let directory = tempfile::Builder::new()
        .prefix("vb-incident-runtime-")
        .tempdir_in(root)
        .map_err(|error| error.to_string())?;
    let db = directory.path().join("journal");
    for (run, complete) in [(RunId::new(420), true), (RunId::new(421), false)] {
        let events = runtime_fixture::create_incident_events(&db, run, complete)?;
        if complete {
            let tail = FjallJournal::open(&db, None)
                .map_err(|error| error.to_string())?
                .events_for_run(run)
                .map_err(|error| error.to_string())?;
            assert!(tail.len() < events.len());
            assert!(!tail.iter().any(|event| {
                matches!(
                    event,
                    JournalEvent::ActionScheduledTicket { .. }
                        | JournalEvent::ActionCompletedEnvelope { .. }
                )
            }));
        }
        let retained_before = retained_events(&db, run)?;
        assert_eq!(events, retained_before);
        let bytes_before = raw_event_bytes(&db, run, &retained_before)?;
        let expected = expected_payload(run, &events);
        assert!(!expected.to_string().contains("987654321"));

        for format in ["text", "yaml", "postcard"] {
            let output = run_cli(&db, run, format)?;
            assert_eq!(
                output.status.code(),
                Some(0),
                "incident {format} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.stderr.is_empty(),
                "incident {format} stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            match format {
                "text" => assert_eq!(
                    String::from_utf8_lossy(&output.stdout),
                    incident_text::text_report(&expected)
                ),
                "yaml" => {
                    let yaml =
                        std::str::from_utf8(&output.stdout).map_err(|error| error.to_string())?;
                    let actual: serde_json::Value =
                        serde_saphyr::from_str(yaml).map_err(|error| error.to_string())?;
                    assert_eq!(actual, expected);
                }
                "postcard" => {
                    let (header, actual) = cli_postcard::decode_postcard_json(&output.stdout)
                        .map_err(|error| error.to_string())?;
                    assert_eq!(header.magic, cli_postcard::CLI_MAGIC);
                    assert_eq!(header.schema_version, cli_postcard::CLI_SCHEMA_VERSION);
                    assert_eq!(header.kind, cli_postcard::CLI_POSTCARD_KIND);
                    assert_eq!(actual, expected);
                }
                _ => return Err(String::from("unexpected output format")),
            }
            let retained_after = retained_events(&db, run)?;
            assert_eq!(retained_after.len(), retained_before.len());
            assert_eq!(retained_after, retained_before);
            assert_eq!(raw_event_bytes(&db, run, &retained_after)?, bytes_before);
        }
    }
    Ok(())
}
