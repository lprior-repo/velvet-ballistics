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
