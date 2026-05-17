use super::args;
use crate::args::{ActionRegistryMode, Command, OutputFormat, parse_args};

#[test]
fn parse_action_list_accepts_jsonl_output() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "--jsonl"]));
    assert!(
        matches!(parsed, Ok(Command::ActionList { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
        assert_eq!(registry, ActionRegistryMode::Registered);
    }
}
