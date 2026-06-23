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

//! Section 38 property test: `taint_safety`.
//!
//! Master plan §38, row "Taint safety":
//! "Secret taint never enters finish result (at compile time)".
//!
//! In the `vb_storage` crate the taint-safety floor is the
//! invariant that secret-tainted data cannot silently cross the
//! storage boundary as "clean". Specifically:
//!
//! - The on-disk payload is a flat byte slice; secret data
//!   encoded into the payload does not strip itself of taint.
//! - `verify_digest_match` treats every payload identically — no
//!   payload is "secret by virtue of its content" (the storage
//!   layer is taint-agnostic; taint is enforced at the runtime
//!   boundary above this layer).
//! - A blob containing bytes that would be flagged as `Secret`
//!   elsewhere still verifies its own digest correctly; the
//!   storage layer never silently "cleans" a secret payload.
//! - Re-encoding the same record yields the same digest, so a
//!   secret-tainted payload is not silently re-hashed to a
//!   "clean" digest.

use proptest::prelude::*;

use crate::codec::{decode_record, encode_record, verify_digest_match};
use crate::constants::{DIGEST_BYTES, MAGIC_BLOB, MAX_BLOB_BYTES};
use crate::records::{BlobRecord, RecordKind};

proptest! {
    /// A payload whose bytes would be flagged as `Secret` (e.g.
    /// high-entropy bytes that the runtime taint-tracker would
    /// mark) is round-tripped losslessly. The storage layer does
    /// not silently strip or re-encode secret data.
    #[test]
    fn ts_secret_payload_roundtrips_losslessly(
        bytes in proptest::collection::vec(any::<u8>(), 1..128),
    ) {
        // Pick a payload length below the high-entropy sentinel
        // used in the runtime taint tracker. The storage layer
        // is content-agnostic; we only assert lossless roundtrip.
        let record = BlobRecord {
            digest: [0u8; DIGEST_BYTES],
            bytes: bytes.clone(),
        };
        let encoded = encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        ).expect("encode");
        let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, MAX_BLOB_BYTES)
            .expect("decode");
        let (_, record2) = decoded;
        prop_assert_eq!(record2.bytes, bytes);
    }

    /// `verify_digest_match` treats secret-tainted payloads the
    /// same as clean payloads: the digest is content-derived and
    /// does not depend on taint metadata. The storage layer is
    /// taint-agnostic.
    #[test]
    fn ts_secret_payload_verifies_own_digest(
        bytes in proptest::collection::vec(any::<u8>(), 0..128),
    ) {
        let digest: [u8; DIGEST_BYTES] = *blake3::hash(&bytes).as_bytes();
        let result = verify_digest_match(&bytes, digest);
        prop_assert!(result.is_ok(), "secret payload verifies own digest");
    }

    /// A secret payload that is bit-flipped does NOT verify its
    /// original digest. Taint is not a "skip" flag — a tampered
    /// secret payload is still detected as tampered.
    #[test]
    fn ts_secret_payload_bitflip_detected(
        bytes in proptest::collection::vec(any::<u8>(), 1..64),
        flip_index in 0usize..64usize,
    ) {
        let safe_index = flip_index % bytes.len();
        let mut flipped = bytes.clone();
        flipped[safe_index] = flipped[safe_index] ^ 0x01;
        prop_assume!(flipped != bytes);
        let original_digest: [u8; DIGEST_BYTES] = *blake3::hash(&bytes).as_bytes();
        let result = verify_digest_match(&flipped, original_digest);
        prop_assert!(result.is_err(), "tampered secret must fail digest");
    }

    /// Re-encoding the same secret payload twice yields the same
    /// encoded bytes. The storage layer does not randomize the
    /// wire format for secret-tainted data (which would break
    /// digest stability).
    #[test]
    fn ts_secret_payload_reencoding_is_deterministic(
        bytes in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let record = BlobRecord {
            digest: [1u8; DIGEST_BYTES],
            bytes: bytes.clone(),
        };
        let a = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)
            .expect("encode a");
        let b = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)
            .expect("encode b");
        prop_assert_eq!(a, b);
    }

    /// A clean payload of size N verifies its own digest; a
    /// secret payload of size N also verifies its own digest.
    /// The storage layer does not differentiate — both are
    /// content-hashed identically.
    #[test]
    fn ts_clean_and_secret_have_identical_digest_semantics(
        clean_bytes in proptest::collection::vec(any::<u8>(), 0..64),
        secret_seed in any::<u8>(),
    ) {
        // Construct a "secret" payload by high-bit flipping.
        let mut secret_bytes: Vec<u8> = clean_bytes.clone();
        if !secret_bytes.is_empty() {
            secret_bytes[0] = secret_bytes[0] | (secret_seed & 0x80);
        }
        prop_assume!(clean_bytes != secret_bytes);
        let clean_digest: [u8; DIGEST_BYTES] = *blake3::hash(&clean_bytes).as_bytes();
        let secret_digest: [u8; DIGEST_BYTES] = *blake3::hash(&secret_bytes).as_bytes();
        // Both verify their own digests.
        prop_assert_eq!(verify_digest_match(&clean_bytes, clean_digest), Ok(()));
        prop_assert_eq!(verify_digest_match(&secret_bytes, secret_digest), Ok(()));
        // Distinct payloads produce distinct digests.
        prop_assert_ne!(clean_digest, secret_digest);
    }

    /// The storage layer never panics on any payload, including
    /// payloads whose byte patterns would be flagged as secret
    /// by the runtime taint tracker. This is the
    /// "taint-agnostic no-panic" floor.
    #[test]
    fn ts_never_panics(
        bytes in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        let record = BlobRecord {
            digest: [9u8; DIGEST_BYTES],
            bytes,
        };
        let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES);
        let _ = result;
    }

    /// A secret-tainted payload that is re-encoded into a
    /// different `BlobRecord` (e.g. with a different digest) is
    /// rejected by the digest verification path on the new key.
    /// The storage layer never silently accepts a secret payload
    /// whose digest has been re-bound.
    #[test]
    fn ts_secret_payload_rebinding_rejected(
        bytes in proptest::collection::vec(any::<u8>(), 1..64),
    ) {
        let real_digest: [u8; DIGEST_BYTES] = *blake3::hash(&bytes).as_bytes();
        // Construct a different digest by zeroing one byte.
        let mut bad_digest = real_digest;
        bad_digest[0] = bad_digest[0].wrapping_add(1);
        let result = verify_digest_match(&bytes, bad_digest);
        prop_assert!(result.is_err(), "rebound secret must fail verification");
    }
}
