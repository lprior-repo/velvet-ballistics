use super::{Fixture, TestResult};
use vb_core::RuntimePolicy;
use vb_storage::{JournalEvent, OwnedRunInputsRecordV1};

pub(super) fn assert_submission_events(
    events: &[JournalEvent],
    f: &Fixture,
    loaded: &OwnedRunInputsRecordV1,
    expected: &OwnedRunInputsRecordV1,
) -> TestResult {
    match events {
        [
            accepted @ JournalEvent::RunAccepted { .. },
            admission @ JournalEvent::RunAdmission { .. },
            compatibility @ JournalEvent::CompatibilityDecisionEvent { .. },
        ] => {
            assert_accepted(accepted, f)?;
            assert_admission_identity(admission, f, expected)?;
            assert_admission_authority(admission, f)?;
            loaded.validate_admission(admission)?;
            assert_compatibility(compatibility, f)?;
        }
        _ => {
            return Err("submission events missing or out of order".into());
        }
    }
    Ok(())
}

fn assert_accepted(event: &JournalEvent, f: &Fixture) -> TestResult {
    let JournalEvent::RunAccepted { run, seq, workflow } = event else {
        return Err("submission events missing or out of order".into());
    };
    assert_eq!(*run, f.run);
    assert_eq!(seq.get(), 0);
    assert_eq!(*workflow, f.workflow.digest());
    Ok(())
}

fn assert_admission_identity(
    admission: &JournalEvent,
    f: &Fixture,
    expected: &OwnedRunInputsRecordV1,
) -> TestResult {
    let JournalEvent::RunAdmission {
        run: admission_run,
        seq: admission_seq,
        artifact_digest,
        input_digest,
        ..
    } = admission
    else {
        return Err("submission events missing or out of order".into());
    };
    assert_eq!(*admission_run, f.run);
    assert_eq!(admission_seq.get(), 1);
    assert_eq!(*artifact_digest, f.artifact.digest);
    assert_eq!(
        *input_digest,
        vb_storage::RunInputBinding::Digest(expected.input_digest())
    );
    Ok(())
}

fn assert_admission_authority(admission: &JournalEvent, f: &Fixture) -> TestResult {
    let JournalEvent::RunAdmission {
        policy_pin,
        granted_capabilities,
        policy,
        ..
    } = admission
    else {
        return Err("submission events missing or out of order".into());
    };
    assert_eq!(
        *policy_pin,
        vb_storage::RunPolicyPin::Pinned(f.artifact.policy_digest)
    );
    assert_eq!(*granted_capabilities, vb_core::CapabilitySet::empty());
    assert_eq!(*policy, RuntimePolicy::Strict);
    Ok(())
}

fn assert_compatibility(event: &JournalEvent, f: &Fixture) -> TestResult {
    let JournalEvent::CompatibilityDecisionEvent {
        run: compat_run,
        seq: compat_seq,
        decision,
    } = event
    else {
        return Err("submission events missing or out of order".into());
    };
    assert_eq!(*compat_run, f.run);
    assert_eq!(compat_seq.get(), 2);
    let expected = vb_storage::compat::decide_boundary(
        vb_storage::compat::artifact_ir_version_inputs(f.artifact.artifact_version),
        vb_storage::compat::schema_version_digest(f.artifact.artifact_version),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
    );
    assert_eq!(*decision, expected);
    Ok(())
}
