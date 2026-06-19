#![forbid(unsafe_code)]
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
//! Red-Queen adversarial state-space pressure tests for tier-a-6-011
//! (CallerCapabilities envelope).
//!
//! Bead: tier-a-6-011
//! State machine: missing / valid / invalid / peer-credentials-fail
//! Pressure: races between capability-producing threads, malformed envelope
//! boundaries (truncated, oversized, version-mismatched), and replay attacks
//! (same capability value used twice must still produce the same acceptance).
//!
//! These tests are deterministic. All checks are performed via exit code
//! comparison (no AI in the gate).

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use bytes::Bytes;
use vb_ipc::bounded::MaxPayloadBytes;
use vb_ipc::capabilities::{
    ACTION_HANDLER_CAPABILITY_BIT, CallerCapabilities, OBSERVER_CAPABILITY_BIT,
    OPERATOR_CAPABILITY_BIT, ROOT_CAPABILITY_BIT, SUBMITTER_CAPABILITY_BIT,
};
use vb_ipc::frame_types::{IpcFrameHeader, decode_frame};
use vb_ipc::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a valid header byte buffer with caller-capabilities envelope
/// set to `caps_bits`.
fn make_header_with_caps(caps_bits: u16) -> [u8; IPC_HEADER_LEN] {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    // flags 8..10 = 0
    bytes[10..12].copy_from_slice(&caps_bits.to_le_bytes());
    // correlation 12..20 = 0
    // payload_len 20..24 = 0
    bytes
}

// ---------------------------------------------------------------------------
// Q1 — Boundary: every bit position 0..16 of the envelope
// ---------------------------------------------------------------------------

#[test]
fn red_queen_envelope_accepts_every_nonzero_bit_position() {
    // Each nonzero bit value (1..=u16::MAX) must produce Some(CallerCapabilities).
    // This is the dense state-space check that bit positions outside the
    // documented constants are still accepted (no panic, no other error).
    for bits in 1u16..=u16::MAX {
        let caps = CallerCapabilities::from_wire(bits);
        assert!(caps.is_some(), "nonzero bits must be Some: bits={bits}");
        let raw = caps.map_or(0, CallerCapabilities::bits);
        assert_eq!(raw, bits, "from_wire must preserve the raw bits");
        let caps = caps.expect("already asserted");
        assert!(!caps.is_empty(), "nonzero bits must not be empty");
    }
}

#[test]
fn red_queen_envelope_rejects_exactly_zero_sentinel() {
    // State: missing capability. The sentinel value 0 must be rejected
    // and only 0.
    assert_eq!(CallerCapabilities::from_wire(0), None);
    assert!(CallerCapabilities::EMPTY.is_empty());
}

// ---------------------------------------------------------------------------
// Q2 — Header boundary: every documented capability bit decodes into the
// expected wire envelope without being rejected by the frame header.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_every_documented_capability_bit_decodes_ok() {
    let documented_bits: [(u16, &str); 5] = [
        (ROOT_CAPABILITY_BIT, "ROOT"),
        (OPERATOR_CAPABILITY_BIT, "OPERATOR"),
        (OBSERVER_CAPABILITY_BIT, "OBSERVER"),
        (SUBMITTER_CAPABILITY_BIT, "SUBMITTER"),
        (ACTION_HANDLER_CAPABILITY_BIT, "ACTION_HANDLER"),
    ];
    for (bit, label) in documented_bits {
        let header_bytes = make_header_with_caps(bit);
        let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
        assert!(
            result.is_ok(),
            "{label} bit ({bit}) header must decode, got {result:?}"
        );
        let header = result.expect("already asserted");
        assert_eq!(
            header.caller_capabilities.bits(),
            bit,
            "{label} bit must roundtrip through header"
        );
    }
}

#[test]
fn red_queen_union_of_capabilities_decodes_to_superset() {
    // The union of all five documented capability bits must decode OK and
    // be detected as a superset of every single one.
    let union = ROOT_CAPABILITY_BIT
        | OPERATOR_CAPABILITY_BIT
        | OBSERVER_CAPABILITY_BIT
        | SUBMITTER_CAPABILITY_BIT
        | ACTION_HANDLER_CAPABILITY_BIT;
    let caps = CallerCapabilities::from_raw(union);
    assert!(caps.contains(CallerCapabilities::ROOT));
    assert!(caps.contains(CallerCapabilities::OPERATOR));
    assert!(caps.contains(CallerCapabilities::OBSERVER));
    assert!(caps.contains(CallerCapabilities::SUBMITTER));
    assert!(caps.contains(CallerCapabilities::ACTION_HANDLER));

    // Roundtrip through header.
    let header_bytes = make_header_with_caps(union);
    let header =
        IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT).expect("union must decode");
    assert_eq!(header.caller_capabilities, caps);
}

// ---------------------------------------------------------------------------
// Q3 — Header invalid paths: truncated, oversized, version-mismatched,
// magic-mismatched.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_zero_capability_with_zero_magic_rejected_as_invalid_magic_first() {
    // When the magic is wrong, the header decoder must surface
    // InvalidMagic rather than PermissionDenied — the magic check is the
    // first gate and the capability check comes after it.
    let mut header_bytes = make_header_with_caps(0);
    header_bytes[..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
    assert!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "wrong magic must surface as InvalidMagic, got {result:?}"
    );
}

#[test]
fn red_queen_zero_capability_with_wrong_version_rejected_as_unsupported_version() {
    // Wrong version with capabilities=0 must surface as UnsupportedVersion,
    // not PermissionDenied. This proves the version check precedes the
    // capability check.
    let mut header_bytes = make_header_with_caps(0);
    // Bump version to 99.
    header_bytes[4..6].copy_from_slice(&99u16.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
    assert!(
        matches!(result, Err(IpcError::UnsupportedVersion { actual: 99 })),
        "wrong version must surface as UnsupportedVersion, got {result:?}"
    );
}

#[test]
fn red_queen_zero_capability_with_zero_command_rejected_as_permission_denied() {
    // When magic, version, and command are all valid but capabilities=0,
    // the envelope check must surface PermissionDenied. This is the
    // happy-path missing-capability rejection.
    let header_bytes = make_header_with_caps(0);
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
    assert_eq!(
        result,
        Err(IpcError::PermissionDenied),
        "capabilities=0 must be PermissionDenied"
    );
}

#[test]
fn red_queen_zero_capability_with_oversized_payload_still_permission_denied() {
    // The capability check must precede the payload bound check: a frame
    // with capabilities=0 and payload_len > max must still surface as
    // PermissionDenied, NOT PayloadTooLarge. This protects the boundary
    // ordering invariant (SEC-01).
    let mut header_bytes = make_header_with_caps(0);
    // payload_len 20..24 = u32::MAX
    let oversized = u32::MAX;
    header_bytes[20..24].copy_from_slice(&oversized.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
    assert_eq!(
        result,
        Err(IpcError::PermissionDenied),
        "missing capability must precede payload bound check"
    );
}

// ---------------------------------------------------------------------------
// Q4 — Race: many threads try to encode/decode the SAME capability value
// simultaneously. Encoding and decoding must be deterministic (no panic, no
// data race on the bitmap).
// ---------------------------------------------------------------------------

#[test]
fn red_queen_race_concurrent_encode_decode_same_capability() {
    // IpcFrameHeader::new defaults to CallerCapabilities::ROOT.
    let expected_caps = CallerCapabilities::ROOT;
    let mismatch_count = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::with_capacity(8);
    for _ in 0..8 {
        let mismatch = Arc::clone(&mismatch_count);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
                let encoded = header.encode().expect("encode must succeed");
                let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT)
                    .expect("encode/decode roundtrip must succeed");
                if decoded.caller_capabilities != expected_caps {
                    mismatch.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread must not panic");
    }
    assert_eq!(
        mismatch_count.load(Ordering::Relaxed),
        0,
        "encode/decode roundtrip must be deterministic across threads"
    );
}

#[test]
fn red_queen_race_concurrent_distinct_capabilities_never_collide() {
    // 16 threads, each with a distinct capability bitmap (bit i set for
    // thread i). After 100 iterations of encode/decode, every decoded
    // envelope must equal the thread's local expected value (no cross-
    // thread leakage of capability bits).
    let mismatch_count = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::with_capacity(16);
    for thread_index in 0..16u16 {
        let bits: u16 = 1u16 << thread_index;
        let expected = CallerCapabilities::from_raw(bits);
        let mismatch = Arc::clone(&mismatch_count);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let header_bytes = make_header_with_caps(bits);
                let decoded = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)
                    .expect("any nonzero bits decode as valid");
                if decoded.caller_capabilities != expected {
                    mismatch.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread must not panic");
    }
    assert_eq!(
        mismatch_count.load(Ordering::Relaxed),
        0,
        "distinct bitmaps must never cross-contaminate during decode"
    );
}

#[test]
fn red_queen_race_distinct_threads_encoding_then_decoding() {
    // 32 threads each encode a unique capability (different bit patterns)
    // and verify decode matches their encode. Tests for thread-local
    // bitmap state corruption.
    let mismatch_count = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::with_capacity(32);
    for thread_index in 0..32u16 {
        let bits: u16 = (thread_index + 1) as u16;
        let expected = CallerCapabilities::from_raw(bits);
        let mismatch = Arc::clone(&mismatch_count);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                let caps = CallerCapabilities::from_raw(bits);
                let header =
                    IpcFrameHeader::new_with_capabilities(IpcCommand::Health, 0, caps, 0, 0);
                let encoded = header.encode().expect("encode");
                let decoded =
                    IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT).expect("decode");
                if decoded.caller_capabilities != expected {
                    mismatch.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread must not panic");
    }
    assert_eq!(
        mismatch_count.load(Ordering::Relaxed),
        0,
        "encode then decode must roundtrip for every distinct capability"
    );
}

// ---------------------------------------------------------------------------
// Q5 — Replay: sending the same valid capability envelope twice must produce
// the same acceptance (no replay-side side effect that breaks idempotency).
// ---------------------------------------------------------------------------

#[test]
fn red_queen_replay_same_envelope_is_idempotent() {
    let header_bytes = make_header_with_caps(ROOT_CAPABILITY_BIT);
    let first =
        IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT).expect("first decode");
    let second =
        IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT).expect("second decode");
    let third =
        IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT).expect("third decode");
    assert_eq!(first.caller_capabilities, second.caller_capabilities);
    assert_eq!(second.caller_capabilities, third.caller_capabilities);
    // Header content other than capabilities must also be identical.
    assert_eq!(first.command, second.command);
    assert_eq!(second.command, third.command);
}

#[test]
fn red_queen_replay_alternating_zero_and_root_is_not_idempotent() {
    // The capability envelope is part of the wire header; the zero
    // sentinel must keep being rejected and the ROOT value must keep
    // being accepted across alternations (no cache poisoning).
    for iteration in 0..10 {
        let zero_bytes = make_header_with_caps(0);
        let root_bytes = make_header_with_caps(ROOT_CAPABILITY_BIT);

        let zero = IpcFrameHeader::decode(&zero_bytes, MaxPayloadBytes::DEFAULT);
        let root = IpcFrameHeader::decode(&root_bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            zero,
            Err(IpcError::PermissionDenied),
            "iteration {iteration}: zero must keep being PermissionDenied"
        );
        assert!(
            root.is_ok(),
            "iteration {iteration}: ROOT must keep decoding as Ok"
        );
    }
}

// ---------------------------------------------------------------------------
// Q6 — Boundary: the bounded reader must surface the same PermissionDenied
// rejection as the unbounded decoder (parity check between reader paths).
// ---------------------------------------------------------------------------

#[test]
fn red_queen_bounded_reader_rejects_zero_capabilities_consistently() {
    let header_bytes = make_header_with_caps(0);
    let mut cursor = Cursor::new(header_bytes.to_vec());

    let result = vb_ipc::read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);
    assert_eq!(
        result,
        Err(IpcError::PermissionDenied),
        "bounded reader must reject zero capabilities the same as unbounded"
    );
}

#[test]
fn red_queen_bounded_reader_accepts_root_capabilities() {
    let header_bytes = make_header_with_caps(ROOT_CAPABILITY_BIT);
    let mut cursor = Cursor::new(header_bytes.to_vec());

    let result = vb_ipc::read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);
    let header = result.expect("ROOT must be accepted by bounded reader");
    assert_eq!(header.caller_capabilities, CallerCapabilities::ROOT);
}

// ---------------------------------------------------------------------------
// Q7 — decode_frame parity: the higher-level decode_frame must agree with
// decode_frame_header on the missing-capability rejection.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_decode_frame_rejects_zero_capabilities() {
    let header_bytes = make_header_with_caps(0);
    let result = decode_frame(&header_bytes, Bytes::new(), MaxPayloadBytes::DEFAULT);
    assert_eq!(
        result,
        Err(IpcError::PermissionDenied),
        "decode_frame must reject zero capabilities"
    );
}

#[test]
fn red_queen_decode_frame_accepts_root_capabilities() {
    let header_bytes = make_header_with_caps(ROOT_CAPABILITY_BIT);
    let frame = decode_frame(&header_bytes, Bytes::new(), MaxPayloadBytes::DEFAULT)
        .expect("ROOT must decode");
    assert_eq!(frame.header().caller_capabilities, CallerCapabilities::ROOT);
}

// ---------------------------------------------------------------------------
// Q8 — Capability lattice: every operator/observer/submitter/action-handler
// envelope must contain ROOT (the documented invariant).
// ---------------------------------------------------------------------------

#[test]
fn red_queen_role_envelopes_always_contain_root() {
    let roles = [
        CallerCapabilities::ROOT,
        CallerCapabilities::OPERATOR,
        CallerCapabilities::OBSERVER,
        CallerCapabilities::SUBMITTER,
        CallerCapabilities::ACTION_HANDLER,
    ];
    for caps in roles {
        assert!(
            caps.has_root(),
            "every documented role envelope must carry ROOT bit (caps={})",
            caps.bits()
        );
    }
}

#[test]
fn red_queen_role_envelopes_are_pairwise_distinct() {
    // Every documented role envelope is unique so the dispatch layer can
    // distinguish them by introspection.
    let roles = [
        ("ROOT", CallerCapabilities::ROOT),
        ("OPERATOR", CallerCapabilities::OPERATOR),
        ("OBSERVER", CallerCapabilities::OBSERVER),
        ("SUBMITTER", CallerCapabilities::SUBMITTER),
        ("ACTION_HANDLER", CallerCapabilities::ACTION_HANDLER),
    ];
    for (i, (name_i, caps_i)) in roles.iter().enumerate() {
        for (name_j, caps_j) in roles.iter().skip(i + 1) {
            assert_ne!(
                caps_i,
                caps_j,
                "{name_i} and {name_j} envelopes must be distinct (both = {})",
                caps_i.bits()
            );
        }
    }
}
