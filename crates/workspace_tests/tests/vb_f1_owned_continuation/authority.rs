use super::support::{TestResult, at_cut};
use core::num::NonZeroU16;
use vb_core::{ActionId, WorkflowDigest};
use vb_runtime::run_continuation::{
    ContinuationAuthority, RunContinuationError, RunContinuationV1,
};
use vb_storage::{JournalEvent, RunInputBinding};

#[test]
fn checked_input_record_requires_the_actual_complete_admission_binding() -> TestResult {
    let fixture = at_cut(false)?;
    let mut admission = fixture.admission.clone();
    let JournalEvent::RunAdmission { input_digest, .. } = &mut admission else {
        return Err("wrong receipt".into());
    };
    *input_digest = RunInputBinding::LegacyUnbound;
    let result = ContinuationAuthority::from_owned_record(
        &fixture.record,
        &admission,
        &fixture.artifact,
        fixture.expected,
    );
    assert!(matches!(result, Err(RunContinuationError::Storage(error))
        if matches!(*error, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::AdmissionMismatch))));
    if let JournalEvent::RunAdmission { input_digest, .. } = &mut admission {
        *input_digest = RunInputBinding::Digest(WorkflowDigest::from_bytes([9; 32]));
    }
    assert!(
        ContinuationAuthority::from_owned_record(
            &fixture.record,
            &admission,
            &fixture.artifact,
            fixture.expected,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn independently_selected_source_attempt_and_prior_sequence_are_checked() -> TestResult {
    let fixture = at_cut(false)?;
    let mut expected = fixture.expected;
    expected.source_digest = WorkflowDigest::from_bytes([8; 32]);
    assert!(matches!(
        ContinuationAuthority::from_owned_record(
            &fixture.record,
            &fixture.admission,
            &fixture.artifact,
            expected,
        ),
        Err(RunContinuationError::Identity)
    ));
    let authority = fixture.authority()?;
    let checkpoint = RunContinuationV1::capture(&fixture.state, &authority)?;
    let mut wrong = fixture.expected;
    wrong.run_attempt = NonZeroU16::new(2).ok_or("zero attempt")?;
    let authority = ContinuationAuthority::from_owned_record(
        &fixture.record,
        &fixture.admission,
        &fixture.artifact,
        wrong,
    )?;
    assert!(matches!(
        checkpoint.validate_against(&authority),
        Err(RunContinuationError::AttemptIdentity)
    ));
    wrong = fixture.expected;
    wrong.prior_seq = vb_storage::EventSeq::new(2);
    let authority = ContinuationAuthority::from_owned_record(
        &fixture.record,
        &fixture.admission,
        &fixture.artifact,
        wrong,
    )?;
    assert!(matches!(
        checkpoint.validate_against(&authority),
        Err(RunContinuationError::SequenceIdentity)
    ));
    Ok(())
}

#[test]
fn historical_idempotency_evidence_is_never_discarded_to_fit_hydration() -> TestResult {
    let mut fixture = at_cut(false)?;
    let admission = fixture
        .state
        .admission
        .as_ref()
        .ok_or("admission missing")?;
    let mut rich = serde_json::to_value(admission)?;
    *rich
        .get_mut("idempotency_attested")
        .ok_or("attestation field missing")? = serde_json::to_value([ActionId::new(1)])?;
    fixture.state.admission = Some(serde_json::from_value(rich)?);
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &fixture.authority()?),
        Err(RunContinuationError::Admission)
    ));
    Ok(())
}

#[test]
fn exact_payload_version_digest_and_consumption_are_enforced() -> TestResult {
    let fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let checkpoint = RunContinuationV1::capture(&fixture.state, &authority)?;
    assert!(matches!(
        RunContinuationV1::from_bytes(&[2], &authority),
        Err(RunContinuationError::UnsupportedVersion { version: 2 })
    ));
    assert!(matches!(
        RunContinuationV1::from_bytes(&[], &authority),
        Err(RunContinuationError::Codec(_))
    ));
    let mut bytes = checkpoint.to_bytes()?;
    bytes.push(0);
    assert!(matches!(
        RunContinuationV1::from_bytes(&bytes, &authority),
        Err(RunContinuationError::TrailingBytes)
    ));
    bytes.pop().ok_or("missing trailing byte")?;
    *bytes.last_mut().ok_or("missing digest")? ^= 1;
    assert!(matches!(
        RunContinuationV1::from_bytes(&bytes, &authority),
        Err(RunContinuationError::Digest)
    ));
    Ok(())
}

#[test]
fn historical_budget_is_never_discarded_to_fit_hydration() -> TestResult {
    let mut fixture = at_cut(false)?;
    let admission = fixture
        .state
        .admission
        .as_ref()
        .ok_or("admission missing")?;
    let mut rich = serde_json::to_value(admission)?;
    let budget = vb_core::budget::AggregateResourceBudget::from_workflow(&fixture.state.workflow)?;
    *rich.get_mut("budget").ok_or("budget field missing")? = serde_json::to_value(budget)?;
    fixture.state.admission = Some(serde_json::from_value(rich)?);
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &fixture.authority()?),
        Err(RunContinuationError::Admission)
    ));
    Ok(())
}

#[test]
fn run_artifact_and_required_policy_are_independently_checked() -> TestResult {
    let fixture = at_cut(false)?;
    let mut wrong_run = fixture.expected;
    wrong_run.run = vb_core::RunId::new(18);
    let mut wrong_artifact = fixture.expected;
    wrong_artifact.artifact_digest = WorkflowDigest::from_bytes([7; 32]);
    for expected in [wrong_run, wrong_artifact] {
        assert!(matches!(
            ContinuationAuthority::from_owned_record(
                &fixture.record,
                &fixture.admission,
                &fixture.artifact,
                expected,
            ),
            Err(RunContinuationError::Identity)
        ));
    }
    let mut wrong_policy = fixture.expected;
    wrong_policy.policy = vb_core::RuntimePolicy::Relaxed;
    assert!(matches!(ContinuationAuthority::from_owned_record(
        &fixture.record, &fixture.admission, &fixture.artifact, wrong_policy,
    ), Err(RunContinuationError::Storage(error))
        if matches!(*error, vb_storage::JournalError::ArtifactMalformed)));
    Ok(())
}
