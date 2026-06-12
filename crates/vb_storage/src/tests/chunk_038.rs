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
    clippy::unwrap_used
)]
use super::prelude::*;


#[test]
fn adversarial_compiled_ir_with_same_digest_rewrites_valid_envelope() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let first = crate::accepted_compiled_ir_record_for_test(b"version".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"caps_test".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"warnings_test".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"idempotency_test".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"idemp_attested_test".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"flags_test".to_vec());
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
    let first = crate::accepted_compiled_ir_record_for_test(b"batch_mut_test".to_vec());
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
