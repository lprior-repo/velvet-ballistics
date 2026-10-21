use super::*;
use crate::{EventSeq, JournalEvent};
use vb_core::{CapabilitySet, RuntimePolicy};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn retained() -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    Ok(RunInputsRecord::new(
        RunId::new(7),
        WorkflowDigest::from_bytes([1; 32]),
        WorkflowDigest::from_bytes([2; 32]),
        &super::tests::workflow()?,
        Box::from([(SlotIdx::new(0), SlotValue::I64(9))]),
    )?)
}

fn receipt(record: &RunInputsRecord) -> JournalEvent {
    JournalEvent::RunAdmission {
        run: record.run(),
        seq: EventSeq::new(1),
        artifact_digest: record.artifact_digest(),
        input_digest: RunInputBinding::Digest(record.input_digest()),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

#[test]
fn canonical_input_digest_binds_slots_values_and_types_without_order_dependency() -> TestResult {
    let first = (SlotIdx::new(0), SlotValue::I64(9));
    let second = (SlotIdx::new(1), SlotValue::Bool(true));
    let digest = compute_input_digest(&[first, second])?;
    assert_eq!(digest, compute_input_digest(&[second, first])?);
    for changed in [
        [(SlotIdx::new(2), SlotValue::I64(9)), second],
        [(SlotIdx::new(0), SlotValue::I64(10)), second],
        [(SlotIdx::new(0), SlotValue::Bool(true)), second],
    ] {
        assert_ne!(digest, compute_input_digest(&changed)?);
    }
    assert_ne!(digest, compute_input_digest(&[])?);
    assert!(matches!(
        compute_input_digest(&[first, first]),
        Err(JournalError::InputSchemaMismatch)
    ));
    Ok(())
}

#[test]
fn retained_input_receipt_rejects_legacy_digest_artifact_and_run_mismatch() -> TestResult {
    let record = retained()?;
    record.validate_admission(&receipt(&record))?;
    let recovered = crate::recovery::replay::recover_run_admission_from_events(&[receipt(&record)])
        .ok_or("missing admission summary")?;
    assert_eq!(
        recovered.input_digest,
        RunInputBinding::Digest(record.input_digest())
    );
    let mut bad_digest = receipt(&record);
    if let JournalEvent::RunAdmission { input_digest, .. } = &mut bad_digest {
        *input_digest = RunInputBinding::Digest(WorkflowDigest::from_bytes([9; 32]));
    }
    let mut legacy = receipt(&record);
    if let JournalEvent::RunAdmission { input_digest, .. } = &mut legacy {
        *input_digest = RunInputBinding::LegacyUnbound;
    }
    let mut bad_artifact = receipt(&record);
    if let JournalEvent::RunAdmission {
        artifact_digest, ..
    } = &mut bad_artifact
    {
        *artifact_digest = WorkflowDigest::from_bytes([9; 32]);
    }
    let mut bad_run = receipt(&record);
    if let JournalEvent::RunAdmission { run, .. } = &mut bad_run {
        *run = RunId::new(8);
    }
    for event in [bad_digest, legacy, bad_artifact, bad_run] {
        assert!(matches!(
            record.validate_admission(&event),
            Err(JournalError::PayloadDigestMismatch)
        ));
    }
    Ok(())
}

#[test]
fn retained_value_tampering_with_recomputed_envelope_checksum_is_rejected() -> TestResult {
    let mut record = retained()?;
    record.inputs = Box::from([(SlotIdx::new(0), SlotValue::I64(10))]);
    assert!(matches!(
        record.validate_workflow(&super::tests::workflow()?),
        Err(JournalError::PayloadDigestMismatch)
    ));
    let bytes = crate::encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunInputs,
        7,
        &record,
        MAX_RUN_INPUT_BYTES,
    )?;
    assert!(matches!(
        crate::decode_record::<RunInputsRecord>(&bytes, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES),
        Err(JournalError::PayloadDigestMismatch)
    ));
    // A new internally consistent input record still cannot impersonate the committed receipt.
    let original = retained()?;
    record.input_digest = compute_input_digest(&record.inputs)?;
    assert!(matches!(
        record.validate_admission(&receipt(&original)),
        Err(JournalError::PayloadDigestMismatch)
    ));
    Ok(())
}
