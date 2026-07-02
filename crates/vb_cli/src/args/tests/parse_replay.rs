//! Parser tests for the `replay` command (vb-wy33p.2).
//!
//! The replay command must accept expected action ABI digests and expected
//! policy digests via repeatable `--expected-action-abi` and
//! `--expected-policy-digest` flags, plus an explicit
//! `--allow-empty-expectations` opt-in for the silent-bypass mode.
//! Malformed specs must fail with [`ParseError::InvalidReplayDigest`].
#![forbid(unsafe_code)]

use super::*;
use vb_core::{ActionId, StepIdx, WorkflowDigest};

fn hex(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn parse_replay_command(args: &[&str]) -> Result<Command, ParseError> {
    let full = std::iter::once("velvet-ballistics")
        .chain(args.iter().copied())
        .map(OsString::from)
        .collect::<Vec<_>>();
    parse_args(&full)
}

#[test]
fn parse_replay_accepts_no_digest_flags() {
    let parsed = parse_replay_command(&["replay", "1", "--db", "journal-db"]);

    assert!(matches!(parsed, Ok(Command::Replay { .. })));
    if let Ok(Command::Replay {
        expected_action_abi,
        expected_policy_digests,
        allow_empty_expectations,
        ..
    }) = parsed
    {
        assert!(expected_action_abi.is_empty());
        assert!(expected_policy_digests.is_empty());
        assert!(!allow_empty_expectations);
    }
}

#[test]
fn parse_replay_accepts_single_expected_action_abi() {
    let digest_bytes = [0xDE; 32];
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        &format!("5={}", hex(digest_bytes)),
    ];
    let parsed = parse_replay_command(&args);

    assert!(matches!(parsed, Ok(Command::Replay { .. })));
    if let Ok(Command::Replay {
        expected_action_abi,
        expected_policy_digests,
        allow_empty_expectations,
        ..
    }) = parsed
    {
        assert_eq!(expected_action_abi.len(), 1);
        assert_eq!(expected_action_abi[0].0, ActionId::new(5));
        assert_eq!(
            expected_action_abi[0].1,
            WorkflowDigest::from_bytes(digest_bytes)
        );
        assert!(expected_policy_digests.is_empty());
        assert!(!allow_empty_expectations);
    }
}

#[test]
fn parse_replay_accepts_multiple_expected_action_abi_flags() {
    let digest_a = [0xAA; 32];
    let digest_b = [0xBB; 32];
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        &format!("1={}", hex(digest_a)),
        "--expected-action-abi",
        &format!("2={}", hex(digest_b)),
    ];
    let parsed = parse_replay_command(&args);

    if let Ok(Command::Replay {
        expected_action_abi,
        ..
    }) = parsed
    {
        assert_eq!(expected_action_abi.len(), 2);
        assert_eq!(expected_action_abi[0].0, ActionId::new(1));
        assert_eq!(
            expected_action_abi[0].1,
            WorkflowDigest::from_bytes(digest_a)
        );
        assert_eq!(expected_action_abi[1].0, ActionId::new(2));
        assert_eq!(
            expected_action_abi[1].1,
            WorkflowDigest::from_bytes(digest_b)
        );
    } else {
        assert!(
            matches!(parsed, Ok(Command::Replay { .. })),
            "got {parsed:?}"
        );
    }
}

#[test]
fn parse_replay_accepts_expected_policy_digest_flags() {
    let digest_bytes = [0xCD; 32];
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-policy-digest",
        &format!("3={}", hex(digest_bytes)),
    ];
    let parsed = parse_replay_command(&args);

    if let Ok(Command::Replay {
        expected_policy_digests,
        ..
    }) = parsed
    {
        assert_eq!(expected_policy_digests.len(), 1);
        assert_eq!(expected_policy_digests[0].0, StepIdx::new(3));
        assert_eq!(
            expected_policy_digests[0].1,
            WorkflowDigest::from_bytes(digest_bytes)
        );
    } else {
        assert!(
            matches!(parsed, Ok(Command::Replay { .. })),
            "got {parsed:?}"
        );
    }
}

#[test]
fn parse_replay_accepts_allow_empty_expectations_switch() {
    let parsed = parse_replay_command(&[
        "replay",
        "1",
        "--db",
        "journal-db",
        "--allow-empty-expectations",
    ]);

    if let Ok(Command::Replay {
        allow_empty_expectations,
        ..
    }) = parsed
    {
        assert!(allow_empty_expectations);
    } else {
        assert!(
            matches!(parsed, Ok(Command::Replay { .. })),
            "got {parsed:?}"
        );
    }
}

#[test]
fn parse_replay_rejects_malformed_expected_action_abi_no_separator() {
    let parsed = parse_replay_command(&[
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        "5deadbeef",
    ]);

    assert!(
        matches!(parsed, Err(ParseError::InvalidReplayDigest(_))),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_rejects_malformed_expected_action_abi_empty_value() {
    // The parser must reject missing value via --expected-action-abi
    // directly followed by another flag. The validate_known_flags layer
    // surfaces this as MissingArgument because the next token starts with
    // "--" (treated as the next flag rather than a value).
    let parsed = parse_replay_command(&[
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        "--db",
        "ignored",
    ]);

    assert!(
        matches!(
            parsed,
            Err(ParseError::MissingArgument("--expected-action-abi"))
                | Err(ParseError::InvalidReplayDigest(_))
        ),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_rejects_expected_action_abi_with_invalid_action_id() {
    let digest_bytes = [0u8; 32];
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        &format!("notanumber={}", hex(digest_bytes)),
    ];
    let parsed = parse_replay_command(&args);

    assert!(
        matches!(parsed, Err(ParseError::InvalidReplayDigest(_))),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_rejects_expected_action_abi_with_wrong_digest_length() {
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        "5=deadbeef",
    ];
    let parsed = parse_replay_command(&args);

    assert!(
        matches!(parsed, Err(ParseError::InvalidReplayDigest(_))),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_rejects_expected_action_abi_with_non_hex_characters() {
    // 63 hex chars + 1 non-hex to hit the non-hex branch.
    let mut hex_value = String::from("0");
    hex_value.push_str(&"a".repeat(63));
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        &format!("5={hex_value}zz"),
    ];
    let parsed = parse_replay_command(&args);

    assert!(
        matches!(parsed, Err(ParseError::InvalidReplayDigest(_))),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_rejects_malformed_expected_policy_digest() {
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-policy-digest",
        "no-equals-here",
    ];
    let parsed = parse_replay_command(&args);

    assert!(
        matches!(parsed, Err(ParseError::InvalidReplayDigest(_))),
        "got {parsed:?}"
    );
}

#[test]
fn parse_replay_accepts_uppercase_hex_digits() {
    let digest_bytes = [0xAB; 32];
    let mut uppercase = String::new();
    for byte in digest_bytes {
        uppercase.push_str(&format!("{byte:02X}"));
    }
    let args = [
        "replay",
        "1",
        "--db",
        "journal-db",
        "--expected-action-abi",
        &format!("7={uppercase}"),
    ];
    let parsed = parse_replay_command(&args);

    if let Ok(Command::Replay {
        expected_action_abi,
        ..
    }) = parsed
    {
        assert_eq!(expected_action_abi.len(), 1);
        assert_eq!(expected_action_abi[0].0, ActionId::new(7));
        assert_eq!(
            expected_action_abi[0].1,
            WorkflowDigest::from_bytes(digest_bytes)
        );
    } else {
        assert!(
            matches!(parsed, Ok(Command::Replay { .. })),
            "got {parsed:?}"
        );
    }
}

#[test]
fn parse_replay_rejects_unknown_flag() {
    let parsed = parse_replay_command(&["replay", "1", "--db", "journal-db", "--bogus", "value"]);

    assert!(
        matches!(parsed, Err(ParseError::UnknownFlag { .. })),
        "got {parsed:?}"
    );
}
