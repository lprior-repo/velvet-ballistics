#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]
use super::prelude::*;

#[test]
fn envelope_round_trips_workflow_source_record() {
    // Given a WorkflowSourceRecord
    // When encoded and decoded with MAGIC_WORKFLOW_SOURCE
    // Then the record survives the round trip
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        source: vec![1, 2, 3],
    };
    let encoded = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    )
    .expect("encoding should succeed");

    let (envelope, decoded) =
        decode_record::<WorkflowSourceRecord>(&encoded, MAGIC_WORKFLOW_SOURCE, 128)
            .expect("decoding should succeed");
    assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
    assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
    assert_eq!(decoded, record);
}

#[test]
fn envelope_round_trips_compiled_ir_record() {
    // Given a CompiledIrRecord
    // When encoded and decoded with MAGIC_COMPILED_ARTIFACT
    // Then the record survives the round trip
    let record = crate::try_accepted_compiled_ir_record_for_test(vec![4, 5, 6])
        .expect("test fixture should encode");
    let encoded = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("encoding should succeed");

    let (envelope, decoded) =
        decode_record::<CompiledIrRecord>(&encoded, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES)
            .expect("decoding should succeed");
    assert_eq!(envelope.magic, MAGIC_COMPILED_ARTIFACT);
    assert_eq!(envelope.record_kind, RecordKind::CompiledIr.id());
    assert_eq!(decoded, record);
}

#[test]
fn envelope_round_trips_blob_record() {
    // Given a BlobRecord
    // When encoded and decoded with MAGIC_BLOB
    // Then the record survives the round trip
    let record = BlobRecord {
        digest: [0xDD; 32],
        bytes: vec![7, 8, 9],
    };
    let encoded =
        encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 128).expect("encoding ok");

    let (envelope, decoded) =
        decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, 128).expect("decoding ok");
    assert_eq!(envelope.magic, MAGIC_BLOB);
    assert_eq!(envelope.record_kind, RecordKind::Blob.id());
    assert_eq!(decoded, record);
}

#[test]
fn declared_keyspaces_returns_ten_entries() {
    // Given FjallJournal::declared_keyspaces()
    // When called
    // Then it returns exactly 11 keyspace names (run_seq_gap was added in vb-1rqz7.1)
    let keyspaces = FjallJournal::declared_keyspaces();
    assert_eq!(keyspaces.len(), 11);
    assert_eq!(keyspaces[0], "workflow_source");
    assert_eq!(keyspaces[1], "compiled_ir");
    assert_eq!(keyspaces[2], "run_header");
    assert_eq!(keyspaces[3], "run_event");
    assert_eq!(keyspaces[4], "run_snapshot");
    assert_eq!(keyspaces[5], "blob");
    assert_eq!(keyspaces[6], "index_status");
    assert_eq!(keyspaces[7], "index_workflow");
    assert_eq!(keyspaces[8], "index_action");
    assert_eq!(keyspaces[9], "recovery_stamp");
    assert_eq!(keyspaces[10], "run_seq_gap");
}

#[test]
fn run_header_returns_none_for_missing_run() {
    // Given an open journal with no stored headers
    // When run_header is called for an arbitrary run
    // Then it returns None
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal
        .run_header(RunId::new(999))
        .expect("lookup should succeed");
    assert_eq!(result, None);
}

#[test]
fn compiled_ir_returns_none_for_missing_digest() {
    // Given an open journal with no stored IR
    // When compiled_ir is called
    // Then it returns None
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal
        .compiled_ir(WorkflowDigest::from_bytes([0; 32]))
        .expect("lookup should succeed");
    assert_eq!(result, None);
}

#[test]
fn snapshot_returns_none_for_missing_entry() {
    // Given an open journal with no snapshots
    // When snapshot is called
    // Then it returns None
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal
        .snapshot(RunId::new(1), EventSeq::new(0))
        .expect("lookup should succeed");
    assert_eq!(result, None);
}

#[test]
fn blob_returns_none_for_missing_digest() {
    // Given an open journal with no blobs
    // When blob is called
    // Then it returns None
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal.blob([0; 32]).expect("lookup should succeed");
    assert_eq!(result, None);
}

#[test]
fn journal_open_creates_fresh_instance_with_no_data() {
    // Given a temporary directory
    // When FjallJournal::open is called
    // Then the journal has no events for any run
    let (_guard, journal) = open_journal();
    let events = journal
        .events_for_run(RunId::new(1))
        .expect("events_for_run should succeed on empty journal");
    assert!(events.is_empty());
}

#[test]
fn append_strict_writes_submitted_event_with_correct_run_id() {
    // Given an open journal
    // When append_strict is called with a RunAccepted event for run 42
    // Then the stored event has run_id 42
    let (_guard, journal) = open_journal();
    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].run_id(), run);
}

#[test]
fn append_strict_writes_accepted_event_after_submitted() {
    // Given an open journal with a RunAccepted event at seq 0
    // When a StepStarted event at seq 1 is appended
    // Then both events are retrieved in order
    let (_guard, journal) = open_journal();
    let run = RunId::new(1);
    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let started = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal
        .append_strict(&accepted)
        .expect("journal.append_strict must succeed");
    journal
        .append_strict(&started)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], accepted);
    assert_eq!(events[1], started);
}
