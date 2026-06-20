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
fn adversarial_compiled_ir_with_same_digest_rewrites_valid_envelope() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"version".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    second_artifact.accepted_at_seq = EventSeq::new(1);
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    // SECURITY: Second write with mutated metadata must be rejected
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "metadata mutation must be detected and rejected"
    );
    // Original record must remain unchanged
    let loaded = journal
        .compiled_ir(first.digest)
        .expect("get")
        .expect("exists");
    assert_eq!(loaded, first, "original record must be preserved");
}

/// VB-FN4VT PO-006: Metadata mutation via required_capabilities is rejected.
#[test]
fn metadata_hash_rejects_required_capabilities_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"caps_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    // Mutate required_capabilities - the first has empty caps, this adds a capability
    let cap = vb_core::capability::Capability::new(
        Box::<str>::from("network.test"),
        vb_core::ActionId::new(42),
    );
    second_artifact.required_capabilities = Box::new([cap]);
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "required_capabilities mutation must be rejected"
    );
}

/// VB-FN4VT PO-006: Metadata mutation via warnings is rejected.
#[test]
fn metadata_hash_rejects_warnings_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"warnings_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    // Mutate warnings
    second_artifact
        .verification
        .warnings
        .push(crate::admission::VerificationWarning {
            code: 999,
            message: "forged warning".into(),
            gate: 1,
        });
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "warnings mutation must be rejected"
    );
}

/// VB-FN4VT PO-006: Metadata mutation via idempotency evidence is rejected.
#[test]
fn metadata_hash_rejects_idempotency_evidence_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"idempotency_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    // Mutate idempotency_keyed - add an action id
    second_artifact.verification.idempotency_keyed = Box::new([vb_core::ActionId::new(99)]);
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "idempotency evidence mutation must be rejected"
    );
}

/// VB-FN4VT PO-006: Metadata hash covers `idempotency_attested` actions.
#[test]
fn metadata_hash_rejects_idempotency_attested_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"idemp_attested_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    // Mutate idempotency_attested - add an attested action id
    second_artifact.verification.idempotency_attested = Box::new([vb_core::ActionId::new(42)]);
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "idempotency_attested mutation must be rejected"
    );
}

/// VB-FN4VT PO-006: Metadata hash covers verification proof flags.
#[test]
fn metadata_hash_rejects_verification_flags_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::try_accepted_compiled_ir_record_for_test(b"flags_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    // Mutate a verification flag
    second_artifact.verification.taint_safe_claimed = false;
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    journal.put_compiled_ir(&first).expect("put1");
    let err = journal
        .put_compiled_ir(&second)
        .expect_err("put2 must fail");
    // Validation rejects the mutated artifact BEFORE metadata hash comparison.
    // The artifact has taint_safe_claimed=false, which fails proof validation.
    // This is correct behavior: bad artifacts are rejected at validation gate.
    assert!(
        matches!(err, JournalError::MissingRequiredProofFlag { flag } if flag == "taint_safe"),
        "mutated artifact must be rejected at validation gate, got {:?}",
        err
    );
}

/// VB-FN4VT PO-006: Batch path also rejects metadata mutation.
#[test]
fn batch_put_compiled_ir_rejects_metadata_mutation() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut batch = journal.batch();
    let first = crate::try_accepted_compiled_ir_record_for_test(b"batch_mut_test".to_vec())
        .expect("test fixture should encode");
    let mut second_artifact: crate::AcceptedArtifact =
        postcard::from_bytes(&first.ir).expect("accepted artifact should decode");
    second_artifact.accepted_at_seq = EventSeq::new(999);
    let second_ir =
        postcard::to_allocvec(&second_artifact).expect("accepted artifact should encode");
    let second = CompiledIrRecord {
        digest: first.digest,
        ir: second_ir,
        ..Default::default()
    };
    batch.put_compiled_ir(&first).expect("put1");
    let err = batch
        .put_compiled_ir(&second)
        .expect_err("batch put2 must fail");
    assert!(
        matches!(err, JournalError::MetadataMutation { digest } if digest == first.digest),
        "batch must also reject metadata mutation"
    );
}
