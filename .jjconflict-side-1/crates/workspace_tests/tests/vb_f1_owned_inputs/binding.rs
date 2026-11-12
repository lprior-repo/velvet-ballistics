use super::support::{SOURCE, TestResult, fixture, record};
use super::wire::{decode, encode, fixture_wire};
use vb_core::{RunId, Taint, WorkflowDigest};
use vb_storage::{JournalError, JournalEvent, OwnedInputError, OwnedRunInputsRecordV1, RecordKind};

#[test]
fn digest_binds_complete_content_item_taints_root_taints_and_run() -> TestResult {
    let a = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let b = fixture(9, Taint::DerivedFromSecret, Taint::Secret)?;
    let c = fixture(7, Taint::Clean, Taint::Secret)?;
    let d = fixture(7, Taint::DerivedFromSecret, Taint::TimeDependent)?;
    let digest = record(&a)?.input_digest();
    for f in [&b, &c, &d] {
        assert_ne!(digest, record(f)?.input_digest());
    }
    let other_run = OwnedRunInputsRecordV1::new(
        RunId::new(18),
        a.artifact.digest,
        a.source.digest,
        &a.bundle,
        &a.workflow,
    )?;
    assert_ne!(digest, other_run.input_digest());
    let changed = SOURCE.replace("name: owned_inputs", "name: owned_inputs_changed");
    let workflow =
        vb_compile::compile_workflow(changed.as_bytes()).map_err(|e| format!("{e:?}"))?;
    assert!(matches!(
        OwnedRunInputsRecordV1::new(
            a.run,
            a.artifact.digest,
            a.source.digest,
            &a.bundle,
            &workflow
        ),
        Err(JournalError::OwnedInputs(
            OwnedInputError::ProjectionMismatch
        ))
    ));
    Ok(())
}

#[test]
fn explicit_identity_and_admission_bindings_are_mandatory() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let bytes = encode(&fixture_wire(&f)?, RecordKind::RunOwnedInputs)?;
    assert!(matches!(
        OwnedRunInputsRecordV1::decode(
            &bytes,
            RunId::new(99),
            f.artifact.digest,
            f.source.digest,
            &f.workflow
        ),
        Err(JournalError::WrongRun { .. })
    ));
    let wrong = WorkflowDigest::from_bytes([99; 32]);
    for (artifact, source) in [(wrong, f.source.digest), (f.artifact.digest, wrong)] {
        assert!(matches!(
            OwnedRunInputsRecordV1::decode(&bytes, f.run, artifact, source, &f.workflow),
            Err(JournalError::OwnedInputs(OwnedInputError::IdentityMismatch))
        ));
    }
    Ok(())
}

#[test]
fn actual_complete_admission_digest_is_required() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let bytes = encode(&fixture_wire(&f)?, RecordKind::RunOwnedInputs)?;
    let wrong = WorkflowDigest::from_bytes([99; 32]);
    let record = decode(&f, &bytes)?;
    let event = JournalEvent::RunAdmission {
        run: f.run,
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: f.artifact.digest,
        input_digest: vb_storage::RunInputBinding::Digest(wrong),
        policy_pin: vb_storage::RunPolicyPin::Pinned(f.artifact.policy_digest),
        granted_capabilities: vb_core::CapabilitySet::empty(),
        policy: vb_core::RuntimePolicy::Strict,
    };
    assert!(matches!(
        record.validate_admission(&event),
        Err(JournalError::OwnedInputs(
            OwnedInputError::AdmissionMismatch
        ))
    ));
    Ok(())
}
