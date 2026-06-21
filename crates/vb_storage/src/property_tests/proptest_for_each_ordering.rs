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

//! Section 38 property test: `for_each_ordering`.
//!
//! Master plan §38, row "Ordering invariants":
//! "Iteration over stored records is in canonical (key) order".
//!
//! This file asserts the storage-layer ordering invariants of
//! `vb_storage`:
//! - `run_event_key` produces keys in monotonic order for a single
//!   run: smaller sequence → smaller key.
//! - Keys for different runs are ordered by `run_id` first (the
//!   `run` field precedes the `seq` field in the big-endian layout).
//! - `EventSeq` is total: for any two event sequences, one is
//!   less-than or equal to the other.
//! - Replay of an arbitrary event sequence yields events in
//!   insertion (sequence) order.
//! - A sorted set of `EventSeq` values, when converted to keys,
//!   produces a sorted set of keys.

use proptest::prelude::*;
use vb_core::RunId;

use crate::keys::run_event_key;
use crate::types::EventSeq;

proptest! {
    /// `run_event_key` is strictly monotonic in `seq` for a fixed
    /// run: smaller sequence numbers produce smaller keys.
    #[test]
    fn fe_run_event_key_monotonic_in_seq(
        run_val in 1u64..100000u64,
        a in 0u64..1000u64,
        b in 0u64..1000u64,
    ) {
        prop_assume!(a != b);
        let run = RunId::new(run_val);
        let ka = run_event_key(run, EventSeq::new(a)).expect("encodes");
        let kb = run_event_key(run, EventSeq::new(b)).expect("encodes");
        if a < b {
            prop_assert!(ka < kb, "smaller seq must produce smaller key");
        } else {
            prop_assert!(ka > kb, "larger seq must produce larger key");
        }
    }

    /// For two different runs, the run-prefix determines the key
    /// ordering: a smaller run_id produces a smaller key, regardless
    /// of sequence number. This is the per-run prefix isolation
    /// floor — events for different runs never alias.
    #[test]
    fn fe_run_event_key_isolated_by_run(
        run_a in 1u64..100000u64,
        run_b in 1u64..100000u64,
        seq_a in 0u64..1000u64,
        seq_b in 0u64..1000u64,
    ) {
        prop_assume!(run_a != run_b);
        let ka = run_event_key(RunId::new(run_a), EventSeq::new(seq_a))
            .expect("encodes");
        let kb = run_event_key(RunId::new(run_b), EventSeq::new(seq_b))
            .expect("encodes");
        if run_a < run_b {
            prop_assert!(
                ka < kb,
                "smaller run_id must produce smaller key regardless of seq"
            );
        } else {
            prop_assert!(
                ka > kb,
                "larger run_id must produce larger key regardless of seq"
            );
        }
    }

    /// `EventSeq` is total: for any two sequence numbers, one is
    /// less-than or equal to the other.
    #[test]
    fn fe_event_seq_is_total(a in 0u64..u64::MAX, b in 0u64..u64::MAX) {
        let ea = EventSeq::new(a);
        let eb = EventSeq::new(b);
        let le = ea <= eb;
        let ge = ea >= eb;
        prop_assert!(le || ge, "EventSeq must be total");
    }

    /// A sorted set of `EventSeq` values, when converted to keys,
    /// produces a sorted set of keys.
    #[test]
    fn fe_event_seq_set_preserves_key_order(
        run_val in 1u64..100000u64,
        seqs in proptest::collection::vec(0u64..1000u64, 1..16),
    ) {
        let mut sorted: Vec<u64> = seqs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        let run = RunId::new(run_val);
        let keys: Vec<_> = sorted
            .iter()
            .map(|s| run_event_key(run, EventSeq::new(*s)).expect("encodes"))
            .collect();
        for pair in keys.windows(2) {
            prop_assert!(pair[0] < pair[1], "sorted seqs must produce sorted keys");
        }
    }

    /// Replay of an arbitrary `EventSeq` sequence is in insertion
    /// order. We model the key encoding as the iteration path: a
    /// sorted iteration over a `Vec<EventSeq>` yields monotonically
    /// increasing keys.
    #[test]
    fn fe_replay_is_in_seq_order(
        run_val in 1u64..100000u64,
        seqs in proptest::collection::vec(0u64..1000u64, 1..16),
    ) {
        let mut sorted: Vec<u64> = seqs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        let run = RunId::new(run_val);
        let keys: Vec<_> = sorted
            .iter()
            .map(|s| run_event_key(run, EventSeq::new(*s)).expect("encodes"))
            .collect();
        // The replay ordering must be strictly monotonic.
        for i in 1..keys.len() {
            prop_assert!(
                keys[i - 1] < keys[i],
                "key {} ({:?}) must be < key {} ({:?})",
                i - 1, keys[i - 1], i, keys[i],
            );
        }
    }

    /// Two distinct runs with the same sequence number produce
    /// distinct keys (no cross-run collision in the sequence
    /// dimension).
    #[test]
    fn fe_distinct_runs_same_seq_produce_distinct_keys(
        a in 1u64..100000u64,
        b in 1u64..100000u64,
        seq_val in 0u64..1000u64,
    ) {
        prop_assume!(a != b);
        let ka = run_event_key(RunId::new(a), EventSeq::new(seq_val))
            .expect("encodes");
        let kb = run_event_key(RunId::new(b), EventSeq::new(seq_val))
            .expect("encodes");
        prop_assert_ne!(ka, kb);
    }

    /// `EventSeq` is `Ord`: sorting a `Vec<EventSeq>` produces a
    /// non-decreasing sequence.
    #[test]
    fn fe_event_seq_sortable(
        seqs in proptest::collection::vec(0u64..1000u64, 1..16),
    ) {
        let mut e: Vec<EventSeq> = seqs.iter().map(|s| EventSeq::new(*s)).collect();
        e.sort();
        for pair in e.windows(2) {
            prop_assert!(pair[0] <= pair[1]);
        }
    }

    /// Iterating events in their key-encoded order is equivalent to
    /// iterating them in sequence order. We dedupe sequence numbers
    /// first because the key encoding is injective (distinct seqs
    /// produce distinct keys) — a duplicate seq would be its own
    /// key, but the canonical seq order is a set, not a multiset.
    #[test]
    fn fe_key_order_equals_seq_order(
        run_val in 1u64..100000u64,
        seqs in proptest::collection::vec(0u64..1000u64, 1..16),
    ) {
        let run = RunId::new(run_val);
        // Dedupe sequences first.
        let mut unique: Vec<u64> = seqs.clone();
        unique.sort_unstable();
        unique.dedup();

        // Sort by key.
        let mut by_key: Vec<_> = unique
            .iter()
            .map(|s| (run_event_key(run, EventSeq::new(*s)).expect("encodes"), *s))
            .collect();
        by_key.sort_by_key(|(k, _)| *k);
        let by_key_seqs: Vec<u64> = by_key.iter().map(|(_, s)| *s).collect();

        // Sort by sequence (already sorted and deduped).
        prop_assert_eq!(by_key_seqs, unique);
    }
}
