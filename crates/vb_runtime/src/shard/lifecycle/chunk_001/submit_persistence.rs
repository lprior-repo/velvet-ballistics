use super::*;
use crate::journal::RuntimeJournalEvent as Event;
use vb_storage::RunInputsRecord;

impl Header {
    pub(super) fn persist(
        &self,
        shard: &mut Shard,
        workflow: &CompiledWorkflow,
        inputs: Vec<(SlotIdx, SlotValue)>,
        frame: RunFrame,
        persist_header: bool,
    ) -> RuntimeResult<(RunFrame, u64)> {
        if persist_header && let Some(admission) = &self.admission {
            let batch = self.prepare_batch(shard, workflow, inputs, admission);
            return self.append_batch(shard, frame, batch);
        }
        let bytes = self.append_individual(shard, persist_header)?;
        Ok((frame, bytes))
    }

    fn prepare_batch(
        &self,
        shard: &Shard,
        workflow: &CompiledWorkflow,
        inputs: Vec<(SlotIdx, SlotValue)>,
        admission: &RunAdmission,
    ) -> RuntimeResult<AdmissionBatch> {
        let retained_inputs = self.retained_inputs(shard, workflow, inputs, admission)?;
        let mut events = vec![
            self.submitted(),
            Event::RunAdmission {
                admission: admission.clone(),
            },
        ];
        for event in self.additional_events(admission) {
            events.push(event);
        }
        let bytes = shard.journal_batch_bytes(&events)?;
        Ok(AdmissionBatch {
            retained_inputs,
            events,
            bytes,
        })
    }

    fn retained_inputs(
        &self,
        shard: &Shard,
        workflow: &CompiledWorkflow,
        inputs: Vec<(SlotIdx, SlotValue)>,
        admission: &RunAdmission,
    ) -> RuntimeResult<Option<RunInputsRecord>> {
        if shard.is_volatile_admission() {
            return Ok(None);
        }
        let record = RunInputsRecord::new(
            self.run,
            admission.artifact_digest(),
            self.digest,
            workflow,
            inputs.into_boxed_slice(),
        )?;
        Ok(Some(record))
    }

    fn append_batch(
        &self,
        shard: &mut Shard,
        frame: RunFrame,
        batch: RuntimeResult<AdmissionBatch>,
    ) -> RuntimeResult<(RunFrame, u64)> {
        let batch = match batch {
            Ok(batch) => batch,
            Err(error) => {
                shard.release_frame(frame);
                shard.release_admission_quota(self.run)?;
                return Err(error);
            }
        };
        let result = match batch.retained_inputs {
            Some(inputs) => shard.append_admission_journal_event_batch(&batch.events, inputs),
            None => shard.append_journal_event_batch(&batch.events),
        };
        if let Err(error) = result {
            shard.release_frame(frame);
            shard.release_admission_quota(self.run)?;
            shard.discard_journal_sequence(self.run);
            return Err(RuntimeError::admission_header_persistence_failed(error));
        }
        Ok((frame, batch.bytes))
    }

    fn append_individual(&self, shard: &mut Shard, persist_header: bool) -> RuntimeResult<u64> {
        let mut bytes = 0u64;
        let mut append = |event: Event| {
            bytes = bytes
                .checked_add(shard.journal_event_bytes(&event)?)
                .ok_or(RuntimeError::EncodeFailed)?;
            shard.append_admission_header_journal_event(self.run, event)
        };
        if persist_header {
            append(self.submitted())?;
        }
        if let Some(admission) = &self.admission {
            append(Event::RunAdmission {
                admission: admission.clone(),
            })?;
            self.additional_events(admission).try_for_each(append)?;
        }
        Ok(bytes)
    }

    fn submitted(&self) -> Event {
        Event::RunSubmitted {
            run: self.run,
            workflow: self.digest,
        }
    }

    fn additional_events(&self, admission: &RunAdmission) -> impl Iterator<Item = Event> {
        let run = self.run;
        let compatibility = admission
            .ir_compat_decision()
            .map(|decision| Event::CompatibilityDecision { run, decision });
        let tenant = crate::journal::tenant_admission_event(admission, run);
        let origin = admission.budget().map(|_| Event::RunBudgetOrigin {
            run,
            started_millis: self.started_millis,
        });
        compatibility.into_iter().chain(tenant).chain(origin)
    }
}

struct AdmissionBatch {
    retained_inputs: Option<RunInputsRecord>,
    events: Vec<Event>,
    bytes: u64,
}
