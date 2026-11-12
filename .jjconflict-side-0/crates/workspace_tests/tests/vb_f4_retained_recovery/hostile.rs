use super::*;
use super::{owned::Case, wire};
use vb_core::root_bundle::OwnedRootBundleError;
use vb_runtime::{RuntimeError, recovery::RuntimeRecoveryProduct};
use vb_storage::{JournalError, OwnedInputError};

fn rejected(case: &Case) -> TestResult<RuntimeError> {
    let journal = case.open()?;
    let runtime = runtime(journal.clone());
    let before = journal.events_for_run_full(RUN)?;
    let error = runtime
        .recover_and_resume(RUN)
        .err()
        .ok_or("forged input resumed")?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert!(matches!(
        runtime.inspect_run_state(RUN),
        Err(RuntimeError::RunNotFound)
    ));
    assert_eq!(journal.events_for_run_full(RUN)?, before);
    Ok(error)
}

#[test]
fn forged_owned_digest_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            wire.digest = WorkflowDigest::from_bytes([9; 32]);
            Ok(())
        },
        false,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::DigestMismatch)))
    );
    Ok(())
}

#[test]
fn mismatched_admission_does_not_authorize_an_intact_owned_record() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::admission_digest(&case, WorkflowDigest::from_bytes([9; 32]))?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::AdmissionMismatch)))
    );
    Ok(())
}

#[test]
fn self_consistent_wrong_artifact_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            wire.body.artifact = WorkflowDigest::from_bytes([9; 32]);
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::IdentityMismatch)))
    );
    Ok(())
}

#[test]
fn self_consistent_wrong_workflow_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            wire.body.workflow = WorkflowDigest::from_bytes([9; 32]);
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::ProjectionMismatch)))
    );
    Ok(())
}

#[test]
fn self_consistent_missing_source_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            wire.body.source = WorkflowDigest::from_bytes([9; 32]);
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::IdentityMismatch)))
    );
    Ok(())
}

#[test]
fn self_consistent_wrong_schema_projection_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            wire.body
                .inputs
                .first_mut()
                .ok_or("declaration")?
                .constraint
                .extra = vb_core::ExtraFields::Allow;
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::ProjectionMismatch)))
    );
    Ok(())
}

#[test]
fn self_consistent_wrong_recursive_value_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            *wire
                .body
                .checkpoint
                .lists
                .first_mut()
                .ok_or("list")?
                .first_mut()
                .ok_or("item")? = SlotValue::Bool(true);
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::Bundle(
            OwnedRootBundleError::RootConstraintMismatch { slot, .. }
        )) if slot == case.input))
    );
    Ok(())
}

#[test]
fn self_consistent_secret_descendant_with_clean_root_is_not_recovered() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    wire::tamper(
        &case,
        |wire| {
            *wire
                .body
                .checkpoint
                .list_taints
                .first_mut()
                .ok_or("taints")?
                .first_mut()
                .ok_or("taint")? = vb_core::Taint::Secret;
            Ok(())
        },
        true,
    )?;
    assert!(
        matches!(rejected(&case)?, RuntimeError::StorageJournalAppend { source }
        if matches!(*source, JournalError::OwnedInputs(OwnedInputError::Bundle(
            OwnedRootBundleError::TaintMismatch { slot, actual: vb_core::Taint::Clean, .. }
        )) if slot == case.input))
    );
    Ok(())
}

#[test]
fn journal_owned_slot_writes_still_require_unknown_runtime_arenas() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    let journal = case.open()?;
    journal.append_journaled(&vb_storage::JournalEvent::SlotWrittenEvent {
        run: RUN,
        seq: vb_storage::EventSeq::new(3),
        slot: case.input,
        value: Some(postcard::to_allocvec(&SlotValue::List(
            vb_core::ListId::new(999),
        ))?),
        extra: Some(
            vb_storage::encode_slot_written_extra(vb_core::Taint::Clean, None)
                .map_err(|error| format!("slot extra: {error:?}"))?,
        ),
        attempt: 1,
    })?;
    journal.persist_strict()?;
    drop(journal);
    let journal = case.open()?;
    let runtime = runtime(journal.clone());
    let product = runtime.recover_product(RUN)?;
    assert!(
        matches!(product, RuntimeRecoveryProduct::CannotResume(product) if product.reason() == "store_missing")
    );
    assert!(
        matches!(runtime.recover_and_resume(RUN), Err(RuntimeError::RecoveryCannotResume { reason }) if reason == "store_missing")
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert_eq!(journal.events_for_run_full(RUN)?.len(), 4);
    Ok(())
}
