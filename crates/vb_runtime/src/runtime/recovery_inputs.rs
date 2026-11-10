use super::{RecoveryArtifactWorkflow, RecoveryPrefixEvidence};
use crate::{RuntimeError, RuntimeResult};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{SlotIdx, SlotValue, Taint};

pub(super) struct RetainedInputs {
    pub(super) roots: Vec<(SlotIdx, SlotValue, Taint)>,
    pub(super) store: Option<ValueStoreCheckpointV1>,
}

pub(super) fn load(
    prefix: &RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
) -> RuntimeResult<Option<RetainedInputs>> {
    let admission = prefix
        .events
        .iter()
        .find(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RunAdmission { .. }
                    | vb_storage::JournalEvent::RunAdmissionBudget { .. }
            )
        })
        .ok_or_else(|| invalid("recovery admission missing"))?;
    if !crate::recovery::admission_is_new_bound(admission) {
        return Ok(None);
    }
    let record = journal
        .admitted_run_inputs(
            run,
            prefix.admission.artifact_digest(),
            &recovered.workflow,
            admission,
        )?
        .ok_or_else(|| invalid("retained input record missing for new-bound run"))?;
    match record {
        vb_storage::RetainedRunInputs::Scalar(record) => Ok(Some(RetainedInputs {
            roots: scalar_roots(&record, &recovered.workflow)?,
            store: None,
        })),
        vb_storage::RetainedRunInputs::Owned(record) => {
            let bundle = record.restore(&recovered.workflow)?;
            let mut roots = Vec::new();
            roots
                .try_reserve_exact(bundle.roots().len())
                .map_err(|_| invalid("retained root capacity"))?;
            roots.extend_from_slice(bundle.roots());
            Ok(Some(RetainedInputs {
                roots,
                store: Some(bundle.checkpoint().clone()),
            }))
        }
    }
}

fn scalar_roots(
    record: &vb_storage::RunInputsRecord,
    workflow: &vb_core::CompiledWorkflow,
) -> RuntimeResult<Vec<(SlotIdx, SlotValue, Taint)>> {
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
        *target = Some(if input.constraint.secret {
            Taint::Secret
        } else {
            Taint::Clean
        });
        Ok::<(), RuntimeError>(())
    })?;
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
