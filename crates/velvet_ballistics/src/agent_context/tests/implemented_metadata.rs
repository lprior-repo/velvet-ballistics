#![cfg(test)]

use crate::agent_context::build;
use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EventStatus, OutputFormat, ParseError,
    StatusOptions, SystemStatusOptions, VerifyProfile, parse_args,
};
use crate::commands_journal::{TraceFilters, TraceStatus};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parsed(arguments: &[&str]) -> Result<Command, ParseError> {
    let arguments: Vec<_> = std::iter::once("velvet-ballistics")
        .chain(arguments.iter().copied())
        .map(OsString::from)
        .collect();
    parse_args(&arguments)
}

#[test]
fn implemented_commands_are_described_and_parse_to_exact_commands() -> TestResult {
    let context = build("test");
    let commands = context.get("commands").ok_or("commands")?;
    let cases = [
        ("help", vec!["help"], Command::Help { command: None }),
        ("version", vec!["version"], Command::Version),
        (
            "status",
            vec![
                "status",
                "--active-runs",
                "1",
                "--queue-depth",
                "2",
                "--trace-dropped",
                "3",
                "--emit",
                "yaml",
            ],
            Command::Status {
                options: StatusOptions {
                    active_runs: Some(1),
                    queue_depth: Some(2),
                    trace_dropped: Some(3),
                    emit_yaml: true,
                },
                output: OutputFormat::Yaml,
            },
        ),
        (
            "system status",
            vec![
                "system",
                "status",
                "--profile",
                "full",
                "--server",
                "none",
                "--emit",
                "yaml",
            ],
            Command::SystemStatus {
                options: SystemStatusOptions {
                    profile: VerifyProfile::Full,
                    server: DurabilityMode::None,
                    emit_yaml: true,
                },
                output: OutputFormat::Yaml,
            },
        ),
        (
            "action list",
            vec![
                "action",
                "list",
                "--registry",
                "empty",
                "--emit",
                "postcard",
            ],
            Command::ActionList {
                output: OutputFormat::Postcard,
                registry: ActionRegistryMode::Empty,
            },
        ),
        (
            "action inspect",
            vec![
                "action",
                "inspect",
                "noop",
                "--registry",
                "registered",
                "--emit",
                "yaml",
            ],
            Command::ActionInspect {
                action_name: "noop".into(),
                output: OutputFormat::Yaml,
                registry: ActionRegistryMode::Registered,
            },
        ),
        (
            "ai-context",
            vec!["ai-context", "17", "--db", "state", "--emit", "yaml"],
            Command::AiContext {
                run_id: "17".into(),
                db: PathBuf::from("state"),
                output: OutputFormat::Yaml,
            },
        ),
        (
            "cancel",
            vec![
                "cancel", "17", "--db", "state", "--reason", "operator", "--emit", "yaml",
            ],
            Command::Cancel {
                run_id: "17".into(),
                db: PathBuf::from("state"),
                reason: Some("operator".into()),
                output: OutputFormat::Yaml,
            },
        ),
    ];
    for (name, arguments, expected) in cases {
        assert!(commands.get(name).is_some(), "missing command {name}");
        assert_eq!(parsed(&arguments), Ok(expected), "{name}");
    }
    Ok(())
}

#[test]
fn operator_metadata_preserves_actual_modes_flags_and_requiredness() -> TestResult {
    let context = build("test");
    let commands = context.get("commands").ok_or("commands")?;
    let status = commands.get("status").ok_or("status")?;
    assert_eq!(
        status.pointer("/flags/--emit/values"),
        Some(&json!(["text", "yaml"]))
    );
    assert_eq!(
        status.pointer("/flags/--active-runs/type"),
        Some(&json!("usize"))
    );
    assert_eq!(
        status.pointer("/flags/--queue-depth/type"),
        Some(&json!("usize"))
    );
    assert_eq!(
        status.pointer("/flags/--trace-dropped/type"),
        Some(&json!("u64"))
    );
    assert!(status.pointer("/flags/--db").is_none());
    let system = commands.get("system status").ok_or("system status")?;
    assert_eq!(
        system.pointer("/flags/--profile/values"),
        Some(&json!(["quick", "standard", "full"]))
    );
    assert_eq!(
        system.pointer("/flags/--profile/default"),
        Some(&json!("standard"))
    );
    assert_eq!(
        system.pointer("/flags/--server/values"),
        Some(&json!(["none"]))
    );
    assert_eq!(
        system.pointer("/flags/--emit/values"),
        Some(&json!(["text", "yaml"]))
    );
    assert!(system.pointer("/flags/--db").is_none());
    for name in ["action list", "action inspect"] {
        let action = commands.get(name).ok_or("action metadata")?;
        assert_eq!(
            action.pointer("/flags/--registry/values"),
            Some(&json!(["registered", "empty", "uninitialized"]))
        );
        assert_eq!(
            action.pointer("/flags/--registry/default"),
            Some(&json!("registered"))
        );
        assert_eq!(
            action.pointer("/flags/--emit/values"),
            Some(&json!(["text", "yaml", "postcard"]))
        );
    }
    assert_eq!(
        commands.pointer("/action inspect/positionals"),
        Some(&json!(["action_name"]))
    );
    for name in ["ai-context", "cancel"] {
        assert_eq!(
            commands
                .get(name)
                .and_then(|value| value.pointer("/flags/--db/required")),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            commands
                .get(name)
                .and_then(|value| value.get("positionals")),
            Some(&json!(["run_id"]))
        );
    }
    assert_eq!(
        commands.pointer("/cancel/flags/--reason/type"),
        Some(&json!("string"))
    );
    assert_eq!(
        parsed(&["status", "--emit", "postcard"]),
        Err(ParseError::InvalidStatusArgument(
            "postcard emit is not supported for status".into()
        ))
    );
    assert_eq!(
        parsed(&["system", "status", "--server", "strict"]),
        Err(ParseError::UnknownServerMode("strict".into()))
    );
    assert_eq!(
        parsed(&["action", "list", "--registry", "invented"]),
        Err(ParseError::UnknownActionRegistry("invented".into()))
    );
    Ok(())
}

#[test]
fn event_trace_filters_and_required_retry_step_are_described() -> TestResult {
    let context = build("test");
    let commands = context.get("commands").ok_or("commands")?;
    let statuses = json!([
        "pending",
        "active",
        "waiting_answer",
        "cancelled",
        "completed",
        "failed"
    ]);
    for name in ["events", "trace"] {
        assert_eq!(
            commands
                .get(name)
                .and_then(|command| command.pointer("/flags/--status/values")),
            Some(&statuses)
        );
        assert!(
            commands
                .get(name)
                .and_then(|command| command.pointer("/flags/--limit"))
                .is_some()
        );
    }
    for (flag, kind) in [
        ("--step", "u16"),
        ("--action", "u16"),
        ("--since-seq", "u64"),
        ("--until-seq", "u64"),
        ("--limit", "usize"),
    ] {
        assert_eq!(
            commands
                .get("trace")
                .and_then(|command| command.get("flags"))
                .and_then(|flags| flags.get(flag))
                .and_then(|flag| flag.get("type")),
            Some(&json!(kind))
        );
    }
    assert_eq!(
        commands.pointer("/events/flags/--limit/type"),
        Some(&json!("i64"))
    );
    assert_eq!(
        commands.pointer("/retry/flags/--step/required"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        parsed(&["retry", "17", "--db", "state"]),
        Err(ParseError::MissingArgument("--step"))
    );
    assert_eq!(
        parsed(&["retry", "17", "--db", "state", "--step", "3"]),
        Ok(Command::Retry {
            run_id: "17".into(),
            db: PathBuf::from("state"),
            step: 3,
            output: OutputFormat::Text
        })
    );
    assert_eq!(
        parsed(&["events", "17", "--db", "state", "--status", "invented"]),
        Err(ParseError::UnknownEventStatus("invented".into()))
    );
    Ok(())
}

#[test]
fn described_query_filters_parse_to_exact_values() {
    for (label, event_status, trace_status) in [
        ("pending", EventStatus::Pending, TraceStatus::Pending),
        ("active", EventStatus::Active, TraceStatus::Active),
        (
            "waiting_answer",
            EventStatus::WaitingAnswer,
            TraceStatus::WaitingAnswer,
        ),
        ("cancelled", EventStatus::Cancelled, TraceStatus::Cancelled),
        ("completed", EventStatus::Completed, TraceStatus::Completed),
        ("failed", EventStatus::Failed, TraceStatus::Failed),
    ] {
        assert_eq!(
            parsed(&[
                "events", "17", "--db", "state", "--status", label, "--limit", "12", "--emit",
                "yaml"
            ]),
            Ok(Command::Events {
                run_id: "17".into(),
                db: PathBuf::from("state"),
                output: OutputFormat::Yaml,
                status: Some(event_status),
                limit: Some(12)
            })
        );
        assert_eq!(
            parsed(&[
                "trace",
                "17",
                "--db",
                "state",
                "--step",
                "3",
                "--action",
                "1",
                "--status",
                label,
                "--since-seq",
                "2",
                "--until-seq",
                "9",
                "--limit",
                "12",
                "--emit",
                "yaml"
            ]),
            Ok(Command::Trace {
                run_id: "17".into(),
                db: PathBuf::from("state"),
                output: OutputFormat::Yaml,
                filters: TraceFilters {
                    step: Some(3),
                    action: Some(1),
                    status: Some(trace_status),
                    since_seq: Some(2),
                    until_seq: Some(9),
                    limit: Some(12)
                }
            })
        );
    }
    assert_eq!(
        parsed(&["status", "--db", "state"]),
        Err(ParseError::InvalidStatusArgument(
            "unknown flag --db".into()
        ))
    );
    assert_eq!(
        parsed(&["system", "status", "--db", "state"]),
        Err(ParseError::InvalidSystemStatusArgument(
            "unknown flag --db".into()
        ))
    );
}

#[test]
fn factual_explain_metadata_matches_existing_parser_shape() {
    let context = build("test");
    assert_eq!(
        context.pointer("/commands/explain"),
        Some(&json!({
            "summary": "Compile without execution and report the execution plan.",
            "positionals": ["workflow.yaml"],
            "flags": {"--emit": {"type":"enum", "values":["text","yaml","postcard"], "default":"text"}}
        }))
    );
    assert_eq!(
        parsed(&["explain", "workflow.yaml", "--emit", "postcard"]),
        Ok(Command::Explain {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Postcard,
        })
    );
}

#[test]
fn workflow_diff_metadata_keeps_run_shape_and_adds_workflow_variant() {
    let context = build("test");
    assert_eq!(
        context.pointer("/commands/diff"),
        Some(&json!({
            "summary": "Compare two durable runs.",
            "positionals": ["run_a", "run_b"],
            "flags": {
                "--db": {"type":"path", "required":true},
                "--emit": {"type":"enum", "values":["text","yaml","postcard"], "default":"text"}
            },
            "variants": [{
                "summary": "Compare workflow definitions using exactly one baseline: --against old.yaml or --db path; these flags are mutually exclusive.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--against": {"type":"path"}, "--db": {"type":"path"},
                    "--emit": {"type":"enum", "values":["text","yaml","postcard"], "default":"text"}
                }
            }]
        }))
    );
}

#[test]
fn workflow_diff_metadata_forms_bind_exact_parser_modes_and_baseline_refusals() {
    for (flag, value, against, db) in [
        (
            "--against",
            "old.yaml",
            Some(PathBuf::from("old.yaml")),
            None,
        ),
        ("--db", "journal", None, Some(PathBuf::from("journal"))),
    ] {
        assert_eq!(
            parsed(&["diff", "workflow.yaml", flag, value, "--emit", "yaml"]),
            Ok(Command::WorkflowDiff {
                workflow: PathBuf::from("workflow.yaml"),
                against,
                db,
                output: OutputFormat::Yaml,
            })
        );
    }
    assert_eq!(
        parsed(&["diff", "1", "2", "--db", "journal", "--emit", "postcard"]),
        Ok(Command::Diff {
            run_a: "1".into(),
            run_b: "2".into(),
            db: PathBuf::from("journal"),
            output: OutputFormat::Postcard,
        })
    );
    assert_eq!(
        parsed(&["diff", "workflow.yaml"]),
        Ok(Command::WorkflowDiff {
            workflow: PathBuf::from("workflow.yaml"),
            against: None,
            db: None,
            output: OutputFormat::Text,
        })
    );
    assert_eq!(
        parsed(&[
            "diff",
            "workflow.yaml",
            "--against",
            "old.yaml",
            "--db",
            "journal"
        ]),
        Err(ParseError::InvalidArgument(
            "--against and --db are alternate baseline modes".into()
        ))
    );
    assert_eq!(
        parsed(&["diff", "1", "2"]),
        Err(ParseError::MissingArgument("--db"))
    );
    assert_eq!(
        parsed(&["diff", "1", "2", "--against", "old.yaml"]),
        Err(ParseError::InvalidArgument(
            "--against cannot be combined with run comparison".into()
        ))
    );
}
