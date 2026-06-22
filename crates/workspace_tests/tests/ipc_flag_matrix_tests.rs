#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! IPC command flag matrix tests — vb-39jp
//!
//! Exhaustive test coverage for IPC command flag roundtrip and
//! reserved-bit validation across all 11 v1 commands.
//!
//! ## Test Plan Reference
//! - 22 behaviors identified
//! - 30 test functions (6 proptest, 14 integration, 8 unit, 2 error taxonomy)
//! - 14 scenarios executable immediately
//! - 8 PRE-INTEGRATION scenarios blocked on GAPs (marked `#[ignore]`)
//!
//! ## GAP Context
//!
//! | Gap | Artifact | Status |
//! |-----|----------|--------|
//! | GAP-1 | `CommandFlags` struct with `validate()`, `valid_mask()`, `as_u16()` | Not implemented |
//! | GAP-2 | `IpcError::InvalidCommandFlags { command, flags }` variant | Not implemented |
//! | GAP-3 | `IpcError::ReservedBitsSet { command, actual, reserved_mask }` variant | Not implemented |
//! | GAP-4 | Diagnostic codes `0x300F`, `0x3010` | Not implemented |
//! | GAP-5 | Flag validation wired into `IpcFrameHeader::decode()` | Not implemented |
//! | GAP-6 | Exhaustive match arm updates for new error variants | Not implemented |
//!
//! Tests marked `#[ignore = "GAP-X"]` are blocked until the corresponding
//! production artifact is implemented.

use std::num::NonZeroUsize;
#[allow(unused_imports)]
use vb_core::DiagnosticCode;
use vb_ipc::{
    IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes,
};

/// Default max payload bytes in u32. Mirrors `MaxPayloadBytes::DEFAULT` (1 MiB)
/// which has a `pub(crate)` getter inaccessible from workspace tests.
const DEFAULT_MAX_PAYLOAD: u32 = 1_048_576;

// ============================================================================
// Reference Model — Contract §2.1, §2.2
// ============================================================================
//
// These values mirror the Kani reference model in kani_flag_validation.rs
// and the domain contract in contract.md §2.2.
//
// TODO(vb-39jp, GAP-1): Replace with `CommandFlags::valid_mask()` and
// `CommandFlags::reserved_global_mask()` when production CommandFlags exists.

/// Global reserved flag mask (bits 8-15). All commands must reject flags
/// with any bit in this range set.
const RESERVED_GLOBAL_MASK: u16 = 0xFF00;

/// Contract-defined valid flag mask per command (§2.2 upper bounds).
///
/// These are the maximum allowed masks. The production implementation may
/// use subset masks. Contract invariant INV-6: ∀ C: valid_mask(C) & 0xFF00 == 0.
const fn valid_mask_ref(command: IpcCommand) -> u16 {
    match command {
        IpcCommand::SubmitRun => 0x00FF,
        IpcCommand::SubmitRunInline => 0x00FF,
        IpcCommand::CancelRun => 0x0000,
        IpcCommand::InspectRun => 0x0003,
        IpcCommand::ListEvents => 0x00FF,
        IpcCommand::AnswerAsk => 0x0000,
        IpcCommand::CompleteAction => 0x0000,
        IpcCommand::FailAction => 0x0000,
        IpcCommand::DrainTrace => 0x0007,
        IpcCommand::Health => 0x0000,
        IpcCommand::Shutdown => 0x0000,
        // Future commands: no flags (mask=0) by default
        _ => 0x0000,
    }
}

/// All 11 IPC v1 commands in declaration order.
const ALL_COMMANDS: [IpcCommand; 11] = [
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

/// Commands with a zero valid mask (accept no flags). Contract §2.2.
const ZERO_MASK_COMMANDS: [IpcCommand; 6] = [
    IpcCommand::CancelRun,
    IpcCommand::AnswerAsk,
    IpcCommand::CompleteAction,
    IpcCommand::FailAction,
    IpcCommand::Health,
    IpcCommand::Shutdown,
];

/// Commands with a non-zero valid mask (accept flags). Contract §2.2.
const NONZERO_MASK_COMMANDS: [IpcCommand; 5] = [
    IpcCommand::SubmitRun,
    IpcCommand::SubmitRunInline,
    IpcCommand::InspectRun,
    IpcCommand::ListEvents,
    IpcCommand::DrainTrace,
];

// ============================================================================
// Helper: build raw 24-byte header manually
// ============================================================================

/// Builds a raw 24-byte IPC header from individual fields.
/// This bypasses `encode()` for testing decode-specific error paths
/// where we need precise control over every byte (e.g., reserved field
/// non-zero, unknown command IDs, invalid flags).
fn raw_header_bytes(
    magic: u32,
    version: u16,
    command: u16,
    flags: u16,
    reserved: u16,
    correlation: u64,
    payload_len: u32,
) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
    let mut bytes = Vec::with_capacity(IPC_HEADER_LEN);
    bytes.extend_from_slice(&magic.to_le_bytes());
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&command.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&reserved.to_le_bytes());
    bytes.extend_from_slice(&correlation.to_le_bytes());
    bytes.extend_from_slice(&payload_len.to_le_bytes());

    <[u8; IPC_HEADER_LEN]>::try_from(bytes.as_slice()).map_err(|_| IpcError::HeaderEncodeFailed)
}

/// Builds a valid raw header with specified flags for a given command.
fn raw_header_with_flags(command: IpcCommand, flags: u16, reserved: u16) -> [u8; IPC_HEADER_LEN] {
    raw_header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        command.as_u16(),
        flags,
        reserved,
        0,
        0,
    )
    .expect("raw_header_bytes must succeed for valid inputs")
}

/// Builds a valid raw header for a command with given command_id and flags.
/// SEC-01: the 5th parameter (`reserved`) is now the caller-capabilities
/// envelope. Use a non-zero value (e.g. ROOT = 0x0001) to avoid the
/// `PermissionDenied` missing-capability sentinel.
fn raw_header_cmd_id(command_id: u16, flags: u16) -> [u8; IPC_HEADER_LEN] {
    raw_header_bytes(IPC_MAGIC, IPC_VERSION, command_id, flags, 0x0001, 0, 0)
        .expect("raw_header_bytes must succeed for valid inputs")
}

// ============================================================================
// Section A: Proptest strategies
// ============================================================================
//
// Strategy helpers are kept inline in proptest functions for locality.
// All random generation uses `rand::rng()` with `random()`/`random_range()`
// from rand 0.9.

// ============================================================================
// Section B: Proptest properties (6)
// ============================================================================

/// PO-VB39JP-001, PO-VB39JP-002, PO-VB39JP-003:
/// For any command and any low-byte flags (current decoder accepts all),
/// encode→decode preserves all 4 header fields bit-exact.
///
/// GAP-5: restrict the random flag word to the per-command valid mask
/// (no reserved bits, no out-of-mask bits) so the decoder accepts the
/// generated header.
#[cfg(not(kani))]
#[test]
fn proptest_header_roundtrip_preserves_all_fields() {
    // Run deterministic random sampling: 1000 cases.
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    // Use a fixed seed for determinism.
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF_CAFE_BABE);

    for _ in 0..1000 {
        // Select random command
        let cmd_idx: usize = rng.random_range(0..11);
        let command = ALL_COMMANDS[cmd_idx];

        // GAP-5: pick a flag value within the command's valid mask only
        // (no reserved bits, no out-of-mask bits).
        let mask = valid_mask_ref(command);
        let flags: u16 = if mask == 0 {
            0x0000
        } else {
            // sample any subset of bits within mask
            let raw: u16 = rng.random_range(0..=mask);
            raw & mask
        };

        // Generate correlation: any u64
        let correlation: u64 = rng.random();

        // Generate payload_len: 0..=DEFAULT
        let payload_len: u32 = rng.random_range(0..=(DEFAULT_MAX_PAYLOAD));

        let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
        let encoded = header.encode().expect("encode must succeed for any header");

        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
            .expect("decode must succeed for any header with flags in operational range");

        assert_eq!(
            decoded.command, command,
            "command mismatch: {:?} vs {:?} for flags={:#06x} corr={} plen={}",
            decoded.command, command, flags, correlation, payload_len
        );
        assert_eq!(
            decoded.flags, flags,
            "flags mismatch: {:#06x} vs {:#06x} for command {:?}",
            decoded.flags, flags, command
        );
        assert_eq!(
            decoded.correlation, correlation,
            "correlation mismatch: {} vs {} for command {:?}",
            decoded.correlation, correlation, command
        );
        assert_eq!(
            decoded.payload_len, payload_len,
            "payload_len mismatch: {} vs {} for command {:?}",
            decoded.payload_len, payload_len, command
        );
    }
}

/// PO-VB39JP-006 through PO-VB39JP-012:
/// For every command, if raw flags have bits outside the command's
/// valid mask or in the reserved global mask, decode returns Err
/// with the correct error variant.
#[test]
fn proptest_decode_rejects_invalid_flags_per_command() {
    // Contract model classification function (inline for clarity).
    fn expected_outcome(command: IpcCommand, raw: u16) -> Result<(), IpcError> {
        if (raw & RESERVED_GLOBAL_MASK) != 0 {
            return Err(IpcError::ReservedBitsSet {
                command,
                actual: raw,
                reserved_mask: RESERVED_GLOBAL_MASK,
            });
        }
        let mask = valid_mask_ref(command);
        if (raw & !mask) != 0 {
            return Err(IpcError::InvalidCommandFlags {
                command,
                flags: raw,
            });
        }
        Ok(())
    }

    // Enumerate all 11 commands × sample flag values; assert against the
    // production decoder's actual return value.
    for command in &ALL_COMMANDS {
        for &test_flag in &[0x0000_u16, 0x0001, 0x00FF, 0x0100, 0xFF00, 0xFF01, 0xFFFF] {
            let header = IpcFrameHeader::new(*command, test_flag, 0, 0);
            let encoded = header.encode().expect("encode must succeed for any flags");
            let actual = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
            let expected_err = expected_outcome(*command, test_flag);
            match expected_err {
                Ok(()) => assert!(
                    actual.is_ok(),
                    "decode must accept flags={:#06x} for {:?}; got {:?}",
                    test_flag,
                    command,
                    actual
                ),
                Err(expected) => assert_eq!(
                    actual,
                    Err(expected),
                    "decode outcome mismatch for {:?} flags={:#06x}",
                    command,
                    test_flag
                ),
            }
        }
    }
}

/// PO-VB39JP-020, PO-VB39JP-021:
/// For any valid header, two encode→decode cycles produce identical
/// decoded headers (idempotent roundtrip).
#[test]
fn proptest_idempotent_header_encode_decode_roundtrip() {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(0xCAFE_FADE_FEED_FACE);

    for _ in 0..500 {
        let cmd_idx: usize = rng.random_range(0..11);
        let command = ALL_COMMANDS[cmd_idx];
        // GAP-5: restrict flags to the command's valid mask so decode accepts.
        let mask = valid_mask_ref(command);
        let flags: u16 = if mask == 0 {
            0x0000
        } else {
            let raw: u16 = rng.random_range(0..=mask);
            raw & mask
        };
        let correlation: u64 = rng.random();
        let payload_len: u32 = rng.random_range(0..=(DEFAULT_MAX_PAYLOAD));

        let header = IpcFrameHeader::new(command, flags, correlation, payload_len);

        // First encode → decode
        let encoded1 = header.encode().expect("encode1 failed");
        let decoded1 =
            IpcFrameHeader::decode(&encoded1, MaxPayloadBytes::DEFAULT).expect("decode1 failed");

        // Second encode → decode from the first decoded header
        let encoded2 = decoded1.encode().expect("encode2 failed");
        let decoded2 =
            IpcFrameHeader::decode(&encoded2, MaxPayloadBytes::DEFAULT).expect("decode2 failed");

        assert_eq!(
            decoded1, decoded2,
            "idempotent roundtrip failed: first decode {:?} != second decode {:?}",
            decoded1, decoded2
        );
    }
}

/// PO-VB39JP-019:
/// `IpcFrameHeader::encode()` always returns `Ok` for any structurally
/// valid header, even with flags=0xFFFF (reserved + invalid bits set).
/// This verifies the trust boundary: encode does not reject, decode does.
#[test]
fn proptest_encode_succeeds_for_any_flags_value() {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(0xFEED_FACE_CAFE_BABE);

    for _ in 0..500 {
        let cmd_idx: usize = rng.random_range(0..11);
        let command = ALL_COMMANDS[cmd_idx];
        // Generate ANY u16 flags value — full range including reserved bits
        let flags: u16 = rng.random();
        let correlation: u64 = rng.random();
        let payload_len: u32 = rng.random_range(0..=(DEFAULT_MAX_PAYLOAD));

        let header = IpcFrameHeader::new(command, flags, correlation, payload_len);

        // Encode must never fail for flags-only invalidity.
        let encoded = header.encode();
        assert!(
            encoded.is_ok(),
            "encode must succeed for any flags value: command={:?} flags={:#06x}",
            command,
            flags
        );

        let bytes = encoded.expect("just checked is_ok");

        // Verify the flag bytes at offset 8..10 match the original flags.
        let encoded_flags = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(
            encoded_flags, flags,
            "encoded flags at offset 8..10 must match input: expected {:#06x}, got {:#06x}",
            flags, encoded_flags
        );
    }
}

/// PO-VB39JP-015:
/// For every command, `valid_mask(C) & reserved_global_mask == 0`.
/// This is an enumeration over all 11 commands, not random generation.
#[test]
fn proptest_valid_mask_is_disjoint_from_reserved_global_mask() {
    for command in &ALL_COMMANDS {
        let mask = valid_mask_ref(*command);
        let intersection = mask & RESERVED_GLOBAL_MASK;
        assert_eq!(
            intersection, 0,
            "valid_mask({:?})={:#06x} must be disjoint from reserved_global_mask={:#06x}, got intersection={:#06x}",
            command, mask, RESERVED_GLOBAL_MASK, intersection
        );
    }
}

/// PO-VB39JP-022:
/// Precedence test for decode error reporting.
///
/// SEC-01 transition: the wire slot at offset 10..12 is now the
/// caller-capabilities envelope (zero is the missing-capability
/// sentinel). The decoder returns `PermissionDenied` when this slot is
/// zero. When the slot is non-zero, decoding proceeds to flag
/// validation: reserved bits beat invalid command flags.
///
/// This test verifies the post-SEC-01 precedence: zero capabilities
/// (PermissionDenied) fires BEFORE flag validation, but valid
/// capabilities with reserved flags fire ReservedBitsSet.
#[test]
fn decode_precedence_capabilities_envelope_then_flag_validation() {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(0xBEEF_DEAD_FADE_CAFE);

    for _ in 0..200 {
        let cmd_idx: usize = rng.random_range(0..11);
        let command = ALL_COMMANDS[cmd_idx];

        // Flags with at least one invalid aspect (reserved bit or outside mask).
        let flags: u16 = loop {
            let f: u16 = rng.random();
            if (f & RESERVED_GLOBAL_MASK) != 0 || (f & !valid_mask_ref(command)) != 0 {
                break f;
            }
        };

        // SEC-01: zero capabilities is the missing-capability sentinel —
        // PermissionDenied must fire BEFORE flag validation.
        let header_bytes = raw_header_with_flags(command, flags, 0x0000);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PermissionDenied),
            "PermissionDenied must take precedence over flag errors: command={:?} flags={:#06x}",
            command,
            flags
        );

        // Non-zero capabilities with reserved flags: ReservedBitsSet fires.
        let header_bytes = raw_header_with_flags(command, flags, 0x0001);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        if (flags & RESERVED_GLOBAL_MASK) != 0 {
            assert_eq!(
                result,
                Err(IpcError::ReservedBitsSet {
                    command,
                    actual: flags,
                    reserved_mask: RESERVED_GLOBAL_MASK,
                }),
                "ReservedBitsSet must surface for flags={:#06x} on command={:?}",
                flags,
                command
            );
        } else {
            assert_eq!(
                result,
                Err(IpcError::InvalidCommandFlags { command, flags }),
                "InvalidCommandFlags must surface for flags={:#06x} on command={:?}",
                flags,
                command
            );
        }
    }
}

// ============================================================================
// Section C: Flag validation unit tests (8 functions)
// ============================================================================

/// B-FLAG-001: CommandFlags::validate returns Ok(CommandFlags(0)) when
/// raw flags equal zero for any command.
#[test]
fn validate_zero_flags_succeeds_for_all_11_commands() {
    for command in &ALL_COMMANDS {
        let result = vb_ipc::CommandFlags::validate(*command, 0x0000);
        assert!(
            result.is_ok(),
            "zero flags should be valid for {:?}",
            command
        );
        let flags = result.expect("zero flags must validate");
        assert_eq!(flags.as_u16(), 0);
    }
}

/// B-FLAG-002: CommandFlags::validate returns Ok when flags are within
/// the command's valid mask and no reserved bits.
#[test]
fn validate_accepts_flags_within_valid_mask_for_each_command() {
    for command in &NONZERO_MASK_COMMANDS {
        let mask = valid_mask_ref(*command);
        let test_flags: Vec<u16> = if mask >= 0x00FF {
            vec![0x0001_u16, 0x00FF, 0x0055]
        } else if mask > 0 {
            vec![0x0001_u16, mask]
        } else {
            vec![0x0000_u16]
        };
        for &f in &test_flags {
            let result = vb_ipc::CommandFlags::validate(*command, f);
            assert!(
                result.is_ok(),
                "validate({:?}, {:#06x}) must succeed; got {:?}",
                command,
                f,
                result
            );
        }
    }
}

/// B-FLAG-003: CommandFlags::validate returns Err(ReservedBitsSet) when
/// raw flags contain any bit in the global reserved mask (0xFF00),
/// regardless of command.
#[test]
fn validate_returns_reserved_bits_set_for_all_11_commands() {
    for command in &ALL_COMMANDS {
        for &reserved_flag in &[0x0100_u16, 0x8000, 0xFF00, 0x0F00] {
            let result = vb_ipc::CommandFlags::validate(*command, reserved_flag);
            assert_eq!(
                result,
                Err(IpcError::ReservedBitsSet {
                    command: *command,
                    actual: reserved_flag,
                    reserved_mask: RESERVED_GLOBAL_MASK,
                }),
                "validate({:?}, {:#06x}) must return ReservedBitsSet",
                command,
                reserved_flag
            );
        }
    }
}

/// B-FLAG-004: CommandFlags::validate returns Err(InvalidCommandFlags)
/// when raw flags contain bits outside the command's valid mask but no
/// reserved bits.
#[test]
fn validate_returns_invalid_command_flags_when_flags_outside_valid_mask() {
    for command in &ALL_COMMANDS {
        let mask = valid_mask_ref(*command);
        let invalid_flags: Vec<u16> = if mask == 0 {
            vec![0x0001_u16, 0x0002, 0x00FF, 0x0055]
        } else if mask == 0x0001 {
            vec![0x0002_u16]
        } else if mask == 0x0003 {
            vec![0x0004_u16]
        } else if mask == 0x0007 {
            vec![0x0008_u16]
        } else {
            // mask == 0x00FF: no invalid low-byte flags exist
            vec![]
        };
        for &f in &invalid_flags {
            let result = vb_ipc::CommandFlags::validate(*command, f);
            assert_eq!(
                result,
                Err(IpcError::InvalidCommandFlags {
                    command: *command,
                    flags: f,
                }),
                "validate({:?}, {:#06x}) must return InvalidCommandFlags",
                command,
                f
            );
        }
    }
}

/// B-FLAG-005: Reserved-bit check takes precedence over valid-mask check.
/// Any bit in 0xFF00 → ReservedBitsSet, even if other low-byte bits are
/// also invalid per the command's mask.
#[test]
fn validate_returns_reserved_bits_set_not_invalid_command_flags_when_both_conditions_apply() {
    for command in &ALL_COMMANDS {
        let raw = 0xFF01_u16;
        let result = vb_ipc::CommandFlags::validate(*command, raw);
        assert_eq!(
            result,
            Err(IpcError::ReservedBitsSet {
                command: *command,
                actual: raw,
                reserved_mask: RESERVED_GLOBAL_MASK,
            }),
            "validate({:?}, 0xFF01) must return ReservedBitsSet (precedence over InvalidCommandFlags)",
            command
        );
    }
}

/// B-FLAG-006: Commands with valid_mask == 0x0000 reject ANY non-zero
/// flag value as InvalidCommandFlags (when no reserved bits are set).
#[test]
fn validate_rejects_every_nonzero_flag_for_zero_mask_commands() {
    for command in &ZERO_MASK_COMMANDS {
        assert_eq!(
            valid_mask_ref(*command),
            0,
            "zero-mask command {:?} must have valid_mask=0",
            command
        );
        for &flag in &[0x0001_u16, 0x0002, 0x00FF, 0x0055] {
            let result = vb_ipc::CommandFlags::validate(*command, flag);
            assert_eq!(
                result,
                Err(IpcError::InvalidCommandFlags {
                    command: *command,
                    flags: flag,
                }),
                "validate({:?}, {:#06x}) must return InvalidCommandFlags",
                command,
                flag
            );
        }
    }
}

/// B-FLAG-007: valid_mask(command) & reserved_global_mask == 0 for
/// every command (masks are disjoint). INV-6 from contract §1.5.
#[test]
fn valid_mask_and_reserved_global_mask_are_disjoint_for_all_commands() {
    for command in &ALL_COMMANDS {
        let mask = valid_mask_ref(*command);
        let intersection = mask & RESERVED_GLOBAL_MASK;
        assert_eq!(
            intersection, 0,
            "INV-6 violation: valid_mask({:?})={:#06x} & reserved_global_mask={:#06x} = {:#06x} (expected 0)",
            command, mask, RESERVED_GLOBAL_MASK, intersection
        );
    }
}

/// B-FLAG-008: CommandFlags::as_u16() returns the exact raw flags value
/// passed to a successful validate() call.
#[test]
fn command_flags_as_u16_returns_the_validated_value() {
    let flags = vb_ipc::CommandFlags::validate(IpcCommand::SubmitRunInline, 0x00FF)
        .expect("SubmitRunInline accepts 0x00FF");
    assert_eq!(flags.as_u16(), 0x00FF);

    let zero = vb_ipc::CommandFlags::validate(IpcCommand::Health, 0).expect("Health accepts zero");
    assert_eq!(zero.as_u16(), 0);
}

// ============================================================================
// Section D: Error taxonomy unit tests (2 functions)
// ============================================================================

/// B-ERROR-001: IpcError::InvalidCommandFlags returns diagnostic code
/// 0x300F, the Display impl contains "invalid flags", the command
/// name, and the flags value.
#[test]
fn invalid_command_flags_error_returns_diagnostic_code_0x300_f() {
    let err = IpcError::InvalidCommandFlags {
        command: IpcCommand::Health,
        flags: 0x0001,
    };
    assert_eq!(err.diagnostic_code(), DiagnosticCode::new(0x300F));
    let msg = format!("{err}");
    assert!(
        msg.contains("invalid flags"),
        "message must contain 'invalid flags': {msg}"
    );
    assert!(
        msg.contains("Health"),
        "message must contain command name: {msg}"
    );
    assert!(
        msg.contains("0x0001"),
        "message must contain flags value: {msg}"
    );
    assert_eq!(
        err.runtime_code(),
        Some(IpcError::IPC_FRAME_INVALID_RUNTIME_CODE)
    );
}

/// B-ERROR-002: IpcError::ReservedBitsSet returns diagnostic code
/// 0x3010, the Display impl contains "reserved", the command name,
/// the actual value, and the mask.
#[test]
fn reserved_bits_set_error_returns_diagnostic_code_0x3010() {
    let err = IpcError::ReservedBitsSet {
        command: IpcCommand::Shutdown,
        actual: 0x8000,
        reserved_mask: 0xFF00,
    };
    assert_eq!(err.diagnostic_code(), DiagnosticCode::new(0x3010));
    let msg = format!("{err}");
    assert!(
        msg.contains("reserved"),
        "message must contain 'reserved': {msg}"
    );
    assert!(
        msg.contains("Shutdown"),
        "message must contain command name: {msg}"
    );
    assert!(
        msg.contains("0x8000"),
        "message must contain actual value: {msg}"
    );
    assert!(
        msg.contains("0xff00"),
        "message must contain reserved_mask: {msg}"
    );
    assert_eq!(
        err.runtime_code(),
        Some(IpcError::IPC_FRAME_INVALID_RUNTIME_CODE)
    );
}

// ============================================================================
// Section E: Header roundtrip integration tests (8 scenarios, 9 functions)
// ============================================================================

/// B-ROUNDTRIP-001:
/// IpcFrameHeader roundtrip preserves all fields when encoding a Health
/// command with zero flags and zero payload, then decoding.
#[test]
fn health_command_header_with_zero_payload_roundtrips() {
    // Given: a frame with command=Health, flags=0x0000, correlation=42, payload_len=0
    let header = IpcFrameHeader::new(IpcCommand::Health, 0x0000, 42, 0);

    // When: the frame is encoded to 24 bytes and decoded back
    let encoded = header
        .encode()
        .expect("encode must succeed for Health header");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
        .expect("decode must succeed for valid Health header");

    // Then: all fields are preserved exactly
    assert_eq!(decoded.command, IpcCommand::Health);
    assert_eq!(decoded.flags, 0x0000);
    assert_eq!(decoded.correlation, 42);
    assert_eq!(decoded.payload_len, 0);
}

/// B-ROUNDTRIP-002:
/// IpcFrameHeader roundtrip preserves all fields when encoding a
/// SubmitRunInline command with valid flags at mask boundary and
/// payload length at max bound.
#[test]
fn submit_run_inline_command_header_with_payload_len_at_bound_roundtrips() {
    // Given: SubmitRunInline, flags=0x00FF (valid mask max), correlation=u64::MAX,
    //        payload_len=MaxPayloadBytes::DEFAULT
    let max_payload = DEFAULT_MAX_PAYLOAD;
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0x00FF, u64::MAX, max_payload);

    // When
    let encoded = header.encode().expect("encode must succeed");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
        .expect("decode must succeed for valid header at bounds");

    // Then
    assert_eq!(decoded.command, IpcCommand::SubmitRunInline);
    assert_eq!(decoded.flags, 0x00FF);
    assert_eq!(decoded.correlation, u64::MAX);
    assert_eq!(decoded.payload_len, max_payload);
}

/// B-ROUNDTRIP-003:
/// All 11 IPC command variants roundtrip through encode→decode with
/// command-appropriate valid flags. Zero-mask commands use flags=0x0000.
/// Non-zero-mask commands use their maximum valid flag value.
#[test]
fn all_11_ipc_commands_roundtrip_header_with_valid_flags() {
    for command in &ALL_COMMANDS {
        let mask = valid_mask_ref(*command);
        // Use the maximum valid flag for this command (or 0 for zero-mask)
        let flags: u16 = if mask > 0 { mask } else { 0x0000 };
        let correlation = command.as_u16() as u64;

        let header = IpcFrameHeader::new(*command, flags, correlation, 0);

        let encoded = header.encode().expect("encode must succeed");
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
            .expect("decode must succeed for valid headers");

        assert_eq!(
            decoded.command, *command,
            "command mismatch for {:?} with flags={:#06x}",
            command, flags
        );
        assert_eq!(
            decoded.flags, flags,
            "flags mismatch for {:?}: expected {:#06x}, got {:#06x}",
            command, flags, decoded.flags
        );
        assert_eq!(
            decoded.correlation, correlation,
            "correlation mismatch for {:?}",
            command
        );
        assert_eq!(
            decoded.payload_len, 0,
            "payload_len mismatch for {:?}",
            command
        );
    }
}

/// B-ROUNDTRIP-004:
/// Header flag bits roundtrip unchanged at extreme values (0x0000,
/// minimum non-zero, and maximum) for each command that accepts flags.
#[test]
fn header_flag_bits_roundtrip_at_zero_min_and_max_for_each_command() {
    for command in &NONZERO_MASK_COMMANDS {
        let mask = valid_mask_ref(*command);
        let test_flags: Vec<u16> = {
            let mut v = vec![0x0000_u16];
            if mask > 0 {
                v.push(0x0001_u16); // minimum non-zero
                v.push(mask); // maximum allowed
            }
            v
        };

        for &flags in &test_flags {
            let header = IpcFrameHeader::new(*command, flags, 0, 0);
            let encoded = header.encode().expect("encode must succeed");
            let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
                .expect("decode must succeed for valid header");

            assert_eq!(
                decoded.flags, flags,
                "flag bit identity lost for {:?}: expected {:#06x}, got {:#06x}",
                command, flags, decoded.flags
            );
            assert_eq!(
                decoded.command, *command,
                "command mismatch for {:?} with flags={:#06x}",
                command, flags
            );
        }
    }
}

/// B-ROUNDTRIP-005:
/// encode() does not reject any flags value, even those that would be
/// rejected at decode (trust boundary). Tests that flags=0xFFFF
/// (all bits set) encodes successfully and places the correct bytes
/// at offset 8..10 in little-endian.
#[test]
fn encode_accepts_all_flags_bits_even_those_rejected_at_decode() {
    // Given: a header with command=Health, flags=0xFFFF (all bits set,
    //        which would be rejected at decode once flag validation exists)
    let header = IpcFrameHeader::new(IpcCommand::Health, 0xFFFF, 0, 0);

    // When: encode() is called
    let encoded = header.encode();

    // Then: encode succeeds
    assert!(
        encoded.is_ok(),
        "encode must succeed even with flags=0xFFFF"
    );

    let bytes = encoded.expect("just checked is_ok");

    // And: the encoded bytes at offset 8..10 contain 0xFFFF in little-endian
    assert_eq!(
        bytes.len(),
        IPC_HEADER_LEN,
        "encoded header must be 24 bytes"
    );
    let actual_flags = u16::from_le_bytes([bytes[8], bytes[9]]);
    assert_eq!(
        actual_flags, 0xFFFF,
        "encoded flags at offset 8..10 must be 0xFFFF LE, got {:#06x}",
        actual_flags
    );
}

/// B-ROUNDTRIP-006:
/// IpcFrameHeader decodes successfully when payload length is zero for
/// commands that allow zero payload (Health, Shutdown).
#[test]
fn decode_accepts_payload_len_zero_for_commands_that_allow_it() {
    let test_commands = [IpcCommand::Health, IpcCommand::Shutdown];

    for &command in &test_commands {
        let header = IpcFrameHeader::new(command, 0x0000, 1, 0);
        let encoded = header.encode().expect("encode must succeed");
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
            .expect("decode must succeed for valid header with zero payload");

        assert_eq!(
            decoded.command, command,
            "command mismatch for {:?}",
            command
        );
        assert_eq!(
            decoded.payload_len, 0,
            "payload_len must be 0 for {:?}",
            command
        );
        assert_eq!(
            decoded.flags, 0x0000,
            "flags must be 0x0000 for {:?}",
            command
        );
    }
}

/// B-ROUNDTRIP-007:
/// IpcFrameHeader decodes successfully when payload length equals
/// MaxPayloadBytes::DEFAULT (boundary value).
#[test]
fn decode_accepts_payload_len_equal_to_max_bound() {
    let max_payload = DEFAULT_MAX_PAYLOAD;

    // Given: SubmitRunInline with payload_len = MaxPayloadBytes::DEFAULT
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0x0000, 1, max_payload);

    let encoded = header.encode().expect("encode must succeed");

    // When: decoding
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    // Then: decode succeeds with exact payload_len
    assert!(
        decoded.is_ok(),
        "decode must succeed for payload_len={} (MaxPayloadBytes::DEFAULT)",
        max_payload
    );
    let decoded = decoded.expect("just checked is_ok");
    assert_eq!(
        decoded.payload_len, max_payload,
        "payload_len must equal max bound: expected {}, got {}",
        max_payload, decoded.payload_len
    );
    assert_eq!(decoded.command, IpcCommand::SubmitRunInline);
}

/// B-ROUNDTRIP-008:
/// Header encode→decode is idempotent: two cycles produce identical
/// headers. Tested for all 11 commands with valid flags.
#[test]
fn header_encode_decode_is_idempotent_for_valid_headers() {
    for command in &ALL_COMMANDS {
        let mask = valid_mask_ref(*command);
        let flags: u16 = if mask > 0 { mask } else { 0x0000 };
        let correlation = (command.as_u16() as u64) * 7;

        let header = IpcFrameHeader::new(*command, flags, correlation, 0);

        // Cycle 1
        let enc1 = header.encode().expect("encode1 failed");
        let dec1 = IpcFrameHeader::decode(&enc1, MaxPayloadBytes::DEFAULT).expect("decode1 failed");

        // Cycle 2 — re-encode the decoded header and decode again
        let enc2 = dec1.encode().expect("encode2 failed");
        let dec2 = IpcFrameHeader::decode(&enc2, MaxPayloadBytes::DEFAULT).expect("decode2 failed");

        assert_eq!(
            dec1, dec2,
            "idempotent roundtrip failed for {:?}: first={:?}, second={:?}",
            command, dec1, dec2
        );
    }
}

/// Regression: explicit test that flags=0xFFFF now REJECTS at decode
/// after GAP-5 (the reserved-bits check fires). Previously (pre-GAP-5)
/// this roundtripped successfully; the post-GAP-5 behavior is the
/// contract-correct one.
#[test]
fn decode_rejects_flags_ffff_with_reserved_bits_set_after_gap5() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0xFFFF, 99, 0);
    let encoded = header.encode().expect("encode must succeed");
    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::ReservedBitsSet {
            command: IpcCommand::Health,
            actual: 0xFFFF,
            reserved_mask: RESERVED_GLOBAL_MASK,
        }),
        "decode must reject 0xFFFF with ReservedBitsSet after GAP-5"
    );
}

// ============================================================================
// Section F: Decode rejection integration tests (2 functions)
// ============================================================================

/// B-DECODE-001:
/// IpcFrameHeader::decode() returns Err(InvalidCommandFlags) when header
/// is structurally valid but flags are outside the command's valid mask.
/// Example: CompleteAction (zero-mask) with flags=0x0001.
#[test]
fn decode_returns_invalid_command_flags_when_complete_action_has_flag_bit_set() {
    for command in &ZERO_MASK_COMMANDS {
        let header_bytes = raw_header_with_flags(*command, 0x0001, 0x0001);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::InvalidCommandFlags {
                command: *command,
                flags: 0x0001,
            }),
            "decode must reject flags=0x0001 for zero-mask command {command:?}"
        );
    }
}

/// B-DECODE-002:
/// IpcFrameHeader::decode() returns Err(ReservedBitsSet) when header is
/// structurally valid but flags have reserved bits set.
/// Example: Health with flags=0x0100 (bit in reserved high byte).
#[test]
fn decode_returns_reserved_bits_set_when_health_has_high_byte_flag_set() {
    let reserved_test_flags: [u16; 4] = [0x0100, 0x8000, 0xFF00, 0x1100];
    for command in &ALL_COMMANDS {
        for &flags in &reserved_test_flags {
            let header_bytes = raw_header_with_flags(*command, flags, 0x0001);
            let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
            assert_eq!(
                result,
                Err(IpcError::ReservedBitsSet {
                    command: *command,
                    actual: flags,
                    reserved_mask: RESERVED_GLOBAL_MASK,
                }),
                "decode must reject flags={flags:#06x} with ReservedBitsSet for {command:?}"
            );
        }
    }
}

// ============================================================================
// Section G: Decode error precedence tests (3 functions)
// ============================================================================

/// B-DECODE-003:
/// Flag validation errors occur at the correct precedence position in
/// decode: after magic, version, command, capabilities envelope, payload
/// bounds; before returning Ok. When the capabilities envelope is the
/// zero sentinel AND flags are invalid, `PermissionDenied` is returned
/// — NOT a flag error.
///
/// SEC-01 transition: the wire slot at offset 10..12 is the
/// caller-capabilities envelope (zero is the missing-capability
/// sentinel). This test verifies the post-SEC-01 precedence: zero
/// capabilities (PermissionDenied) fires BEFORE flag validation.
#[test]
fn decode_returns_permission_denied_not_flag_error_when_capabilities_zero_and_flags_invalid() {
    // Given: a header with capabilities_envelope = 0 (the sentinel) and
    //        flags that would be rejected if flag check ran first
    let header_bytes = raw_header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0x0100, // flags with reserved bit set
        0x0000, // capabilities envelope = 0 (sentinel)
        42,
        0,
    )
    .expect("raw_header_bytes must succeed");

    // When: decode() is called
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    // Then: PermissionDenied is returned, NOT a flag error
    assert_eq!(
        result,
        Err(IpcError::PermissionDenied),
        "PermissionDenied (capabilities=0) must take precedence over flag errors"
    );
}

/// B-DECODE-004:
/// IpcFrameHeader::decode() preserves UnknownCommand(n) for command
/// IDs 0 and 12..=65535. Downstream dispatch rejects the typed command.
#[test]
fn decode_returns_unknown_command_for_command_ids_0_and_above_11() {
    // Command ID 0
    {
        let header_bytes = raw_header_cmd_id(0, 0x0000);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(0), 0, 0, 0,)),
            "command ID 0 must preserve UnknownCommand(0)"
        );
    }

    // Command ID 17 (first above valid range)
    {
        let header_bytes = raw_header_cmd_id(17, 0x0000);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(17), 0, 0, 0,)),
            "command ID 17 must preserve UnknownCommand(17)"
        );
    }

    // Command ID 65535 (u16::MAX)
    {
        let header_bytes = raw_header_cmd_id(65535, 0x0000);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Ok(IpcFrameHeader::new(
                IpcCommand::UnknownCommand(65535),
                0,
                0,
                0,
            )),
            "command ID 65535 must preserve UnknownCommand(65535)"
        );
    }

    // Spot-check some intermediate values
    for &cmd_id in &[100_u16, 256, 1000, 16384] {
        let header_bytes = raw_header_cmd_id(cmd_id, 0x0000);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Ok(IpcFrameHeader::new(
                IpcCommand::UnknownCommand(cmd_id),
                0,
                0,
                0,
            )),
            "command ID {} must preserve UnknownCommand({})",
            cmd_id,
            cmd_id
        );
    }
}

/// B-DECODE-005:
/// The two wire-related error sources for the SEC-01 envelope and
/// flag-reserved bits must be distinguishable:
///
///   - `PermissionDenied` is returned when the capabilities envelope
///     (offset 10..12) is zero — the missing-capability sentinel.
///   - `ReservedBitsSet` is returned when flags (offset 8..10) carry
///     reserved bits (mask 0xFF00) AND the capabilities envelope is
///     non-zero.
///
/// This test verifies the post-SEC-01 behavior end-to-end.
#[test]
fn decode_distinguishes_capabilities_envelope_from_reserved_bits_in_flags() {
    // Test A: capabilities envelope = 0x00FF (non-zero, ROOT+more) and
    // flags = 0 → Ok. A non-zero capabilities envelope is valid; only
    // the zero sentinel triggers PermissionDenied.
    {
        let header_bytes = raw_header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::Health.as_u16(),
            0x0000, // flags clean
            0x00FF, // capabilities envelope: non-zero, valid
            1,
            0,
        )
        .expect("raw_header_bytes must succeed");

        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert!(
            result.is_ok(),
            "non-zero capabilities envelope must decode; got {:?}",
            result
        );
    }

    // Test A2: capabilities envelope = 0 → PermissionDenied
    {
        let header_bytes = raw_header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::Health.as_u16(),
            0x0000,
            0x0000, // capabilities envelope = 0 → PermissionDenied sentinel
            1,
            0,
        )
        .expect("raw_header_bytes must succeed");

        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PermissionDenied),
            "zero capabilities envelope must produce PermissionDenied (SEC-01 sentinel)"
        );
    }

    // Test B: capabilities envelope = non-zero, flags with reserved bits
    // → ReservedBitsSet.
    {
        let header_bytes = raw_header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::Health.as_u16(),
            0x0100, // flags with reserved bit (bit 8)
            0x0001, // capabilities envelope = ROOT
            1,
            0,
        )
        .expect("raw_header_bytes must succeed");

        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::ReservedBitsSet {
                command: IpcCommand::Health,
                actual: 0x0100,
                reserved_mask: RESERVED_GLOBAL_MASK,
            }),
            "non-zero capabilities + reserved-bit flags must surface ReservedBitsSet"
        );
    }

    // Test C: clean header (capabilities=ROOT, flags=0) → Ok
    {
        let header_bytes = raw_header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::Health.as_u16(),
            0x0000,
            0x0001,
            1,
            0,
        )
        .expect("raw_header_bytes must succeed");

        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert!(result.is_ok(), "clean header must decode successfully");
        let decoded = result.expect("just checked");
        assert_eq!(decoded.command, IpcCommand::Health);
        assert_eq!(decoded.flags, 0x0000);
    }
}

// ============================================================================
// Section H: Additional regression and boundary tests
// ============================================================================

/// Verify that decode rejects payload_len = MaxPayloadBytes::DEFAULT + 1
/// (off-by-one at upper bound). Tests that boundary enforcement is strict.
#[test]
fn decode_rejects_payload_len_exceeding_max_bound() {
    let max = DEFAULT_MAX_PAYLOAD;
    let oversized = max.saturating_add(1);

    let header_bytes = raw_header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0x0000,
        0x0001,
        1,
        oversized,
    )
    .expect("raw_header_bytes must succeed");

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: oversized as usize,
            limit: max as usize,
        }),
        "decode must reject payload_len={} exceeding max={}",
        oversized,
        max
    );
}

/// Verify that decode rejects payload_len = MaxPayloadBytes::DEFAULT + 1
/// with a smaller MaxPayloadBytes bound (tight bound test).
#[test]
fn decode_rejects_payload_len_with_tight_max_bound() {
    let tight_max = MaxPayloadBytes::new(NonZeroUsize::new(1024).expect("1024 > 0"));
    let oversized: u32 = 1025;

    let header_bytes = raw_header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0x0000,
        0x0001,
        1,
        oversized,
    )
    .expect("raw_header_bytes must succeed");

    let result = IpcFrameHeader::decode(&header_bytes, tight_max);
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 1025,
            limit: 1024,
        }),
        "decode must reject payload_len=1025 exceeding tight max=1024"
    );
}

/// Verify exact payload_len matches when at max bound but not exceeding.
#[test]
fn decode_accepts_payload_len_exactly_at_tight_max_bound() {
    let tight_max = MaxPayloadBytes::new(NonZeroUsize::new(1024).expect("1024 > 0"));

    let header_bytes = raw_header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0x0000,
        0x0001,
        1,
        1024,
    )
    .expect("raw_header_bytes must succeed");

    let result = IpcFrameHeader::decode(&header_bytes, tight_max);
    assert!(
        result.is_ok(),
        "decode must accept payload_len=1024 at tight max=1024"
    );
    let decoded = result.expect("just checked is_ok");
    assert_eq!(decoded.payload_len, 1024);
}

/// Verify that all 11 commands report their expected wire command ID
/// via as_u16(), and that from_u16() roundtrips correctly for 1..=11.
#[test]
fn all_11_commands_have_correct_wire_ids_and_roundtrip_from_u16() {
    for &command in &ALL_COMMANDS {
        let wire_id = command.as_u16();
        assert!(
            (1..=11).contains(&wire_id),
            "command {:?} has wire ID {} outside 1..=11",
            command,
            wire_id
        );

        let roundtripped =
            IpcCommand::from_u16(wire_id).expect("from_u16 must recognize all valid command IDs");
        assert_eq!(
            roundtripped, command,
            "from_u16({}) must return {:?}, got {:?}",
            wire_id, command, roundtripped
        );
    }
}

// ============================================================================
// Section I: Regression reference — existing macro-based roundtrip tests
// ============================================================================
//
// The existing tests at vb_ipc/src/frame/tests.rs use ipc_header_roundtrip_test!
// which passes flags=0xABCD and flags=0xFFFF. After GAP-5 integration, these
// flag values will be rejected.
//
// Test plan §10 documents migration impact:
//   - frame/tests.rs:671 (flags=0xABCD) → replace with command-valid flags
//   - frame/tests.rs:673 (flags=0xFFFF) → replace with command-valid flags
//   - tests.rs:944-954 (flags=0xABCD for SubmitRun) → replace with 0x00FF
//   - tests.rs:1806-1817 (flags=0xABCD for CompleteAction) → replace with 0x0000
//
// This section includes a test that verifies the current behavior (flags=0xABCD
// and 0xFFFF roundtrip) matches the pre-integration state, serving as a
// regression checkpoint.

/// Regression: documents that the existing macro-based roundtrip test
/// values (0xABCD, 0xFFFF) now REJECT at decode after GAP-5. Replace
/// the pre-GAP-5 acceptance assertion with the contract-correct
/// rejection behavior. Downstream tests must be updated to use
/// command-valid flags.
#[test]
fn regression_existing_roundtrip_macro_flag_values_rejected_after_gap5() {
    // These are the flag values previously used in ipc_header_roundtrip_test!.
    let rejected_flags: [u16; 3] = [0x0001, 0xABCD, 0xFFFF];

    for command in &ALL_COMMANDS {
        for &flags in &rejected_flags {
            let header = IpcFrameHeader::new(*command, flags, 0, 0);
            let encoded = header.encode().expect("encode must succeed");
            let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

            // Reserved bits must fire first (precedence: ReservedBitsSet
            // beats InvalidCommandFlags). 0x0001 has no reserved bits and
            // is rejected only for zero-mask commands.
            if (flags & RESERVED_GLOBAL_MASK) != 0 {
                assert_eq!(
                    result,
                    Err(IpcError::ReservedBitsSet {
                        command: *command,
                        actual: flags,
                        reserved_mask: RESERVED_GLOBAL_MASK,
                    }),
                    "decode must reject flags={flags:#06x} with ReservedBitsSet for {command:?}"
                );
            } else if valid_mask_ref(*command) == 0 {
                assert_eq!(
                    result,
                    Err(IpcError::InvalidCommandFlags {
                        command: *command,
                        flags,
                    }),
                    "decode must reject flags={flags:#06x} with InvalidCommandFlags for zero-mask command {command:?}"
                );
            } else {
                // Non-zero-mask command with flags within low byte:
                // accept only flags that are within the mask; for the
                // 0x0001 case on commands with mask>=1 (SubmitRun,
                // SubmitRunInline, ListEvents) this should roundtrip.
                let within_mask = (flags & !valid_mask_ref(*command)) == 0;
                if within_mask {
                    assert!(
                        result.is_ok(),
                        "decode must accept flags={flags:#06x} for {command:?} (within mask)"
                    );
                } else {
                    assert_eq!(
                        result,
                        Err(IpcError::InvalidCommandFlags {
                            command: *command,
                            flags,
                        }),
                        "decode must reject flags={flags:#06x} with InvalidCommandFlags for {command:?}"
                    );
                }
            }
        }
    }
}

/// Regression: CompleteAction with flags=0xABCD now REJECTS at decode
/// after GAP-5 (InvalidCommandFlags for zero-mask commands).
#[test]
fn regression_complete_action_with_flags_0x_abcd_now_rejects() {
    let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0xABCD, 0x1234_5678_9ABC_DEF0, 8);
    let encoded = header.encode().expect("encode must succeed");
    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::ReservedBitsSet {
            command: IpcCommand::CompleteAction,
            actual: 0xABCD,
            reserved_mask: RESERVED_GLOBAL_MASK,
        }),
        "CompleteAction+0xABCD must reject with ReservedBitsSet (reserved bits win precedence)"
    );
}

/// Regression: SubmitRun with flags=0xABCD now REJECTS at decode after
/// GAP-5 (ReservedBitsSet because 0xABCD has bits in 0xFF00).
#[test]
fn regression_submit_run_with_flags_0x_abcd_now_rejects() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0xABCD, 999, 10);
    let encoded = header.encode().expect("encode must succeed");
    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::ReservedBitsSet {
            command: IpcCommand::SubmitRun,
            actual: 0xABCD,
            reserved_mask: RESERVED_GLOBAL_MASK,
        }),
        "SubmitRun+0xABCD must reject with ReservedBitsSet after GAP-5"
    );
}

// ============================================================================
// Section J: Comprehensive flag test matrix — all 11 commands × flag classes
// ============================================================================

/// Combinatorial test: for each of the 11 commands, verify the contract
/// behavior across all flag classes defined in the matrix (§8.1).
/// After GAP-5, only valid flag classes roundtrip; reserved and
/// invalid-low classes must reject with the contract-correct error
/// variants.
#[test]
fn flag_matrix_all_11_commands_all_classes_decode_contract() {
    /// Represents a flag test case in the contract matrix.
    struct FlagCase {
        label: &'static str,
        flags: u16,
    }

    // Flag classes from §8.1:
    // Valid=0, Valid Low, Valid Max, Invalid Low, Reserved, Reserved+Invalid
    let flag_cases = |cmd: IpcCommand| -> Vec<FlagCase> {
        let mask = valid_mask_ref(cmd);
        let mut cases = Vec::new();

        // Class: Flags=0 (always valid)
        cases.push(FlagCase {
            label: "zero",
            flags: 0x0000,
        });

        // Class: Valid Low (non-zero valid flag, if mask > 0)
        if mask > 0 {
            cases.push(FlagCase {
                label: "valid_low",
                flags: 0x0001,
            });
        }

        // Class: Valid Max (if mask > 0 and > 1)
        if mask > 1 {
            cases.push(FlagCase {
                label: "valid_max",
                flags: mask,
            });
        }

        // Class: Invalid Low (bit outside mask, no reserved)
        let invalid_low = if mask == 0 {
            Some(0x0001_u16)
        } else if mask == 0x0001 {
            Some(0x0002_u16)
        } else if mask == 0x0003 {
            Some(0x0004_u16)
        } else if mask == 0x0007 {
            Some(0x0008_u16)
        } else {
            // mask == 0x00FF: every low-byte bit is valid, no invalid_low
            None
        };
        if let Some(f) = invalid_low {
            cases.push(FlagCase {
                label: "invalid_low",
                flags: f,
            });
        }

        // Class: Reserved (bit in high byte, low byte clean)
        cases.push(FlagCase {
            label: "reserved",
            flags: 0x0100,
        });
        cases.push(FlagCase {
            label: "reserved_high",
            flags: 0x8000,
        });

        // Class: Reserved + Invalid (bits in both high byte and outside mask)
        let reserved_invalid = if mask == 0 {
            0xFF01_u16
        } else if mask <= 0x0007 {
            // Small mask: use 0xFF00 | (mask + 1)
            0xFF00_u16 | (mask.saturating_add(1) & 0x00FF)
        } else {
            0xFF00_u16 // mask == 0x00FF: all low bits valid, just reserved
        };
        cases.push(FlagCase {
            label: "reserved_plus_invalid",
            flags: reserved_invalid,
        });

        cases
    };

    /// Classifies a flag test case into the expected decode outcome.
    fn expected_outcome(command: IpcCommand, flags: u16) -> Result<(), IpcError> {
        if (flags & RESERVED_GLOBAL_MASK) != 0 {
            return Err(IpcError::ReservedBitsSet {
                command,
                actual: flags,
                reserved_mask: RESERVED_GLOBAL_MASK,
            });
        }
        if (flags & !valid_mask_ref(command)) != 0 {
            return Err(IpcError::InvalidCommandFlags { command, flags });
        }
        Ok(())
    }

    for command in &ALL_COMMANDS {
        let cases = flag_cases(*command);

        for case in &cases {
            let header = IpcFrameHeader::new(*command, case.flags, 0, 0);
            let encoded = match header.encode() {
                Ok(bytes) => bytes,
                Err(e) => {
                    panic!(
                        "encode failed for {:?} flags={:#06x} label={}: {:?}",
                        command, case.flags, case.label, e
                    );
                }
            };

            let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
            let expected_err = expected_outcome(*command, case.flags);
            match expected_err {
                Ok(()) => assert!(
                    result.is_ok(),
                    "decode must accept flags={:#06x} label={} for {:?}; got {:?}",
                    case.flags,
                    case.label,
                    command,
                    result
                ),
                Err(expected) => assert_eq!(
                    result,
                    Err(expected),
                    "decode outcome mismatch for {:?} flags={:#06x} label={}",
                    command,
                    case.flags,
                    case.label
                ),
            }
        }
    }
}
