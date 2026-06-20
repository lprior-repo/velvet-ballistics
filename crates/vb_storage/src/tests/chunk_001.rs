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
fn journal_key_is_fixed_width() {
    // Given a run id 1 and event sequence 2
    // When the journal key is constructed
    // Then the key is exactly 17 bytes wide
    let key = journal_key(RunId::new(1), EventSeq::new(2));

    let key = key.expect("journal key construction should succeed");
    assert_eq!(key.len(), 17);
}

#[test]
fn run_event_key_uses_required_prefix_and_big_endian_layout() {
    // Given run id 0x0102030405060708 and event sequence 9
    // When the run event key is constructed
    // Then the layout is [0x11 prefix][run id big-endian][seq big-endian]
    let key = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9));

    let key = key.expect("run event key construction should succeed");
    let expected: [u8; 17] = [
        0x11, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x09,
    ];
    assert_eq!(key.as_slice(), expected.as_slice());
}

#[test]
fn key_encoders_use_required_lengths() {
    // Given a standard 32-byte digest and common run/step identifiers
    // When each key encoder is called
    // Then the produced keys have the contract-specified byte widths
    let digest = [7_u8; 32];

    let ws = workflow_source_key(digest).expect("workflow_source_key should succeed");
    assert_eq!(ws.len(), 33);

    let ci = compiled_ir_key(digest).expect("compiled_ir_key should succeed");
    assert_eq!(ci.len(), 33);

    let rh = run_header_key(RunId::new(1)).expect("run_header_key should succeed");
    assert_eq!(rh.len(), 9);

    let rs =
        run_snapshot_key(RunId::new(1), EventSeq::new(2)).expect("run_snapshot_key should succeed");
    assert_eq!(rs.len(), 17);

    let bl = blob_key(digest).expect("blob_key should succeed");
    assert_eq!(bl.len(), 33);

    let is = index_status_key(IndexStatusState::Other(3), 4, RunId::new(5))
        .expect("index_status_key should succeed");
    assert_eq!(is.len(), 18);

    let iw = index_workflow_key(WorkflowId::new(6), RunId::new(7))
        .expect("index_workflow_key should succeed");
    assert_eq!(iw.len(), 13);

    let ia = index_action_key(ActionId::new(8), RunId::new(9), StepIdx::new(10))
        .expect("index_action_key should succeed");
    assert_eq!(ia.len(), 13);
}

#[test]
fn encode_key_dispatches_to_existing_key_encoders() {
    let digest = [9_u8; 32];

    let run_event = encode_key(StorageKey::RunEvent {
        run: RunId::new(0x0102_0304_0506_0708),
        seq: EventSeq::new(9),
    })
    .expect("encode_key should encode run event key");
    let expected_run_event = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9))
        .expect("run_event_key should succeed")
        .to_vec();
    assert_eq!(run_event, expected_run_event);

    let blob = encode_key(StorageKey::Blob { digest }).expect("encode_key should encode blob key");
    let expected_blob = blob_key(digest).expect("blob_key should succeed").to_vec();
    assert_eq!(blob, expected_blob);
}

#[test]
fn record_header_wrappers_encode_decode_and_verify_digest() {
    let payload = b"compact payload";

    let header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        7,
        payload,
        128,
    )
    .expect("record header encoding should succeed");
    assert_eq!(header.len(), RECORD_HEADER_BYTES);

    let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 128)
        .expect("record header decoding should succeed");
    assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    assert_eq!(decoded.payload_len, 15);
    assert_eq!(decoded.sequence, 7);
    verify_digest_match(payload, decoded.payload_digest)
        .expect("payload digest should match decoded header");
}

#[test]
fn verify_digest_match_rejects_mismatched_payload() {
    let digest = *blake3::hash(b"original").as_bytes();

    let result = verify_digest_match(b"changed", digest);

    assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
}

#[test]
fn free_put_wrappers_delegate_to_journal_methods() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let journal = open_store(temp.path()).expect("journal should open");
    let source_bytes = vec![b'a'];
    let workflow_digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let compiled = crate::try_accepted_compiled_ir_record_for_test(vec![b'i'])
        .expect("test fixture should encode");
    let compiled_digest = compiled.digest;
    let blob_bytes = vec![b'b'];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();

    let source = WorkflowSourceRecord {
        digest: workflow_digest,
        source: source_bytes,
    };
    put_workflow_source(&journal, &source).expect("workflow source should store");
    let stored_source = journal
        .workflow_source(workflow_digest)
        .expect("workflow source lookup should succeed");
    assert_eq!(stored_source, Some(source));

    journal
        .put_compiled_ir(&compiled)
        .expect("compiled ir should store");
    let stored_compiled = journal
        .compiled_ir(compiled_digest)
        .expect("compiled ir lookup should succeed");
    assert_eq!(stored_compiled, Some(compiled));

    let header = RunHeaderRecord {
        run: RunId::new(11),
        workflow_id: WorkflowId::new(12),
        compiled_digest,
        status: 1,
        accepted_at_ms: 13,
    };
    put_run_header(&journal, &header).expect("run header should store");
    let stored_header = journal
        .run_header(RunId::new(11))
        .expect("run header lookup should succeed");
    assert_eq!(stored_header, Some(header));

    let blob = BlobRecord {
        digest: blob_digest,
        bytes: blob_bytes,
    };
    put_blob(&journal, &blob).expect("blob should store");
    let stored_blob = read_blob(&journal, blob_digest).expect("blob lookup should succeed");
    assert_eq!(stored_blob, Some(blob));
}

#[test]
fn envelope_round_trips_and_reports_metadata() {
    // Given a RunFinished journal event with run 99, seq 12, result slot 1
    // When the event is encoded and then decoded
    // Then the envelope metadata and deserialized event match the originals
    let event = JournalEvent::RunFinished {
        run: RunId::new(99),
        seq: EventSeq::new(12),
        result: vb_core::SlotIdx::new(1),
        attempt: 1,
    };

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFinished,
        event.seq().get(),
        &event,
        128,
    );
    let encoded = encoded.expect("encoding should succeed");
    assert!(encoded.len() > 60, "encoded record must exceed header size");

    let (envelope, decoded_event) =
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunFinished.id());
    assert_eq!(envelope.sequence, 12);
    assert_eq!(decoded_event, event);
}
