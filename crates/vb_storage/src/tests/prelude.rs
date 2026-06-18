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

pub(crate) use crate::keys::{
    blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key, index_workflow_key,
    journal_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
};
pub(crate) use crate::queue::BatchBuilder;
pub(crate) use crate::recovery::{ActionReplayTracker, RunSnapshot};
pub(crate) use crate::{
    BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, DIGEST_BYTES, EventSeq, FjallJournal,
    IndexStatusState, JournalError, JournalEvent, JournalWriterQueue, KeyspaceProfile, MAGIC_BLOB,
    MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT,
    MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES,
    MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
    PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
    PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
    RecordKind, RunHeaderRecord, StorageKey, StorageLimits, WorkflowSourceRecord,
    append_journal_event, decode_record, decode_record_header, encode_record, encode_record_header,
    flush_profile, init_keyspaces, keyspace_options_for, open_store, put_blob, put_run_header,
    put_workflow_source, read_blob, read_run_events, replay_journal, verify_digest_match,
    write_snapshot,
};
pub(crate) use vb_core::{
    ActionId, CODE_REGISTRY, CapabilitySet, DiagnosticCode, RunId, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest, WorkflowId,
};

// --- Section 4: Journal Lifecycle BDD Tests ---

pub(crate) fn open_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal should open");
    (temp_dir, journal)
}

pub(crate) fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

// =========================================================================
// Section: Adversarial Record Header Decode Tests
// =========================================================================

pub(crate) fn encode_and_patch_field(
    event: &JournalEvent,
    kind: RecordKind,
    offset: usize,
    new_bytes: &[u8],
) -> Vec<u8> {
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    let end = offset.saturating_add(new_bytes.len());
    assert!(end <= 56, "patch must be within CRC-protected region");
    encoded
        .get_mut(offset..end)
        .expect("patch range valid")
        .copy_from_slice(new_bytes);
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;
    encoded
}
