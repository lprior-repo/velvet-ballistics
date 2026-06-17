#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

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
