use super::*;

struct LegacyDurableAdapter(StorageRuntimeJournal);

impl RuntimeJournal for LegacyDurableAdapter {
    fn probe(&self) -> crate::RuntimeResult<()> {
        self.0.probe()
    }
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.0.append(event)
    }
    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.0.append_sequenced_batch(events, seq)
    }
    fn durability_profile(&self) -> Option<DurabilityProfile> {
        Some(DurabilityProfile::Strict)
    }
    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.0.storage_journal()
    }
}

#[test]
fn review_custom_durable_port_cannot_acknowledge_missing_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, journal) = temp_journal()?;
    let adapter = LegacyDurableAdapter(StorageRuntimeJournal::strict(journal.clone()));
    let mut parts = single_finish_workflow(WorkflowDigest::from_bytes([0; 32]))?.to_parts();
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )?;
    let run = RunId::new(702);
    let inputs = vb_storage::RunInputsRecord::new(
        run,
        artifact.digest,
        workflow.digest(),
        &workflow,
        Box::from([(SlotIdx::new(0), vb_core::SlotValue::I64(11))]),
    )?;
    let admission = crate::admission::RunAdmission::new(
        artifact.digest,
        run,
        vb_core::CapabilitySet::empty(),
        vb_core::RuntimePolicy::Strict,
    )
    .with_input_digest(vb_storage::RunInputBinding::Digest(inputs.input_digest()));
    let events = [
        RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: workflow.digest(),
        },
        RuntimeJournalEvent::RunAdmission { admission },
    ];
    let result = adapter.append_admission_batch(&events, EventSeq::new(0), inputs.clone());
    if result.is_ok() {
        assert_eq!(journal.run_inputs(run)?, Some(inputs));
    } else {
        assert!(journal.events_for_run(run)?.is_empty());
        assert!(journal.run_inputs(run)?.is_none());
    }
    Ok(())
}
