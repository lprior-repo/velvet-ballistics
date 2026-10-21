#![forbid(unsafe_code)]
//! Input binding validation for cold startup recovery.

use vb_core::{CompiledWorkflow, RunId, WorkflowDigest};
use vb_storage::{JournalEvent, RunInputsRecord};

use crate::RuntimeError;

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

/// Checks if the admission event is new-bound (has input digest) or legacy unbound.
pub fn admission_is_new_bound(event: &JournalEvent) -> bool {
    match event {
        JournalEvent::RunAdmission { input_digest, .. } => {
            matches!(input_digest, vb_storage::RunInputBinding::Digest(_))
        }
        _ => false,
    }
}
