use super::support::{Case, TestResult};
use vb_core::{RuntimePolicy, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_storage::{JournalEvent, RunInputBinding};

#[path = "malformed/wire.rs"]
mod wire;

fn rejected(case: &Case) -> TestResult<RuntimeError> {
    let journal = case.open()?;
    let runtime = case.runtime(journal.clone(), 10)?;
    let before = journal.events_for_run_full(case.run)?;
    let error = runtime
        .activate_owned_submission(case.run, case.source.digest)
        .err()
        .ok_or("invalid durable state was activated")?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert!(matches!(
        runtime.inspect_run_state(case.run),
        Err(RuntimeError::RunNotFound)
    ));
    assert_eq!(journal.events_for_run_full(case.run)?, before);
    Ok(error)
}

#[test]
fn historical_unbound_and_wrong_complete_input_binding_are_refused() -> TestResult {
    for binding in [
        RunInputBinding::LegacyUnbound,
        RunInputBinding::Digest(WorkflowDigest::from_bytes([9; 32])),
    ] {
        let case = Case::new(false, RuntimePolicy::Strict)?;
        wire::admission(&case, |event| {
            let JournalEvent::RunAdmission { input_digest, .. } = event else {
                return Err("admission missing".into());
            };
            *input_digest = binding;
            Ok(())
        })?;
        assert!(
            matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
            if matches!(*source, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::AdmissionMismatch)))
        );
    }
    Ok(())
}

#[test]
fn wrong_artifact_receipt_and_missing_actual_artifact_are_refused() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    wire::admission(&case, |event| {
        let JournalEvent::RunAdmission {
            artifact_digest, ..
        } = event
        else {
            return Err("admission missing".into());
        };
        *artifact_digest = WorkflowDigest::from_bytes([6; 32]);
        Ok(())
    })?;
    assert!(matches!(
        rejected(&case)?,
        RuntimeError::AdmissionArtifactNotFound { .. }
    ));
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let key = vb_storage::keys::compiled_ir_key(case.artifact.digest.as_bytes())?;
    wire::replace(
        &case,
        vb_storage::constants::KEYSPACE_COMPILED_IR,
        &key,
        None,
    )?;
    assert!(matches!(
        rejected(&case)?,
        RuntimeError::AdmissionArtifactNotFound { .. }
    ));
    Ok(())
}

#[test]
fn missing_and_corrupt_owned_payload_cannot_create_partial_authority() -> TestResult {
    for payload in [None, Some(vec![0u8])] {
        let case = Case::new(false, RuntimePolicy::Strict)?;
        let key = vb_storage::keys::run_header_key(case.run)?;
        wire::replace(
            &case,
            vb_storage::constants::KEYSPACE_RUN_INPUTS,
            &key,
            payload,
        )?;
        let error = rejected(&case)?;
        assert!(matches!(
            error,
            RuntimeError::UnsupportedOperation {
                operation: "owned_activation_requires_retained_owned_inputs"
            } | RuntimeError::StorageJournalAppend { .. }
        ));
    }
    Ok(())
}

#[test]
fn actual_source_bytes_are_checked_not_just_digest_fields() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let mut source = case.source.clone();
    source.source.push(b' ');
    let bytes = vb_storage::encode_record(
        vb_storage::constants::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &source,
        vb_storage::constants::MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    let key = vb_storage::keys::workflow_source_key(case.source.digest.as_bytes())?;
    wire::replace(
        &case,
        vb_storage::constants::KEYSPACE_WORKFLOW_SOURCE,
        &key,
        Some(bytes),
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, vb_storage::JournalError::PayloadDigestMismatch))
    );
    Ok(())
}

#[test]
fn strict_artifact_cannot_upgrade_unpinned_or_non_strict_admission() -> TestResult {
    for change_policy in [false, true] {
        let case = Case::new(false, RuntimePolicy::Strict)?;
        wire::admission(&case, |event| {
            let JournalEvent::RunAdmission {
                policy, policy_pin, ..
            } = event
            else {
                return Err("admission missing".into());
            };
            if change_policy {
                *policy = RuntimePolicy::Relaxed;
            } else {
                *policy_pin = vb_storage::RunPolicyPin::Unverified;
            }
            Ok(())
        })?;
        assert!(
            matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
            if matches!(*source, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::IdentityMismatch)))
        );
    }
    Ok(())
}

#[test]
fn unsupported_owned_version_and_complete_digest_corruption_are_refused() -> TestResult {
    for wrong_version in [false, true] {
        let case = Case::new(false, RuntimePolicy::Strict)?;
        wire::owned_payload(&case, |bytes| {
            if wrong_version {
                let version = bytes.first_mut().ok_or("version missing")?;
                assert_eq!(*version, 1);
                *version = 2;
            } else {
                *bytes.last_mut().ok_or("digest missing")? ^= 1;
            }
            Ok(())
        })?;
        let error = rejected(&case)?;
        if wrong_version {
            assert!(
                matches!(error, RuntimeError::StorageJournalAppend { source }
                if matches!(*source, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::UnsupportedVersion { version: 2 })))
            );
        } else {
            assert!(
                matches!(error, RuntimeError::StorageJournalAppend { source }
                if matches!(*source, vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::DigestMismatch)))
            );
        }
    }
    Ok(())
}
