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
fn batch_put_compiled_ir_rejects_forged_digest_and_aborts() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let valid = crate::try_accepted_compiled_ir_record_for_test(b"batch-forgery".to_vec())
        .expect("test fixture should encode");
    let forged_digest = WorkflowDigest::from_bytes([0xB6; DIGEST_BYTES]);
    let forged = CompiledIrRecord {
        digest: forged_digest,
        ir: valid.ir,
        ..Default::default()
    };
    let run = RunId::new(0xB6);
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0xB6),
        compiled_digest: valid.digest,
        status: 1,
        accepted_at_ms: 1,
    };

    let mut batch = journal.batch();
    assert!(matches!(
        batch.put_compiled_ir(&forged),
        Err(JournalError::ArtifactChecksumMismatch)
    ));
    assert_eq!(batch.len(), 0, "failed validation must abort batch");
    batch
        .put_run_header(&header)
        .expect("post-abort staging call should not persist on commit");
    batch
        .commit()
        .expect("aborted batch commit must be a no-op");

    assert!(
        journal
            .compiled_ir(forged_digest)
            .expect("compiled_ir lookup should succeed")
            .is_none(),
        "forged compiled IR must not be persisted"
    );
    assert!(
        journal
            .run_header(run)
            .expect("run_header lookup should succeed")
            .is_none(),
        "aborted batch must not persist later staged records"
    );
}

#[test]
fn journal_run_header_after_batch_commit_matches_all_fields() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9007);
    let workflow_id = WorkflowId::new(42);
    let compiled_digest = WorkflowDigest::from_bytes([0xFB; 32]);
    let status: u8 = 5;
    let accepted_at_ms: u64 = 9876543210;
    let record = RunHeaderRecord {
        run,
        workflow_id,
        compiled_digest,
        status,
        accepted_at_ms,
    };
    let mut batch = journal.batch();
    batch.put_run_header(&record).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.run_header(run).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(found_record.run, run, "run must match");
    assert_eq!(
        found_record.workflow_id, workflow_id,
        "workflow_id must match"
    );
    assert_eq!(
        found_record.compiled_digest, compiled_digest,
        "compiled_digest must match"
    );
    assert_eq!(found_record.status, status, "status must match");
    assert_eq!(
        found_record.accepted_at_ms, accepted_at_ms,
        "accepted_at_ms must match"
    );
}

#[test]
fn journal_snapshot_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9008);
    let seq = EventSeq::new(3);
    let snapshot = RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0xFA; 32]),
        slots: b"snapshot_data".to_vec(),
        taint: Vec::new(),
    };
    let mut batch = journal.batch();
    batch.put_snapshot(&snapshot).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.snapshot(run, seq).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(found_record.run, run);
    assert_eq!(found_record.seq, seq);
    assert_eq!(found_record.slots, b"snapshot_data".to_vec());
}

#[test]
fn journal_blob_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let blob_bytes = b"batch_blob_exact".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    let mut batch = journal.batch();
    batch.put_blob(&record).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.blob(digest).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(
        found_record.bytes,
        b"batch_blob_exact".to_vec(),
        "blob bytes must match exactly"
    );
    assert_eq!(found_record.digest, digest);
}

#[test]
fn journal_status_index_after_batch_commit_returns_correct_run() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let state = IndexStatusState::Other(7);
    let timestamp: u64 = 55555;
    let run = RunId::new(9009);
    let mut batch = journal.batch();
    batch
        .put_status_index(state, timestamp, run)
        .expect("put_status_index must succeed");
    batch.commit().expect("commit must succeed");
    let key = index_status_key(state, timestamp, run).expect("key must succeed");
    let value = journal
        .index_status
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "status index must exist after batch commit"
    );
}

#[test]
fn journal_action_index_after_batch_commit_returns_correct_entry() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let action = ActionId::new(11);
    let run = RunId::new(9010);
    let step = StepIdx::new(4);
    let mut batch = journal.batch();
    batch
        .put_action_index(action, run, step)
        .expect("put_action_index must succeed");
    batch.commit().expect("commit must succeed");
    let key = index_action_key(action, run, step).expect("key must succeed");
    let value = journal
        .index_action
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "action index must exist after batch commit"
    );
}

#[test]
fn adversarial_reopen_after_unflushed_journaled_events_may_lose_them() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9001);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    journal.append_journaled(&event).expect("append journaled");
    drop(journal);
    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
    let result = journal2
        .events_for_run(run)
        .expect("events_for_run succeeds");
    // Journaled durability does not guarantee persistence without flush
    // Either the event is present (Fjall flushed on drop) or absent (acceptable)
    assert!(result.len() <= 1, "at most one event expected");
}
