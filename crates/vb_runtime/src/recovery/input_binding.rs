#![forbid(unsafe_code)]
//! Input binding validation for cold startup recovery.

use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue, Taint, ValueStore, WorkflowDigest};
use vb_storage::{FjallJournal, JournalEvent, RetainedRunInputs, RunInputsRecord};

use crate::{RuntimeError, RuntimeResult};

/// Validates that a retained input record matches an admission event's binding.
/// Returns Err if the binding is inconsistent or the record is for a different run.
pub fn validate_input_record_binding(
    record: &RunInputsRecord,
    run: RunId,
    admission: &JournalEvent,
) -> Result<(), RuntimeError> {
    if record.run() != run {
        return Err(RuntimeError::Recovery {
            error: format!(
                "retained input record run mismatch: expected {:?}, got {:?}",
                run,
                record.run()
            ),
        });
    }
    record
        .validate_admission(admission)
        .map_err(|err| RuntimeError::Recovery {
            error: format!("retained input admission binding invalid: {}", err),
        })
}

/// Validates that retained inputs are consistent with the compiled workflow.
pub fn validate_input_record_workflow(
    record: &RunInputsRecord,
    workflow: &CompiledWorkflow,
) -> Result<(), RuntimeError> {
    if record.workflow_digest() != workflow.digest() {
        return Err(RuntimeError::Recovery {
            error: format!(
                "retained input workflow digest mismatch: expected {:?}, got {:?}",
                workflow.digest(),
                record.workflow_digest()
            ),
        });
    }
    record
        .validate_workflow(workflow)
        .map_err(|err| RuntimeError::Recovery {
            error: format!("retained input workflow validation failed: {}", err),
        })
}

/// Validates that a retained record matches the expected artifact digest.
pub fn validate_input_record_artifact(
    record: &RunInputsRecord,
    expected_artifact: WorkflowDigest,
) -> Result<(), RuntimeError> {
    if record.artifact_digest() != expected_artifact {
        return Err(RuntimeError::Recovery {
            error: format!(
                "retained input artifact digest mismatch: expected {:?}, got {:?}",
                expected_artifact,
                record.artifact_digest()
            ),
        });
    }
    Ok(())
}

/// Checks if the event carries the current budgeted and bound admission shape.
pub fn admission_is_new_bound(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::RunAdmissionBudget {
            input_digest: vb_storage::RunInputBinding::Digest(_),
            policy_pin: vb_storage::RunPolicyPin::Pinned(_),
            ..
        }
    )
}

pub(crate) fn current_admission_event(events: &[JournalEvent]) -> Option<&JournalEvent> {
    let mut admissions = events.iter().filter(|event| {
        matches!(
            event,
            JournalEvent::RunAdmission { .. } | JournalEvent::RunAdmissionBudget { .. }
        )
    });
    let admission = admissions.next()?;
    if admissions.next().is_some() {
        return None;
    }
    if admission_is_new_bound(admission) {
        Some(admission)
    } else {
        None
    }
}

/// Retained inputs restored for cold startup recovery.
///
/// Scalar roots retain declaration taint. Owned runs carry the
/// complete validated bundle: canonical roots with their taints plus the full
/// value-store checkpoint, so recovered `List`/`Object`/`Blob` handles
/// resolve against a restored arena instead of dangling.
pub(crate) struct RecoveredRetainedInputs {
    initial_inputs: Vec<(SlotIdx, SlotValue, Taint)>,
    store: Option<ValueStore>,
}

impl RecoveredRetainedInputs {
    /// Empty input state for recovery tests and runs with no input slots.
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            initial_inputs: Vec::new(),
            store: None,
        }
    }

    /// Initial slot seeds (with taints) applied before journal slot writes.
    pub(crate) fn initial_inputs(&self) -> &[(SlotIdx, SlotValue, Taint)] {
        &self.initial_inputs
    }

    /// Restored cold value store for arena-handle resolution, when the run
    /// retained owned inputs.
    pub(crate) fn store(&self) -> Option<&ValueStore> {
        self.store.as_ref()
    }

    /// Versioned checkpoint of the restored store for the recovery command.
    pub(crate) fn store_checkpoint(&self) -> Option<vb_core::value_store::ValueStoreCheckpointV1> {
        self.store.as_ref().map(ValueStore::checkpoint_v1)
    }
}

/// Loads and validates retained typed inputs for cold startup recovery.
///
/// Current admissions require their retained record: scalar records seed plain
/// slot values, owned records additionally restore the value-store arenas that
/// recovered `List`/`Object`/`Blob` handles point into.
pub(crate) fn load_retained_inputs_for_recovery(
    events: &[JournalEvent],
    journal: &FjallJournal,
    artifact: &vb_storage::admission::AcceptedArtifact,
    workflow: &CompiledWorkflow,
    run: RunId,
) -> RuntimeResult<RecoveredRetainedInputs> {
    let admission_event = current_admission_for_run(events, run)?;
    let retained = journal
        .admitted_run_inputs(run, artifact.digest, workflow, admission_event)?
        .ok_or(RuntimeError::Recovery {
            error: format!("recovery: retained input record missing for new-bound run {run:?}"),
        })?;
    match retained {
        RetainedRunInputs::Scalar(record) => {
            scalar_retained_inputs(&record, run, admission_event, artifact.digest, workflow)
        }
        RetainedRunInputs::Owned(record) => {
            owned_retained_inputs(&record, workflow, run, admission_event)
        }
    }
}

fn current_admission_for_run(events: &[JournalEvent], run: RunId) -> RuntimeResult<&JournalEvent> {
    let admission_event =
        current_admission_event(events).ok_or(RuntimeError::RecoveryCannotResume {
            reason: String::from("current_admission_missing_or_invalid"),
        })?;
    if admission_event.run_id() != run {
        return Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_run_mismatch"),
        });
    }
    Ok(admission_event)
}

fn scalar_retained_inputs(
    record: &RunInputsRecord,
    run: RunId,
    admission_event: &JournalEvent,
    artifact_digest: WorkflowDigest,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<RecoveredRetainedInputs> {
    validate_input_record_binding(record, run, admission_event)?;
    validate_input_record_artifact(record, artifact_digest)?;
    validate_input_record_workflow(record, workflow)?;
    let initial_inputs = scalar_roots(record, workflow)?;
    Ok(RecoveredRetainedInputs {
        initial_inputs,
        store: None,
    })
}

fn owned_retained_inputs(
    record: &vb_storage::OwnedRunInputsRecordV1,
    workflow: &CompiledWorkflow,
    run: RunId,
    admission_event: &JournalEvent,
) -> RuntimeResult<RecoveredRetainedInputs> {
    if record.run() != run {
        return Err(RuntimeError::Recovery {
            error: format!(
                "recovery: retained owned input run mismatch: expected {run:?}, got {:?}",
                record.run()
            ),
        });
    }
    // Binds the record to the durable admission: the admission's input digest
    // covers the record (including its embedded source digest), so the
    // presented arenas are the admitted ones.
    record.validate_admission(admission_event)?;
    let bundle = record.restore(workflow)?;
    let store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    Ok(RecoveredRetainedInputs {
        initial_inputs: record.roots().to_vec(),
        store: Some(store),
    })
}

fn scalar_roots(
    record: &vb_storage::RunInputsRecord,
    workflow: &vb_core::CompiledWorkflow,
) -> RuntimeResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let taints = declared_input_taints(workflow)?;
    roots_with_taints(record, &taints)
}

fn declared_input_taints(
    workflow: &vb_core::CompiledWorkflow,
) -> RuntimeResult<Vec<Option<Taint>>> {
    let mut taints = Vec::new();
    let slots = usize::from(workflow.slot_count());
    taints
        .try_reserve_exact(slots)
        .map_err(|_| invalid("retained declaration capacity"))?;
    taints.resize(slots, None);
    workflow.input_slots().iter().try_for_each(|input| {
        let target = taints
            .get_mut(input.slot.as_usize())
            .ok_or_else(|| invalid("retained declaration slot"))?;
        *target = Some(if input.constraint.secretness.is_secret() {
            Taint::Secret
        } else {
            Taint::Clean
        });
        Ok::<(), RuntimeError>(())
    })?;
    Ok(taints)
}

fn roots_with_taints(
    record: &vb_storage::RunInputsRecord,
    taints: &[Option<Taint>],
) -> RuntimeResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(record.inputs().len())
        .map_err(|_| invalid("retained input capacity"))?;
    record.inputs().iter().try_for_each(|(slot, value)| {
        let taint = taints
            .get(slot.as_usize())
            .copied()
            .flatten()
            .ok_or_else(|| invalid("retained input declaration missing"))?;
        roots.push((*slot, *value, taint));
        Ok::<(), RuntimeError>(())
    })?;
    Ok(roots)
}

fn invalid(error: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: error.into(),
    }
}
