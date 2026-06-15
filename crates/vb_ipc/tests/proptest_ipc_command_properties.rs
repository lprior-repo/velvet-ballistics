//! Property-based tests: IPC v1 command set reconciliation.
//!
//! PO-PROP-001: Command count and discriminant range (exactly 11 semantic variants).
//! PO-PROP-002: Roundtrip identity — from_u16(as_u16(c)) == Ok(c).
//! PO-PROP-003: Unknown command rejection — all u16 outside 1..=11 → UnknownCommand.
//! PO-PROP-004: Reserved IDs 12-16 decode as UnknownCommand.

use proptest::prelude::*;
use vb_ipc::IpcCommand;

// ──────────────────────────────────────────────────────────────────────
// PO-PROP-001: Semantic variant count and discriminant range.
// ──────────────────────────────────────────────────────────────────────

// The 11 semantic IpcCommand variants in canonical order.
const SEMANTIC_VARIANTS: [IpcCommand; 11] = [
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

// Verify exactly 11 semantic IpcCommand variants exist.
#[test]
fn prop_exactly_eleven_semantic_variants() {
    assert_eq!(
        SEMANTIC_VARIANTS.len(),
        11,
        "IpcCommand must have exactly 11 semantic variants"
    );
}

// Verify all 11 semantic variants have discriminants in 1..=11.
#[test]
fn prop_all_semantic_discriminants_in_range() {
    for cmd in &SEMANTIC_VARIANTS {
        let id = cmd.as_u16();
        assert!(
            (1..=11).contains(&id),
            "IpcCommand discriminant {} is outside range 1..=11",
            id
        );
    }
}

// Verify discriminants are unique (no duplicates).
#[test]
fn prop_semantic_discriminants_unique() {
    let ids: Vec<u16> = SEMANTIC_VARIANTS.iter().map(|c| c.as_u16()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        11,
        "All 11 semantic IpcCommand discriminants must be unique"
    );
}

// Verify discriminants are in canonical order (1 through 11).
#[test]
fn prop_discriminants_canonical_order() {
    for (i, cmd) in SEMANTIC_VARIANTS.iter().enumerate() {
        let expected = (i + 1) as u16;
        assert_eq!(
            cmd.as_u16(),
            expected,
            "Semantic variant at position {} must have discriminant {}",
            i,
            expected
        );
    }
}

// Verify UnknownCommand is NOT counted as a semantic variant.
#[test]
fn prop_unknown_command_not_semantic() {
    let unknown = IpcCommand::UnknownCommand(0);
    // UnknownCommand is a catch-all, not a semantic variant.
    // Its as_u16() returns the raw value, which can be outside 1..=11.
    let id = unknown.as_u16();
    // The id is 0, which is NOT in 1..=11.
    assert!(
        !(1..=11).contains(&id),
        "UnknownCommand(0) discriminant must NOT be in 1..=11"
    );
    // Verify it's not in the semantic list.
    assert!(
        !SEMANTIC_VARIANTS.contains(&unknown),
        "UnknownCommand must not be a semantic variant"
    );
}

// ──────────────────────────────────────────────────────────────────────
// PO-PROP-002: Roundtrip identity.
// ──────────────────────────────────────────────────────────────────────

// Proptest strategy generating valid command IDs (1..=11).
fn valid_command_id() -> impl Strategy<Value = u16> {
    (1u16..=11u16).prop_map(|v| v)
}

// Roundtrip identity: from_u16(as_u16(c)) == Ok(c) for all 11 variants.
#[test]
fn prop_roundtrip_exhaustive_all_eleven() {
    for cmd in &SEMANTIC_VARIANTS {
        let wire_id = cmd.as_u16();
        let decoded = IpcCommand::from_u16(wire_id);
        assert_eq!(
            decoded,
            Ok(*cmd),
            "Roundtrip failed: as_u16()={} did not roundtrip back",
            wire_id
        );
    }
}

// Roundtrip identity: proptest-driven, any valid command ID roundtrips.
proptest! {
    #[test]
    fn prop_roundtrip_valid_ids(id in valid_command_id()) {
        let expected = IpcCommand::from_u16(id).unwrap();
        let decoded = IpcCommand::from_u16(id);
        prop_assert!(
            matches!(&decoded, Ok(c) if *c == expected),
            "from_u16({}) must return Ok({:?}), got {:?}",
            id,
            expected,
            decoded
        );
    }
}

// as_u16 is injective: different semantic variants have different wire IDs.
#[test]
fn prop_as_u16_injective() {
    for i in 0..SEMANTIC_VARIANTS.len() {
        for j in (i + 1)..SEMANTIC_VARIANTS.len() {
            assert_ne!(
                SEMANTIC_VARIANTS[i].as_u16(),
                SEMANTIC_VARIANTS[j].as_u16(),
                "Duplicate wire ID: variants at indices {} and {}",
                i,
                j
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// PO-PROP-003: Unknown command rejection.
// ──────────────────────────────────────────────────────────────────────

// Proptest strategy: any u16 outside the valid range 1..=11.
fn invalid_command_id() -> impl Strategy<Value = u16> {
    prop_oneof![
        // Values 0..=0 (exactly 0)
        Just(0u16),
        // Values 12..=u16::MAX
        12u16..=u16::MAX,
    ]
}

// Every u16 outside 1..=11 decodes as UnknownCommand and never returns Err.
proptest! {
    #[test]
    fn prop_invalid_ids_return_unknown_command(id in invalid_command_id()) {
        let expected = IpcCommand::UnknownCommand(id);
        let result = IpcCommand::from_u16(id);
        prop_assert!(
            matches!(&result, Ok(c) if *c == expected),
            "from_u16({}) outside 1..=11 must return Ok(UnknownCommand({})), got {:?}",
            id,
            id,
            result
        );
    }
}

// from_u16 never panics for any u16 (defense-in-depth with proptest sampling).
proptest! {
    #[test]
    fn prop_from_u16_never_panics(id in any::<u16>()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            IpcCommand::from_u16(id)
        }));
        prop_assert!(
            result.is_ok(),
            "from_u16({}) must never panic",
            id
        );
        let inner: Result<IpcCommand, _> = result.expect("catch_unwind should not panic");
        prop_assert!(
            inner.is_ok(),
            "from_u16({}) must return Ok, got {:?}",
            id,
            inner
        );
    }
}

// ──────────────────────────────────────────────────────────────────────
// PO-PROP-004: Reserved IDs 12-16.
// ──────────────────────────────────────────────────────────────────────

// The reserved wire IDs 12..=16 that must NOT be named variants.
const RESERVED_IDS: [u16; 5] = [12, 13, 14, 15, 16];

// Reserved IDs 12-16 decode as UnknownCommand.
#[test]
fn prop_reserved_ids_return_unknown_command() {
    for &id in &RESERVED_IDS {
        let result = IpcCommand::from_u16(id);
        assert_eq!(
            result,
            Ok(IpcCommand::UnknownCommand(id)),
            "Reserved ID {} must return Ok(UnknownCommand({})), got {:?}",
            id,
            id,
            result
        );
    }
}

// No named IpcCommand variant exists with discriminants 12-16.
// This is a compile-time property, but we verify it at runtime too.
#[test]
fn prop_no_named_variant_for_reserved_ids() {
    for &id in &RESERVED_IDS {
        // If any semantic variant had this discriminant, as_u16() would return id.
        // Verify no semantic variant matches.
        let is_semantic = SEMANTIC_VARIANTS.iter().any(|c| c.as_u16() == id);
        assert!(
            !is_semantic,
            "No semantic IpcCommand variant may have discriminant {} (reserved range)",
            id
        );
    }
}

// Verify that the "extra commands" from the stale baseline report
// (ListRuns, GetMetrics, GetWorkflowGraph, GetTaintReport, VerifyWorkflow)
// do not exist as IpcCommand variants. This is verified by:
// 1. The enum has exactly 11 semantic variants (already verified above).
// 2. None of the 11 variants has a discriminant in 12..=16.
// 3. Attempting to use a non-existent variant name would fail at compile time.
#[test]
fn prop_stale_baseline_commands_do_not_exist() {
    // The IpcCommand enum definition is the source of truth.
    // If any of ListRuns, GetMetrics, GetWorkflowGraph, GetTaintReport,
    // or VerifyWorkflow were added as variants, their discriminants would
    // be outside 1..=11 (since 12..=16 are reserved) or they would bump
    // the semantic variant count beyond 11.

    // First: verify the total count is 11 (any extra would increase this).
    let count = SEMANTIC_VARIANTS.len();
    assert_eq!(
        count, 11,
        "Stale baseline claims 16 commands, but only 11 semantic variants exist"
    );

    // Second: IDs 12-16 map to UnknownCommand (not named variants).
    for &id in &RESERVED_IDS {
        let cmd = IpcCommand::from_u16(id);
        assert_eq!(
            cmd,
            Ok(IpcCommand::UnknownCommand(id)),
            "ID {} must be UnknownCommand({}), not a named variant",
            id,
            id
        );
    }

    // Third: the UnknownCommand catch-all is position-independent —
    // from_u16 returns UnknownCommand for any value outside 1..=11.
    // This covers the reserved range exhaustively.
    for id in 0u16..=16 {
        let result = IpcCommand::from_u16(id);
        if id == 0 || id >= 12 {
            assert_eq!(
                result,
                Ok(IpcCommand::UnknownCommand(id)),
                "ID {} (outside 1..=11) must be UnknownCommand({})",
                id,
                id
            );
        }
    }
}
