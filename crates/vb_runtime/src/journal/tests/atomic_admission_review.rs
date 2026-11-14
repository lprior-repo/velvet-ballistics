struct LegacyDurableAdapter(StorageRuntimeJournal);

impl RuntimeJournal for LegacyDurableAdapter {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.0.append(event)
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.0.append_sequenced_batch(events, start_seq)
    }

    fn durability_profile(&self) -> Option<DurabilityProfile> {
        Some(DurabilityProfile::Strict)
    }

    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.0.storage_journal()
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        self.0.probe()
    }
}

#[test]
fn durable_default_port_fails_closed_and_volatile_default_remains_nonpersistent()
-> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let workflow = single_finish_workflow(WorkflowDigest::from_bytes([0; 32]))?;
    let mut workflow_parts = workflow.to_parts();
    workflow_parts.digest = WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&workflow_parts).map_err(|error| error.to_string())?)
            .into(),
    );
    let workflow = vb_core::CompiledWorkflow::try_from_parts(workflow_parts)
        .map_err(|error| error.to_string())?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .map_err(|error| error.to_string())?;
    let run = RunId::new(702);
    let inputs = vb_storage::RunInputsRecord::new(
        run,
        artifact.digest,
        workflow.digest(),
        &workflow,
        Box::from([(SlotIdx::new(0), vb_core::SlotValue::I64(11))]),
    )
    .map_err(|error| error.to_string())?;
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

    let durable = LegacyDurableAdapter(StorageRuntimeJournal::strict(journal.clone()));
    let result = durable.append_admission_batch(&events, EventSeq::new(0), inputs.clone());
    assert!(
        matches!(
            result,
            Err(crate::RuntimeError::UnsupportedOperation {
                operation: "runtime_journal_admission_batch"
            })
        ),
        "{result:?}"
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    assert!(
        journal
            .run_inputs(run)
            .map_err(|error| error.to_string())?
            .is_none()
    );

    let volatile = VolatileRuntimeJournal::new();
    volatile
        .append_admission_batch(&events, EventSeq::new(0), inputs)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        volatile
            .snapshot()
            .map_err(|error| error.to_string())?
            .len(),
        2
    );
    Ok(())
}
