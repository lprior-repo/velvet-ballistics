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
fn batch_builder_round_trips_via_append_strict_batch() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(64);
    let mut builder = BatchBuilder::new();
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    });
    builder.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    });

    journal
        .append_strict_batch(builder.as_slice())
        .expect("journal.append_strict_batch must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 2);
}

#[test]
fn flush_profile_batches_strict_events_into_single_fsync() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = open_store(temp_dir.path()).expect("setup: journal open");
    let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
        return;
    };
    let run = RunId::new(58);
    let strict1 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([6; 32]),
    };
    let strict2 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    queue
        .enqueue_strict(strict1.clone())
        .expect("queue.enqueue_strict must succeed");
    queue
        .enqueue_strict(strict2.clone())
        .expect("queue.enqueue_strict must succeed");
    let report = flush_profile(&queue, &journal);

    let report = report.expect("flush_profile should succeed");
    assert_eq!(report.drained, 2);
    assert_eq!(report.written, 2);
    let events = read_run_events(&journal, run);
    let events = events.expect("read_run_events should succeed");
    assert_eq!(events, vec![strict1, strict2]);
}

#[test]
fn write_batch_commits_cross_keyspace_atomically() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let source_bytes = b"test workflow".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let run = RunId::new(42);

    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: source_bytes,
        })
        .expect("put_workflow_source must succeed");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(7),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 1234,
        })
        .expect("put_run_header must succeed");
    batch.commit().expect("batch.commit must succeed");

    let source = journal
        .workflow_source(digest)
        .expect("workflow source roundtrip");
    assert!(
        source.is_some(),
        "workflow source must be present after cross-keyspace commit"
    );
    let source_record = source.unwrap();
    assert_eq!(source_record.source, b"test workflow".to_vec());
    assert_eq!(source_record.digest, digest);

    let header = journal.run_header(run).expect("run header roundtrip");
    assert!(
        header.is_some(),
        "run header must be present after cross-keyspace commit"
    );
    let header_record = header.unwrap();
    assert_eq!(header_record.run, run);
    assert_eq!(header_record.workflow_id, WorkflowId::new(7));
    assert_eq!(header_record.compiled_digest, digest);
    assert_eq!(header_record.status, 1);
}

#[test]
fn write_batch_strict_commits_with_durability() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let blob_bytes = b"blob data".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let mut batch = journal.batch().strict();
    batch
        .put_blob(&BlobRecord {
            digest,
            bytes: blob_bytes,
        })
        .expect("action must succeed");
    batch.commit().expect("batch.commit must succeed");

    let blob = journal.blob(digest).expect("blob roundtrip");
    assert!(
        blob.is_some(),
        "blob must be present after strict batch commit, got {:?}",
        blob
    );
    let blob_record = blob.unwrap();
    assert_eq!(blob_record.bytes, b"blob data".to_vec());
    assert_eq!(blob_record.digest, digest);
}

#[test]
fn write_batch_appends_events_and_indexes() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(99);
    let workflow = WorkflowId::new(5);
    let action = ActionId::new(3);
    let step = StepIdx::new(2);

    let mut batch = journal.batch();
    batch
        .append_event(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        })
        .expect("action must succeed");
    batch
        .put_workflow_index(workflow, run)
        .expect("batch.put_workflow_index must succeed");
    batch
        .put_action_index(action, run, step)
        .expect("batch.put_action_index must succeed");
    batch
        .put_status_index(IndexStatusState::Submitted, 5678, run)
        .expect("batch.put_status_index must succeed");
    batch.commit().expect("batch.commit must succeed");

    let events = journal.events_for_run(run);
    let events = events.expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
}

#[test]
fn write_batch_empty_commit_succeeds() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    batch.commit().expect("batch.commit must succeed");
}

#[test]
fn write_batch_is_empty_after_construction() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

#[test]
fn write_batch_len_tracks_operations() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let source = b"a".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest, source })
        .expect("action must succeed");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());

    let compiled = crate::try_accepted_compiled_ir_record_for_test(b"ir".to_vec())
        .expect("test fixture should encode");
    batch
        .put_compiled_ir(&compiled)
        .expect("action must succeed");
    assert_eq!(batch.len(), 2);
}
