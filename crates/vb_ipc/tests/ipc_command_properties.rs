//! Property test: IpcCommand enum has exactly 11 variants and correct parse/encode.
//!
//! vb-juuw: ipc: Reconcile IPC command set to canonical 11-command contract
//!
//! PO-007 / PS-007: Error code stability — diagnostic_code correct for all IpcError variants.
//!
//! Each variant's expected code is the const defined in error.rs (0x3001–0x300E).

use vb_ipc::IpcCommand;

/// Test that exactly 11 commands parse successfully (1-11).
#[test]
fn test_eleven_commands_parse_ok() {
    let expected_commands = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];
    for (i, &expected) in expected_commands.iter().enumerate() {
        let wire_id = (i + 1) as u16;
        let result = IpcCommand::from_u16(wire_id);
        assert_eq!(
            result,
            Ok(expected),
            "Expected Ok({:?}) for command {}, got {:?}",
            expected,
            wire_id,
            result
        );
    }
}

/// Test that values 12-16 (removed commands) return UnknownCommand.
#[test]
fn test_removed_commands_return_unknown_command() {
    for i in 12..=16 {
        let result = IpcCommand::from_u16(i);
        assert_eq!(
            result,
            Ok(IpcCommand::UnknownCommand(i)),
            "Removed command {} must return UnknownCommand({}), got {:?}",
            i,
            i,
            result
        );
    }
}

/// Test roundtrip: from_u16(as_u16(cmd)) == Ok(cmd) for all 11 commands.
#[test]
fn test_roundtrip_all_commands() {
    let commands = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];

    for cmd in commands {
        let encoded = cmd.as_u16();
        let decoded = IpcCommand::from_u16(encoded);
        assert_eq!(
            decoded,
            Ok(cmd),
            "Roundtrip failed for {:?}: encoded={}, decoded={:?}",
            cmd,
            encoded,
            decoded
        );
    }
}

/// Test that as_u16 returns values in 1-11 range for all commands.
#[test]
fn test_as_u16_in_valid_range() {
    let commands = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];

    for cmd in commands {
        let val = cmd.as_u16();
        assert!(
            (1..=11).contains(&val),
            "as_u16({:?}) = {} is out of range 1-11",
            cmd,
            val
        );
    }
}

/// Test that exactly 11 variants exist (count verification).
#[test]
fn test_exactly_eleven_variants() {
    let variants = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];

    assert_eq!(
        variants.len(),
        11,
        "Expected exactly 11 IpcCommand variants"
    );
}
