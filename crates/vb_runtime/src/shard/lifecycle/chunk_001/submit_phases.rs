use super::*;
use crate::admission::RunAdmission;
use crate::shard::helpers;
use vb_core::{RunFrame, WorkflowDigest};

mod persistence {
    include!("submit_persistence.rs");
}

struct Submission<'a> {
    run: RunId,
    workflow: CompiledWorkflow,
    contracts: &'a [ActionContract],
    options: SubmitOptions,
}

struct PreparedInputs {
    values: Vec<(SlotIdx, SlotValue)>,
    store: ValueStore,
}

struct Header {
    run: RunId,
    digest: WorkflowDigest,
    started_millis: u64,
    admission: Option<RunAdmission>,
}

impl Shard {
    pub(super) fn handle_submit_with_inputs_contracts_and_header_mode(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
        contracts: &[ActionContract],
        options: SubmitOptions,
    ) -> RuntimeResult<()> {
        let submission = Submission {
            run,
            workflow,
            contracts,
            options,
        };
        submission.submit(self, inputs, caps)
    }
}

impl Submission<'_> {
    fn submit(
        self,
        shard: &mut Shard,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        validate_capacity(shard, self.run)?;
        let inputs = self.prepare_inputs(shard, inputs)?;
        let header = self.header(shard, &inputs.values, caps)?;
        let frame = self.prepare_frame(shard, &inputs.values, header.admission.as_ref())?;
        let (frame, bytes) = header.persist(
            shard,
            &self.workflow,
            inputs.values,
            frame,
            self.options.persist_header,
        )?;
        shard.counters.inc_submitted();
        let run = self.run;
        let state = self.into_state(frame, inputs.store, header, bytes);
        admit_and_drive(shard, run, state)
    }

    fn prepare_inputs(
        &self,
        shard: &Shard,
        inputs: &[(SlotIdx, SlotValue)],
    ) -> RuntimeResult<PreparedInputs> {
        let mut store = ValueStore::with_max_slots(self.workflow.resource_contract().max_slots);
        shard.validate_secret_presence(&self.workflow)?;
        let secrets = shard.prepare_secret_inputs(&self.workflow, inputs)?;
        let values = helpers::prepare_input_slots(&secrets, &self.workflow)?;
        if let Some(metadata) = self.workflow.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        Ok(PreparedInputs { values, store })
    }

    fn header(
        &self,
        shard: &Shard,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<Header> {
        let digest = self.workflow.digest();
        let input_digest =
            vb_storage::inputs::compute_input_digest(inputs).map_err(RuntimeError::from)?;
        let started_millis = crate::shard::helpers::submission_time()?;
        let admission = self.bind_inputs(shard, caps, input_digest)?;
        Ok(Header {
            run: self.run,
            digest,
            started_millis,
            admission,
        })
    }

    fn bind_inputs(
        &self,
        shard: &Shard,
        caps: CapabilitySet,
        input_digest: WorkflowDigest,
    ) -> RuntimeResult<Option<RunAdmission>> {
        let admission = shard.build_admission_with_budget(
            self.run,
            &self.workflow,
            caps,
            self.options.lifetime_cap,
        )?;
        Ok(admission.map(|admission| {
            let bound =
                admission.with_input_digest(vb_storage::RunInputBinding::Digest(input_digest));
            match self.options.tenant {
                Some(tenant) => bound.with_tenant(tenant),
                None => bound,
            }
        }))
    }

    fn prepare_frame(
        &self,
        shard: &mut Shard,
        inputs: &[(SlotIdx, SlotValue)],
        admission: Option<&RunAdmission>,
    ) -> RuntimeResult<RunFrame> {
        execution_binding::validate(
            shard.artifact_store.as_ref(),
            shard.policy,
            &self.workflow,
            self.contracts,
        )?;
        let run = self.run;
        shard.prepare_run_slots(run)?;
        let mut frame = shard.take_frame_for(run, &self.workflow)?;
        if let Err(error) = helpers::seed_input_slots(&mut frame, inputs, &self.workflow) {
            shard.release_frame(frame);
            return Err(error);
        }
        shard.trace_ring.push(TraceEvent::RunSubmitted { run });
        if let Err(error) = reserve_quota(shard, run, admission) {
            shard.release_frame(frame);
            return Err(error);
        }
        Ok(frame)
    }

    fn into_state(
        self,
        frame: RunFrame,
        store: ValueStore,
        header: Header,
        journal_bytes_written: u64,
    ) -> RunState {
        let action_attempts = helpers::new_action_attempts(frame.step_count());
        RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
            frame,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow: self.workflow,
            store,
            action_attempts,
            admission: header.admission,
            run_started_millis: Some(header.started_millis),
            collect_states: CollectStates::new(),
            journal_bytes_written,
            action_contracts: self.contracts.to_vec().into_boxed_slice(),
        }
    }
}

fn reserve_quota(
    shard: &mut Shard,
    run: RunId,
    admission: Option<&RunAdmission>,
) -> RuntimeResult<()> {
    if let Some(admission) = admission
        && let Some(budget) = admission.budget()
    {
        shard.reserve_admission_quota(run, admission.tenant(), budget.max_cost_units)?;
    }
    Ok(())
}

fn validate_capacity(shard: &Shard, run: RunId) -> RuntimeResult<()> {
    if shard.run_state_contains(run) {
        return Err(RuntimeError::RunAlreadyExists);
    }
    if shard.active_run_count() >= shard.max_active_runs {
        return Err(RuntimeError::ActiveRunCapacityExceeded {
            capacity: shard.max_active_runs,
        });
    }
    Ok(())
}

fn admit_and_drive(shard: &mut Shard, run: RunId, state: RunState) -> RuntimeResult<()> {
    shard.terminal_runs_remove(run);
    if let Err(error) = shard.admit_run_state(run, state, RuntimeState::Initial) {
        if let Some(state) = shard.run_state_remove(run) {
            shard.release_frame(state.frame);
        }
        shard.release_admission_quota(run)?;
        shard.discard_journal_sequence(run);
        return Err(error);
    }
    if let Err(error) = shard.drive_run(run) {
        release_unowned_submission_after_drive_failure(shard, run)?;
        return Err(error);
    }
    Ok(())
}

// Failed drive may leave checked-out or runtime ownership after a partial
// terminal transition. Retain its accounting and cursor until cleanup resolves.
fn release_unowned_submission_after_drive_failure(
    shard: &mut Shard,
    run: RunId,
) -> RuntimeResult<()> {
    if shard.run_state_contains(run)
        || shard.checked_out_run_contains(run)
        || shard.runtime_state_get(run).is_some()
    {
        return Ok(());
    }
    shard.release_admission_quota(run)?;
    shard.discard_journal_sequence(run);
    Ok(())
}

#[cfg(test)]
mod drive_failure_accounting_tests {
    include!("submit_phases_tests.rs");
}
