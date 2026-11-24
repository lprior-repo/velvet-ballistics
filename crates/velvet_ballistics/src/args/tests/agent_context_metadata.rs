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

const STATUS_VALUES: [&str; 6] = [
    "pending",
    "active",
    "waiting_answer",
    "cancelled",
    "completed",
    "failed",
];

#[test]
fn query_command_metadata_types_and_status_enum_match_parser() -> TestResult {
    let context = build("test");
    let expected_types = [
        ("events", "--limit", "i64"),
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
    for command in ["events", "trace"] {
        assert_eq!(
            context.pointer(&format!("/commands/{command}/flags/--status/values")),
            Some(&json!(STATUS_VALUES)),
            "{command} --status"
        );
        for status in STATUS_VALUES {
            assert!(
                parsed(&[command, "7", "--db", "s", "--status", status]).is_ok(),
                "{command} must accept advertised status {status}"
            );
        }
        assert_eq!(
            parsed(&[command, "7", "--db", "s", "--status", "bogus"]),
            Err(ParseError::UnknownEventStatus("bogus".into()))
        );
    }
    Ok(())
}

#[test]
fn events_and_trace_numeric_domains_match_metadata_types() -> TestResult {
    assert_eq!(
        parsed(&[
            "events", "7", "--db", "s", "--status", "failed", "--limit", "-1"
        ]),
        Ok(Command::Events {
            run_id: "7".into(),
            db: PathBuf::from("s"),
            output: OutputFormat::Text,
            status: Some(crate::args::EventStatus::Failed),
            limit: Some(-1),
        })
    );
    assert!(parsed(&["events", "7", "--db", "s", "--limit", "9223372036854775808"]).is_err());
    assert!(parsed(&["trace", "7", "--db", "s", "--since-seq", "-1"]).is_err());
    assert!(parsed(&["trace", "7", "--db", "s", "--step", "65536"]).is_err());
    assert!(parsed(&["trace", "7", "--db", "s", "--limit", "-1"]).is_err());
    Ok(())
}
