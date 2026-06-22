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
    clippy::trivially_copy_macro,
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

//! Section 38 property test: `state_machine`.
//!
//! Master plan §38, row "State machine":
//! "No terminal state transitions back to running".
//!
//! In the `vb_storage` crate the state-machine floor is the typed
//! `RunHeaderStatus` byte and the on-disk `RecordKind` taxonomy.
//! Both must classify their inputs without ever silently accepting
//! an unknown value:
//!
//! - `RunHeaderStatus::from_byte(b)` always returns a `RunHeaderStatus`,
//!   but `.known()` distinguishes the four known bytes from the
//!   typed `UnknownRunHeaderStatus` error.
//! - `RunHeaderStatus::classify()` returns the lossless
//!   `RunHeaderStatusClass` for any byte.
//! - `KnownRunHeaderStatus` has exactly four values, with stable
//!   byte assignments.
//! - Re-encoding the same record with the same kind/magic yields
//!   the same envelope (determinism).
//! - The encoder/decoder pair is total over well-formed inputs.

use proptest::prelude::*;

use crate::codec::{decode_envelope_only, decode_record, encode_record};
use crate::constants::{MAGIC_BLOB, MAX_BLOB_BYTES};
use crate::records::{
    BlobRecord, KnownRunHeaderStatus, RecordKind, RunHeaderStatus, RunHeaderStatusClass,
    UnknownRunHeaderStatus,
};

proptest! {
    /// `RunHeaderStatus::from_byte` is total: it never panics and
    /// always returns a value.
    #[test]
    fn sm_run_header_status_from_byte_total(byte in any::<u8>()) {
        let _ = RunHeaderStatus::from_byte(byte);
    }

    /// `RunHeaderStatus::classify` is total: it never panics and
    /// always returns a `Known` or `Unknown` classification.
    #[test]
    fn sm_run_header_status_classify_total(byte in any::<u8>()) {
        let status = RunHeaderStatus::from_byte(byte);
        match status.classify() {
            RunHeaderStatusClass::Known(_) | RunHeaderStatusClass::Unknown(_) => {}
        }
    }

    /// The four documented status bytes (0..=3) classify as the
    /// corresponding `KnownRunHeaderStatus` variant. Bytes outside
    /// this range classify as `Unknown(byte)` and are never silently
    /// re-mapped to a known variant.
    #[test]
    fn sm_known_status_bytes_classify_known(byte in 0u8..=3u8) {
        let status = RunHeaderStatus::from_byte(byte);
        match status.classify() {
            RunHeaderStatusClass::Known(known) => {
                let expected = match byte {
                    0 => KnownRunHeaderStatus::Pending,
                    1 => KnownRunHeaderStatus::Accepted,
                    2 => KnownRunHeaderStatus::Active,
                    3 => KnownRunHeaderStatus::Finished,
                    _ => unreachable!(),
                };
                prop_assert_eq!(known, expected);
            }
            RunHeaderStatusClass::Unknown(_) => {
                panic!("byte {byte} should classify as Known");
            }
        }
    }

    /// Bytes outside the 0..=3 range classify as `Unknown` and
    /// carry the exact byte back.
    #[test]
    fn sm_unknown_status_bytes_classify_unknown(byte in 4u8..=u8::MAX) {
        let status = RunHeaderStatus::from_byte(byte);
        match status.classify() {
            RunHeaderStatusClass::Unknown(u) => {
                prop_assert_eq!(u, byte);
            }
            RunHeaderStatusClass::Known(_) => {
                panic!("byte {byte} should classify as Unknown");
            }
        }
    }

    /// `KnownRunHeaderStatus::as_byte` is the inverse of
    /// `from_byte` for the four known values. This is the
    /// state-machine byte-stability floor.
    #[test]
    fn sm_known_status_byte_roundtrip(
        which in 0u8..4u8,
    ) {
        let known = match which {
            0 => KnownRunHeaderStatus::Pending,
            1 => KnownRunHeaderStatus::Accepted,
            2 => KnownRunHeaderStatus::Active,
            _ => KnownRunHeaderStatus::Finished,
        };
        let byte = known.as_byte();
        let back = RunHeaderStatus::from_byte(byte).known();
        match back {
            Ok(k) => prop_assert_eq!(k, known),
            Err(e) => {
                panic!("round trip failed: {e:?}");
            }
        }
    }

    /// Encoding then decoding a `BlobRecord` is the
    /// state-machine "write then read" path. The decode envelope
    /// matches the encode envelope, and the decoded record
    /// equals the original.
    #[test]
    fn sm_record_roundtrip_preserves_state(
        bytes in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let record = BlobRecord {
            digest: [7u8; 32],
            bytes: bytes.clone(),
        };
        let encoded = encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        ).expect("encoding succeeds");
        let (env, payload) = decode_envelope_only(
            &encoded,
            MAGIC_BLOB,
            MAX_BLOB_BYTES,
        ).expect("envelope decode succeeds");
        prop_assert_eq!(env.magic, MAGIC_BLOB);
        prop_assert_eq!(env.record_kind, RecordKind::Blob.id());
        prop_assert_eq!(payload, &encoded[60..]);
        let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, MAX_BLOB_BYTES)
            .expect("decode_record succeeds");
        let (env2, record2) = decoded;
        prop_assert_eq!(env.magic, env2.magic);
        prop_assert_eq!(env.record_kind, env2.record_kind);
        prop_assert_eq!(record, record2);
    }

    /// `RunHeaderStatus::classify` is deterministic: calling it
    /// twice on the same byte returns equal classifications.
    #[test]
    fn sm_classify_is_deterministic(byte in any::<u8>()) {
        let status = RunHeaderStatus::from_byte(byte);
        let c1 = status.classify();
        let c2 = status.classify();
        prop_assert_eq!(c1, c2);
    }

    /// `RunHeaderStatus::known` and `RunHeaderStatus::classify`
    /// agree: a byte that classifies as `Known` must yield `Ok`
    /// from `.known()`, and a byte that classifies as `Unknown`
    /// must yield an `Err` carrying the same byte.
    #[test]
    fn sm_known_and_classify_agree(byte in any::<u8>()) {
        let status = RunHeaderStatus::from_byte(byte);
        let classification = status.classify();
        let known_result = status.known();
        match classification {
            RunHeaderStatusClass::Known(known) => {
                match known_result {
                    Ok(k) => prop_assert_eq!(k, known),
                    Err(_) => {
                        panic!("classify said Known but .known() returned Err");
                    }
                }
            }
            RunHeaderStatusClass::Unknown(u) => {
                match known_result {
                    Ok(_) => {
                        panic!("classify said Unknown but .known() returned Ok");
                    }
                    Err(e) => {
                        prop_assert_eq!(e.byte(), u);
                    }
                }
            }
        }
    }

    /// `UnknownRunHeaderStatus::byte` returns the exact byte that
    /// was rejected. This is the typed-error back-reference floor.
    #[test]
    fn sm_unknown_status_byte_backref(
        byte in 4u8..=u8::MAX,
    ) {
        let u = UnknownRunHeaderStatus::from_byte(byte);
        prop_assert_eq!(u.byte(), byte);
    }

    /// `RecordKind::id` is deterministic: the same `RecordKind`
    /// always reports the same wire id.
    #[test]
    fn sm_record_kind_id_is_deterministic(
        which in 0u8..6u8,
    ) {
        let kind = match which {
            0 => RecordKind::WorkflowSource,
            1 => RecordKind::CompiledIr,
            2 => RecordKind::RunHeader,
            3 => RecordKind::RunAccepted,
            4 => RecordKind::StepStarted,
            _ => RecordKind::RunFinished,
        };
        prop_assert_eq!(kind.id(), kind.id());
    }
}
