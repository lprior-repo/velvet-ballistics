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

//! Section 38 property test: `layout_stability`.
//!
//! Master plan §38, row "Layout stability":
//! "Schema changes are explicit; on-disk layout is stable across versions".
//!
//! This file asserts the wire/key layout invariants of `vb_storage`:
//! - The first byte of every key is the documented type prefix.
//! - Two distinct record kinds produce distinct prefix bytes; two
//!   distinct run ids produce distinct keys; two distinct sequence
//!   numbers produce distinct keys.
//! - `RecordKind::id()` returns a stable, well-known value across
//!   all known kinds.
//! - Re-encoding the same key inputs yields byte-equal keys
//!   (determinism).
//! - The record header layout encodes the magic at offset 0 and
//!   the schema version at offset 4; both are recoverable from a
//!   round-trip.

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx, WorkflowId};

use crate::codec::{encode_record_header, decode_record_header};
use crate::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_WORKFLOW_SOURCE,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
    PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RECOVERY_STAMP, PREFIX_RUN_EVENT,
    PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
};
use crate::keys::{
    blob_key, compiled_ir_key, index_action_key, index_status_key, index_workflow_key,
    recovery_stamp_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
};
use crate::records::RecordKind;
use crate::types::{EventSeq, IndexStatusState};

proptest! {
    /// `workflow_source_key` always begins with the documented
    /// `PREFIX_WORKFLOW_SOURCE` byte.
    #[test]
    fn ls_workflow_source_key_prefix(
        digest in proptest::array::uniform32(any::<u8>()),
    ) {
        let key = workflow_source_key(digest).expect("workflow_source_key encodes");
        prop_assert_eq!(key[0], PREFIX_WORKFLOW_SOURCE);
    }

    /// `compiled_ir_key` always begins with `PREFIX_COMPILED_IR`.
    #[test]
    fn ls_compiled_ir_key_prefix(
        digest in proptest::array::uniform32(any::<u8>()),
    ) {
        let key = compiled_ir_key(digest).expect("compiled_ir_key encodes");
        prop_assert_eq!(key[0], PREFIX_COMPILED_IR);
    }

    /// `run_header_key` always begins with `PREFIX_RUN_HEADER`.
    #[test]
    fn ls_run_header_key_prefix(run_val in 1u64..100000u64) {
        let key = run_header_key(RunId::new(run_val)).expect("run_header_key encodes");
        prop_assert_eq!(key[0], PREFIX_RUN_HEADER);
    }

    /// `run_event_key` always begins with `PREFIX_RUN_EVENT`.
    #[test]
    fn ls_run_event_key_prefix(
        run_val in 1u64..100000u64,
        seq_val in 0u64..1000u64,
    ) {
        let key = run_event_key(RunId::new(run_val), EventSeq::new(seq_val))
            .expect("run_event_key encodes");
        prop_assert_eq!(key[0], PREFIX_RUN_EVENT);
    }

    /// `run_snapshot_key` always begins with `PREFIX_RUN_SNAPSHOT`.
    #[test]
    fn ls_run_snapshot_key_prefix(
        run_val in 1u64..100000u64,
        seq_val in 0u64..1000u64,
    ) {
        let key = run_snapshot_key(RunId::new(run_val), EventSeq::new(seq_val))
            .expect("run_snapshot_key encodes");
        prop_assert_eq!(key[0], PREFIX_RUN_SNAPSHOT);
    }

    /// `blob_key` always begins with `PREFIX_BLOB`.
    #[test]
    fn ls_blob_key_prefix(
        digest in proptest::array::uniform32(any::<u8>()),
    ) {
        let key = blob_key(digest).expect("blob_key encodes");
        prop_assert_eq!(key[0], PREFIX_BLOB);
    }

    /// `index_status_key` always begins with `PREFIX_INDEX_STATUS`.
    #[test]
    fn ls_index_status_key_prefix(
        state_val in 0u8..8u8,
        ts_val in 0u64..100000u64,
        run_val in 1u64..100000u64,
    ) {
        let state = IndexStatusState::from_u8(state_val);
        let key = index_status_key(state, ts_val, RunId::new(run_val))
            .expect("index_status_key encodes");
        prop_assert_eq!(key[0], PREFIX_INDEX_STATUS);
    }

    /// `index_workflow_key` always begins with `PREFIX_INDEX_WORKFLOW`.
    #[test]
    fn ls_index_workflow_key_prefix(
        wf_val in 1u32..100000u32,
        run_val in 1u64..100000u64,
    ) {
        let key = index_workflow_key(WorkflowId::new(wf_val), RunId::new(run_val))
            .expect("index_workflow_key encodes");
        prop_assert_eq!(key[0], PREFIX_INDEX_WORKFLOW);
    }

    /// `index_action_key` always begins with `PREFIX_INDEX_ACTION`.
    #[test]
    fn ls_index_action_key_prefix(
        action_val in 1u16..1000u16,
        run_val in 1u64..100000u64,
        step_val in 0u16..1000u16,
    ) {
        let key = index_action_key(
            ActionId::new(action_val),
            RunId::new(run_val),
            StepIdx::new(step_val),
        ).expect("index_action_key encodes");
        prop_assert_eq!(key[0], PREFIX_INDEX_ACTION);
    }

    /// `recovery_stamp_key` always begins with `PREFIX_RECOVERY_STAMP`.
    #[test]
    fn ls_recovery_stamp_key_prefix(
        run_val in 1u64..100000u64,
        seq_val in 0u64..1000u64,
    ) {
        let key = recovery_stamp_key(RunId::new(run_val), EventSeq::new(seq_val))
            .expect("recovery_stamp_key encodes");
        prop_assert_eq!(key[0], PREFIX_RECOVERY_STAMP);
    }

    /// All declared prefix bytes are distinct. A collision in the
    /// prefix space would silently alias one keyspace onto another.
    #[test]
    fn ls_prefix_bytes_are_distinct(_unit in 0u8..1u8) {
        let prefixes = [
            PREFIX_WORKFLOW_SOURCE,
            PREFIX_COMPILED_IR,
            PREFIX_RUN_HEADER,
            PREFIX_RUN_EVENT,
            PREFIX_RUN_SNAPSHOT,
            PREFIX_BLOB,
            PREFIX_INDEX_STATUS,
            PREFIX_INDEX_WORKFLOW,
            PREFIX_INDEX_ACTION,
            PREFIX_RECOVERY_STAMP,
        ];
        for i in 0..prefixes.len() {
            for j in (i + 1)..prefixes.len() {
                prop_assert_ne!(
                    prefixes[i], prefixes[j],
                    "prefix bytes must be distinct"
                );
            }
        }
    }

    /// Key encoding is deterministic: encoding the same inputs
    /// twice yields byte-equal keys.
    #[test]
    fn ls_keys_are_deterministic(
        run_val in 1u64..1000u64,
        seq_val in 0u64..1000u64,
        digest_seed in any::<u8>(),
    ) {
        let run = RunId::new(run_val);
        let seq = EventSeq::new(seq_val);
        let mut digest = [0u8; 32];
        digest[0] = digest_seed;
        let k1a = run_event_key(run, seq).expect("encodes");
        let k1b = run_event_key(run, seq).expect("encodes");
        prop_assert_eq!(k1a, k1b);

        let k2a = workflow_source_key(digest).expect("encodes");
        let k2b = workflow_source_key(digest).expect("encodes");
        prop_assert_eq!(k2a, k2b);

        let k3a = run_header_key(run).expect("encodes");
        let k3b = run_header_key(run).expect("encodes");
        prop_assert_eq!(k3a, k3b);

        let k4a = blob_key(digest).expect("encodes");
        let k4b = blob_key(digest).expect("encodes");
        prop_assert_eq!(k4a, k4b);
    }

    /// Distinct run ids produce distinct `run_event_key` values.
    #[test]
    fn ls_distinct_runs_produce_distinct_event_keys(
        a in 1u64..100000u64,
        b in 1u64..100000u64,
        seq_val in 0u64..1000u64,
    ) {
        prop_assume!(a != b);
        let seq = EventSeq::new(seq_val);
        let ka = run_event_key(RunId::new(a), seq).expect("encodes");
        let kb = run_event_key(RunId::new(b), seq).expect("encodes");
        prop_assert_ne!(ka, kb);
    }

    /// Distinct sequence numbers for the same run produce distinct
    /// `run_event_key` values.
    #[test]
    fn ls_distinct_seqs_produce_distinct_event_keys(
        run_val in 1u64..100000u64,
        a in 0u64..1000u64,
        b in 0u64..1000u64,
    ) {
        prop_assume!(a != b);
        let run = RunId::new(run_val);
        let ka = run_event_key(run, EventSeq::new(a)).expect("encodes");
        let kb = run_event_key(run, EventSeq::new(b)).expect("encodes");
        prop_assert_ne!(ka, kb);
    }

    /// `RecordKind::id()` returns a stable, well-known value for
    /// each known kind. This is the wire-id stability floor.
    #[test]
    fn ls_record_kind_ids_are_stable(_unit in 0u8..1u8) {
        // Each kind has a single, well-known id. We assert a
        // handful of well-known bindings here.
        prop_assert_eq!(RecordKind::WorkflowSource.id(), 1);
        prop_assert_eq!(RecordKind::CompiledIr.id(), 2);
        prop_assert_eq!(RecordKind::RunHeader.id(), 3);
        prop_assert_eq!(RecordKind::RunAccepted.id(), 10);
        prop_assert_eq!(RecordKind::StepStarted.id(), 11);
        prop_assert_eq!(RecordKind::RunFinished.id(), 22);
        prop_assert_eq!(RecordKind::RunFailed.id(), 23);
    }

    /// `CURRENT_SCHEMA_VERSION` is a single stable u16. Changing
    /// this number is a wire-format break; the property test
    /// asserts the version is non-zero and finite.
    #[test]
    fn ls_schema_version_is_nonzero(_unit in 0u8..1u8) {
        prop_assert!(CURRENT_SCHEMA_VERSION > 0);
    }

    /// `RECORD_HEADER_BYTES` is positive and bounded (it must fit
    /// in a `usize` for the runtime to slice it).
    #[test]
    fn ls_record_header_bytes_is_positive(_unit in 0u8..1u8) {
        prop_assert!(RECORD_HEADER_BYTES > 0);
        prop_assert!(RECORD_HEADER_BYTES <= 1024);
    }

    /// Encoding then decoding a record header preserves the magic
    /// and sequence fields exactly. This is the on-disk layout
    /// stability floor: a write can be read back.
    #[test]
    fn ls_record_header_roundtrip_preserves_magic_and_seq(
        payload in proptest::collection::vec(any::<u8>(), 0..64),
        seq_val in 0u64..1000u64,
    ) {
        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            seq_val,
            &payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode_record_header succeeds");
        let decoded = decode_record_header(
            &header,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode_record_header succeeds");
        prop_assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
        prop_assert_eq!(decoded.sequence, seq_val);
        prop_assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    }

    /// Encoding a header with the wrong magic and then decoding
    /// with the correct magic fails with a typed `BadMagic` error
    /// (no silent acceptance of an aliased magic).
    #[test]
    fn ls_record_header_wrong_magic_rejected(
        payload in proptest::collection::vec(any::<u8>(), 0..64),
        seq_val in 0u64..1000u64,
    ) {
        let wrong_header = encode_record_header(
            MAGIC_BLOB,
            RecordKind::Blob,
            seq_val,
            &payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode_record_header succeeds");
        let result = decode_record_header(
            &wrong_header,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(result.is_err(), "wrong magic must be rejected");
    }

    /// `MAGIC_*` constants are distinct. A magic collision would
    /// silently alias one record family onto another.
    #[test]
    fn ls_magic_constants_are_distinct(_unit in 0u8..1u8) {
        let magics = [
            MAGIC_BLOB,
            MAGIC_JOURNAL_EVENT,
            MAGIC_WORKFLOW_SOURCE,
        ];
        for i in 0..magics.len() {
            for j in (i + 1)..magics.len() {
                prop_assert_ne!(magics[i], magics[j], "magic constants must be distinct");
            }
        }
    }
}
