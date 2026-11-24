use crate::agent_context::build;
use crate::args::{AnswerSelector, Command, OutputFormat, ParseError, parse_args};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parsed(arguments: &[&str]) -> Result<Command, ParseError> {
    let arguments = std::iter::once("velvet-ballistics")
        .chain(arguments.iter().copied())
        .map(OsString::from)
        .collect::<Vec<_>>();
    parse_args(&arguments)
}

#[test]
fn answer_metadata_matches_current_slot_route_and_rejects_legacy_flags() -> TestResult {
    let context = build("test");
    let answer = context
        .pointer("/commands/answer")
        .ok_or("answer command")?;
    assert_eq!(
        answer.get("summary").and_then(Value::as_str),
        Some("Answer a suspended ask by its pending slot using --slot with --value.")
    );
    let flags = answer.get("flags").ok_or("answer flags")?;
    assert_answer_flags(flags)?;
    assert_eq!(flags.get("--step"), None);
    assert_eq!(flags.get("--value-file"), None);
    assert_eq!(flags.pointer("/--db/required"), Some(&json!(true)));
    assert_eq!(
        flags.get("--emit"),
        Some(&json!({
            "type": "enum",
            "values": ["text", "yaml", "postcard"],
            "default": "text"
        }))
    );
    assert_legacy_parser_flags_are_rejected()?;
    assert_slot_parser_route()?;
    Ok(())
}

fn assert_answer_flags(flags: &Value) -> TestResult {
    for (flag, kind) in [("--slot", "u16"), ("--value", "path")] {
        assert_eq!(flags.pointer(&format!("/{flag}/type")), Some(&json!(kind)));
        assert_eq!(
            flags.pointer(&format!("/{flag}/required")),
            Some(&json!(true))
        );
    }
    Ok(())
}

fn assert_legacy_parser_flags_are_rejected() -> TestResult {
    assert_eq!(
        parsed(&[
            "answer",
            "17",
            "--step",
            "3",
            "--value-file",
            "answer.bin",
            "--db",
            "state",
        ]),
        Err(ParseError::UnknownFlag {
            command: "answer",
            flag: "--step".into(),
        })
    );
    assert_eq!(
        parsed(&[
            "answer",
            "17",
            "--slot",
            "2",
            "--value-file",
            "answer.bin",
            "--db",
            "state",
        ]),
        Err(ParseError::UnknownFlag {
            command: "answer",
            flag: "--value-file".into(),
        })
    );
    Ok(())
}

fn assert_slot_parser_route() -> TestResult {
    assert_eq!(
        parsed(&[
            "answer",
            "17",
            "--slot",
            "2",
            "--value",
            "answer.bin",
            "--db",
            "state",
        ]),
        Ok(Command::Answer {
            run_id: "17".into(),
            selector: AnswerSelector::Slot(vb_core::ids::SlotIdx::new(2)),
            value_file: PathBuf::from("answer.bin"),
            db: PathBuf::from("state"),
            output: OutputFormat::Text,
        })
    );
    Ok(())
}

fn flag_names(context: &Value, command: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let flags = context
        .pointer(&format!("/commands/{command}/flags"))
        .and_then(Value::as_object)
        .ok_or("command flags")?;
    let mut names: Vec<String> = flags.keys().cloned().collect();
    names.sort();
    Ok(names)
}

#[test]
fn query_command_metadata_lists_every_parser_accepted_flag() -> TestResult {
    let context = build("test");
    assert_eq!(
        flag_names(&context, "events")?,
        ["--db", "--emit", "--limit", "--status"]
    );
    assert_eq!(
        flag_names(&context, "trace")?,
        [
            "--action",
            "--db",
            "--emit",
            "--limit",
            "--since-seq",
            "--status",
            "--step",
            "--until-seq"
        ]
    );
    assert_eq!(flag_names(&context, "retry")?, ["--db", "--emit", "--step"]);
    assert_eq!(
        context.pointer("/commands/retry/flags/--step/required"),
        Some(&json!(true))
    );
    assert!(
        parsed(&[
            "events", "7", "--db", "s", "--status", "failed", "--limit", "5"
        ])
        .is_ok()
    );
    assert!(
        parsed(&[
            "trace",
            "7",
            "--db",
            "s",
            "--step",
            "1",
            "--action",
            "2",
            "--status",
            "active",
            "--since-seq",
            "3",
            "--until-seq",
            "9",
            "--limit",
            "4",
        ])
        .is_ok()
    );
    assert_eq!(
        parsed(&["retry", "7", "--db", "s"]),
        Err(ParseError::MissingArgument("--step"))
    );
    Ok(())
}

use crate::args::EventStatus;
use crate::commands_journal::{TraceFilters, TraceStatus};

const EVENT_STATUSES: [EventStatus; 6] = [
    EventStatus::Pending,
    EventStatus::Active,
    EventStatus::WaitingAnswer,
    EventStatus::Cancelled,
    EventStatus::Completed,
    EventStatus::Failed,
];

const TRACE_STATUSES: [TraceStatus; 6] = [
    TraceStatus::Pending,
    TraceStatus::Active,
    TraceStatus::WaitingAnswer,
    TraceStatus::Cancelled,
    TraceStatus::Completed,
    TraceStatus::Failed,
];

fn events_cmd(extra: &[&str]) -> Result<Command, ParseError> {
    parsed(&[&["events", "7", "--db", "s"], extra].concat())
}

fn trace_cmd(extra: &[&str]) -> Result<Command, ParseError> {
    parsed(&[&["trace", "7", "--db", "s"], extra].concat())
}

#[test]
fn query_command_metadata_types_and_status_enum_match_parser() -> TestResult {
    let context = build("test");
    let expected_types = [
        ("events", "--status", "enum"),
        ("events", "--limit", "i64"),
        ("trace", "--status", "enum"),
        ("trace", "--step", "u16"),
        ("trace", "--action", "u16"),
        ("trace", "--since-seq", "u64"),
        ("trace", "--until-seq", "u64"),
        ("trace", "--limit", "usize"),
        ("retry", "--step", "u16"),
    ];
    for (command, flag, kind) in expected_types {
        assert_eq!(
            context.pointer(&format!("/commands/{command}/flags/{flag}/type")),
            Some(&json!(kind)),
            "{command} {flag}"
        );
    }
    let advertised: Vec<&str> = EVENT_STATUSES.iter().map(EventStatus::as_str).collect();
    let advertised_trace: Vec<&str> = TRACE_STATUSES.iter().map(|s| s.as_str()).collect();
    assert_eq!(advertised, advertised_trace);
    for command in ["events", "trace"] {
        assert_eq!(
            context.pointer(&format!("/commands/{command}/flags/--status/values")),
            Some(&json!(advertised)),
            "{command} --status"
        );
    }
    Ok(())
}

#[test]
fn status_flag_returns_the_matching_variant_and_refuses_unknown() -> TestResult {
    for status in EVENT_STATUSES {
        let command = events_cmd(&["--status", status.as_str()]).map_err(|e| format!("{e:?}"))?;
        let Command::Events { status: got, .. } = command else {
            return Err("events parsed to a different command".into());
        };
        assert_eq!(got, Some(status));
    }
    for status in TRACE_STATUSES {
        let command = trace_cmd(&["--status", status.as_str()]).map_err(|e| format!("{e:?}"))?;
        let Command::Trace { filters, .. } = command else {
            return Err("trace parsed to a different command".into());
        };
        assert_eq!(filters.status, Some(status));
    }
    let unknown = Err(ParseError::UnknownEventStatus("bogus".into()));
    assert_eq!(events_cmd(&["--status", "bogus"]), unknown);
    assert_eq!(trace_cmd(&["--status", "bogus"]), unknown);
    Ok(())
}

#[test]
fn trace_parses_every_filter_into_its_field() -> TestResult {
    let command = trace_cmd(&[
        "--step",
        "1",
        "--action",
        "2",
        "--status",
        "active",
        "--since-seq",
        "3",
        "--until-seq",
        "9",
        "--limit",
        "4",
    ])
    .map_err(|e| format!("{e:?}"))?;
    let Command::Trace { filters, .. } = command else {
        return Err("trace parsed to a different command".into());
    };
    assert_eq!(
        filters,
        TraceFilters {
            step: Some(1),
            action: Some(2),
            status: Some(TraceStatus::Active),
            since_seq: Some(3),
            until_seq: Some(9),
            limit: Some(4),
        }
    );
    Ok(())
}

#[test]
fn events_limit_is_signed_and_bounded_to_i64() -> TestResult {
    for limit in [i64::MIN, -1, 0, i64::MAX] {
        let command = events_cmd(&["--limit", &limit.to_string()]).map_err(|e| format!("{e:?}"))?;
        let Command::Events { limit: got, .. } = command else {
            return Err("events parsed to a different command".into());
        };
        assert_eq!(got, Some(limit));
    }
    assert!(events_cmd(&["--limit", &format!("{}0", i64::MAX)]).is_err());
    Ok(())
}

#[test]
fn trace_and_retry_numeric_flags_accept_their_max_and_refuse_overflow() {
    let u16_max = u16::MAX.to_string();
    let u16_over = format!("{u16_max}0");
    for flag in ["--step", "--action"] {
        assert!(trace_cmd(&[flag, &u16_max]).is_ok(), "trace {flag} max");
        assert!(trace_cmd(&[flag, &u16_over]).is_err(), "trace {flag} over");
    }
    assert!(parsed(&["retry", "7", "--db", "s", "--step", &u16_max]).is_ok());
    assert!(parsed(&["retry", "7", "--db", "s", "--step", &u16_over]).is_err());
    let u64_max = u64::MAX.to_string();
    let u64_over = format!("{u64_max}0");
    for flag in ["--since-seq", "--until-seq"] {
        assert!(trace_cmd(&[flag, &u64_max]).is_ok(), "trace {flag} max");
        assert!(trace_cmd(&[flag, &u64_over]).is_err(), "trace {flag} over");
        assert!(trace_cmd(&[flag, "-1"]).is_err(), "trace {flag} negative");
    }
    let usize_max = usize::MAX.to_string();
    assert!(trace_cmd(&["--limit", &usize_max]).is_ok());
    assert!(trace_cmd(&["--limit", &format!("{usize_max}0")]).is_err());
    assert!(trace_cmd(&["--limit", "-1"]).is_err());
}
