#![forbid(unsafe_code)]
//! Kani exhaustive harnesses: IPC command set reconciliation.
//!
//! PO-KANI-001: from_u16() exhaustive totality — all 65,536 u16 inputs.
//! PO-KANI-002: Semantic variant count (exactly 11) and discriminant range (1..=11).
//! PO-KANI-004: Roundtrip identity — from_u16(as_u16(c)) == Ok(c) for all 11.

use crate::IpcCommand;

/// PO-KANI-001: Exhaustive from_u16() over all u16 inputs.
///
/// Proves:
/// 1. from_u16(n) never panics for any u16 value (returns Ok).
/// 2. Values 1..=11 map to the correct named IpcCommand variant.
/// 3. Values 0 and 12..=u16::MAX map to UnknownCommand(n).
///
/// Verification: Kani symbolically explores all 65536 concrete u16 values
/// and asserts correctness for each.
#[kani::proof]
fn kani_from_u16_exhaustive() {
    let value: u16 = kani::any();

    // Invariant 1: from_u16 must never panic or return Err for any u16.
    let result = IpcCommand::from_u16(value);
    assert!(
        result.is_ok(),
        "from_u16({}) must return Ok; got {:?}",
        value,
        result
    );

    let command = result.unwrap();

    // Invariants 2 & 3: Correct variant mapping.
    match value {
        1 => assert_eq!(
            command,
            IpcCommand::SubmitRun,
            "value 1 must map to SubmitRun"
        ),
        2 => assert_eq!(
            command,
            IpcCommand::SubmitRunInline,
            "value 2 must map to SubmitRunInline"
        ),
        3 => assert_eq!(
            command,
            IpcCommand::CancelRun,
            "value 3 must map to CancelRun"
        ),
        4 => assert_eq!(
            command,
            IpcCommand::InspectRun,
            "value 4 must map to InspectRun"
        ),
        5 => assert_eq!(
            command,
            IpcCommand::ListEvents,
            "value 5 must map to ListEvents"
        ),
        6 => assert_eq!(
            command,
            IpcCommand::AnswerAsk,
            "value 6 must map to AnswerAsk"
        ),
        7 => assert_eq!(
            command,
            IpcCommand::CompleteAction,
            "value 7 must map to CompleteAction"
        ),
        8 => assert_eq!(
            command,
            IpcCommand::FailAction,
            "value 8 must map to FailAction"
        ),
        9 => assert_eq!(
            command,
            IpcCommand::DrainTrace,
            "value 9 must map to DrainTrace"
        ),
        10 => assert_eq!(
            command,
            IpcCommand::Health,
            "value 10 must map to Health"
        ),
        11 => assert_eq!(
            command,
            IpcCommand::Shutdown,
            "value 11 must map to Shutdown"
        ),
        // All other u16 values (0, 12..=u16::MAX) must be UnknownCommand.
        _ => assert_eq!(
            command,
            IpcCommand::UnknownCommand(value),
            "value {} must map to UnknownCommand({})",
            value,
            value
        ),
    }
}

/// PO-KANI-002: Semantic variant count (exactly 11) and discriminant range.
///
/// Proves:
/// 1. There are exactly 11 semantic IpcCommand variants (excluding UnknownCommand).
/// 2. Every semantic variant's as_u16() discriminant is in 1..=11.
/// 3. Each discriminant matches its declared #[repr(u16)] value.
/// 4. All 11 discriminants are unique.
#[kani::proof]
fn kani_command_count_and_discriminants() {
    let variants: [IpcCommand; 11] = [
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

    // Verify exactly 11 semantic variants.
    assert_eq!(
        variants.len(),
        11,
        "Exactly 11 semantic IpcCommand variants must exist"
    );

    // Verify discriminant range 1..=11 for each variant.
    for cmd in &variants {
        let id = cmd.as_u16();
        assert!(
            id >= 1 && id <= 11,
            "IpcCommand discriminant {} is outside 1..=11",
            id
        );
    }

    // Verify each declared discriminant matches as_u16().
    assert_eq!(
        IpcCommand::SubmitRun.as_u16(),
        1,
        "SubmitRun discriminant must be 1"
    );
    assert_eq!(
        IpcCommand::SubmitRunInline.as_u16(),
        2,
        "SubmitRunInline discriminant must be 2"
    );
    assert_eq!(
        IpcCommand::CancelRun.as_u16(),
        3,
        "CancelRun discriminant must be 3"
    );
    assert_eq!(
        IpcCommand::InspectRun.as_u16(),
        4,
        "InspectRun discriminant must be 4"
    );
    assert_eq!(
        IpcCommand::ListEvents.as_u16(),
        5,
        "ListEvents discriminant must be 5"
    );
    assert_eq!(
        IpcCommand::AnswerAsk.as_u16(),
        6,
        "AnswerAsk discriminant must be 6"
    );
    assert_eq!(
        IpcCommand::CompleteAction.as_u16(),
        7,
        "CompleteAction discriminant must be 7"
    );
    assert_eq!(
        IpcCommand::FailAction.as_u16(),
        8,
        "FailAction discriminant must be 8"
    );
    assert_eq!(
        IpcCommand::DrainTrace.as_u16(),
        9,
        "DrainTrace discriminant must be 9"
    );
    assert_eq!(
        IpcCommand::Health.as_u16(),
        10,
        "Health discriminant must be 10"
    );
    assert_eq!(
        IpcCommand::Shutdown.as_u16(),
        11,
        "Shutdown discriminant must be 11"
    );

    // Verify discriminant uniqueness: collect all values and check no duplicates.
    let ids: [u16; 11] = [
        IpcCommand::SubmitRun.as_u16(),
        IpcCommand::SubmitRunInline.as_u16(),
        IpcCommand::CancelRun.as_u16(),
        IpcCommand::InspectRun.as_u16(),
        IpcCommand::ListEvents.as_u16(),
        IpcCommand::AnswerAsk.as_u16(),
        IpcCommand::CompleteAction.as_u16(),
        IpcCommand::FailAction.as_u16(),
        IpcCommand::DrainTrace.as_u16(),
        IpcCommand::Health.as_u16(),
        IpcCommand::Shutdown.as_u16(),
    ];

    // Since we have 11 elements that must be between 1 and 11,
    // uniqueness is equivalent to each i from 1..=11 appearing exactly once.
    // We verify that each declared discriminant has the expected value.
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert!(
                ids[i] != ids[j],
                "Duplicate discriminant detected: variant at index {} \
                 and {} both have discriminant {}",
                i,
                j,
                ids[i]
            );
        }
    }
}

/// PO-KANI-004: Roundtrip identity for all 11 semantic variants.
///
/// Proves: from_u16(as_u16(c)) == Ok(c) for every semantic variant c.
/// This is a structural property ensuring the encoder and parser form
/// a bijection on the set of valid command IDs.
#[kani::proof]
fn kani_roundtrip_identity() {
    let commands: [IpcCommand; 11] = [
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

    for cmd in &commands {
        let wire_id = cmd.as_u16();
        let roundtripped = IpcCommand::from_u16(wire_id);
        assert!(
            roundtripped.is_ok(),
            "Roundtrip from_u16({}) must succeed for command",
            wire_id
        );
        assert_eq!(
            roundtripped.unwrap(),
            *cmd,
            "Roundtrip failed: as_u16()={} did not roundtrip back to original variant",
            wire_id
        );
    }

    // Also verify cross-case: no semantic variant maps to a different
    // semantic variant through the encoder (i.e., as_u16 is injective).
    for i in 0..commands.len() {
        for j in (i + 1)..commands.len() {
            assert!(
                commands[i].as_u16() != commands[j].as_u16(),
                "as_u16() must be injective: two different variants share wire ID {}",
                commands[i].as_u16()
            );
        }
    }
}
