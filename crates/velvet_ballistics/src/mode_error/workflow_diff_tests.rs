#![cfg(test)]
use super::{CommandMode, command_mode};
use crate::args::{Command, parse_args};
use std::ffi::OsString;

fn parsed(arguments: &[&str]) -> Command {
    let arguments: Vec<OsString> = arguments.iter().map(OsString::from).collect();
    parse_args(&arguments).expect("current CLI arguments")
}

#[test]
fn workflow_diff_mode_follows_the_parsed_baseline_storage_requirement() {
    for (flag, baseline, expected) in [
        ("--against", "old.yaml", CommandMode::Pure),
        ("--db", "journal", CommandMode::Storage),
    ] {
        let command = parsed(&["velvet-ballistics", "diff", "workflow.yaml", flag, baseline]);
        assert!(matches!(command, Command::WorkflowDiff { .. }));
        assert_eq!(command_mode(&command), expected);
    }
    let command = parsed(&["velvet-ballistics", "diff", "1", "2", "--db", "journal"]);
    assert!(matches!(command, Command::Diff { .. }));
    assert_eq!(command_mode(&command), CommandMode::Storage);
}
