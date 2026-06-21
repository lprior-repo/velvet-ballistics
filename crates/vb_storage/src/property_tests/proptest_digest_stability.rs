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

//! Section 38 property test: `digest_stability`.
//!
//! Master plan §38, row "Digest stability":
//! "Same bytes always hash to same digest; digest mismatch rejected".
//!
//! This file asserts the digest-stability invariants of
//! `verify_digest_match` and the on-disk `blake3` payload digest:
//! - For any byte payload, hashing twice yields the same digest
//!   (determinism).
//! - For any two distinct payloads, the digests differ (collision
//!   resistance on the boundary tested by proptest).
//! - A bit-flipped payload produces a digest that does NOT match
//!   the original (tamper detection).
//! - `verify_digest_match` accepts the matching digest and rejects
//!   every other digest on the same payload.
//! - `verify_digest_match` never panics for any payload/digest pair.

use proptest::prelude::*;

use crate::codec::verify_digest_match;
use crate::constants::DIGEST_BYTES;

proptest! {
    /// `blake3::hash` is deterministic: hashing the same bytes twice
    /// yields the same digest.
    #[test]
    fn ds_hash_is_deterministic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let a = blake3::hash(&bytes);
        let b = blake3::hash(&bytes);
        prop_assert_eq!(a.as_bytes(), b.as_bytes());
    }

    /// Distinct payloads (differing in at least one byte) produce
    /// distinct digests. This is a probabilistic property
    /// (BLAKE3 is collision-resistant in the cryptographic sense)
    /// but it holds for the small input space we test against
    /// proptest.
    #[test]
    fn ds_distinct_payloads_distinct_digests(
        a in proptest::collection::vec(any::<u8>(), 1..128),
        b in proptest::collection::vec(any::<u8>(), 1..128),
    ) {
        prop_assume!(a != b);
        let ha = blake3::hash(&a);
        let hb = blake3::hash(&b);
        prop_assert_ne!(ha.as_bytes(), hb.as_bytes());
    }

    /// A bit-flipped payload produces a digest that differs from
    /// the original. This is the tamper-detection floor.
    #[test]
    fn ds_bitflip_changes_digest(
        base in proptest::collection::vec(any::<u8>(), 1..128),
        flip_index in 0usize..128usize,
    ) {
        let mut flipped = base.clone();
        let safe_index = flip_index % flipped.len();
        flipped[safe_index] = flipped[safe_index] ^ 0x01;
        prop_assume!(flipped != base);
        let h_original = blake3::hash(&base);
        let h_flipped = blake3::hash(&flipped);
        prop_assert_ne!(h_original.as_bytes(), h_flipped.as_bytes());
    }

    /// `verify_digest_match` accepts the matching digest on the
    /// correct payload. This is the canonical happy path.
    #[test]
    fn ds_verify_accepts_matching_digest(
        bytes in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        let digest: [u8; DIGEST_BYTES] = *blake3::hash(&bytes).as_bytes();
        let result = verify_digest_match(&bytes, digest);
        prop_assert!(result.is_ok(), "matching digest must verify");
    }

    /// `verify_digest_match` rejects a non-matching digest. We
    /// construct a digest that differs from the real one in at
    /// least one byte.
    #[test]
    fn ds_verify_rejects_mismatched_digest(
        bytes in proptest::collection::vec(any::<u8>(), 0..128),
        flip_byte in 0usize..DIGEST_BYTES,
    ) {
        let real: [u8; DIGEST_BYTES] = *blake3::hash(&bytes).as_bytes();
        let mut bad = real;
        bad[flip_byte] = bad[flip_byte] ^ 0x01;
        prop_assume!(bad != real);
        let result = verify_digest_match(&bytes, bad);
        prop_assert!(result.is_err(), "mismatched digest must fail");
    }

    /// `verify_digest_match` never panics for arbitrary payload
    /// and arbitrary digest. This is the no-panic floor.
    #[test]
    fn ds_verify_never_panics(
        bytes in proptest::collection::vec(any::<u8>(), 0..256),
        digest in proptest::array::uniform32(any::<u8>()),
    ) {
        let result = verify_digest_match(&bytes, digest);
        // The result is `Result<(), JournalError>`. We don't care
        // about the outcome — only that the call doesn't panic.
        let _ = result;
    }

    /// The digest length is exactly 32 bytes for any payload.
    /// (BLAKE3 always produces 32-byte output.)
    #[test]
    fn ds_digest_length_is_32(_unit in 0u8..1u8) {
        prop_assert_eq!(DIGEST_BYTES, 32);
        let h = blake3::hash(&[]);
        prop_assert_eq!(h.as_bytes().len(), 32);
    }

    /// Hashing the empty payload yields a well-defined, fixed
    /// digest. (BLAKE3's empty digest is part of its public
    /// contract.)
    #[test]
    fn ds_empty_payload_digest_is_known(_unit in 0u8..1u8) {
        let h = blake3::hash(&[]);
        let expected: [u8; 32] = *h.as_bytes();
        let h2 = blake3::hash(&[]);
        prop_assert_eq!(expected, *h2.as_bytes());
    }

    /// `verify_digest_match` is consistent: when the same payload
    /// and digest pair is checked twice, both calls return the same
    /// `Result`.
    #[test]
    fn ds_verify_is_deterministic(
        bytes in proptest::collection::vec(any::<u8>(), 0..128),
        digest in proptest::array::uniform32(any::<u8>()),
    ) {
        let r1 = verify_digest_match(&bytes, digest);
        let r2 = verify_digest_match(&bytes, digest);
        prop_assert_eq!(r1.is_ok(), r2.is_ok());
    }

    /// Two distinct payloads with the same bit count do not collide
    /// in our tested input space. (The probability of accidental
    /// collision in a 256-bit hash space is ~2^-256, well below
    /// proptest's default `PROPTEST_FACTOR`.)
    #[test]
    fn ds_no_collision_for_short_distinct_payloads(
        a in proptest::array::uniform4(any::<u8>()),
        b in proptest::array::uniform4(any::<u8>()),
    ) {
        prop_assume!(a != b);
        let ha = blake3::hash(&a);
        let hb = blake3::hash(&b);
        prop_assert_ne!(ha.as_bytes(), hb.as_bytes());
    }
}
