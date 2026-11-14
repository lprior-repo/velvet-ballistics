mod default_binding {
    use std::error::Error;
    use std::sync::Arc;

    use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind, node};
    use vb_core::{SlotValue, WorkflowDigest};

    use super::{CapabilitySet, CompiledNodeKind, RunId, Shard, ShardConfig, SlotIdx, StepIdx};
    use crate::journal::{
        RuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal, StorageRuntimeJournal,
        VolatileRuntimeJournal,
    };

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    fn input_workflow(default: Option<SlotValue>) -> TestResult<vb_core::CompiledWorkflow> {
        let mut parts = super::finished_workflow()
            .ok_or("base workflow fixture failed")?
            .to_parts();
        parts.name = "scalar-default-input".into();
        parts.nodes = Box::from([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]);
        parts.slot_count = 2;
        parts.input_slots = Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::I64,
            constraint: CompiledInputConstraint {
                default_value: default,
                ..CompiledInputConstraint::unconstrained()
            },
        }]);
        parts.digest = WorkflowDigest::from_bytes([0; 32]);
        parts.digest =
            WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
        Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
    }

    fn one_step_config() -> ShardConfig {
        ShardConfig {
            step_budget_per_tick: 1,
            ..super::small_config()
        }
    }

    fn assert_frame_input(shard: &Shard, run: RunId, expected: Option<SlotValue>) -> TestResult {
        let state = shard.run_state_get(run).ok_or("run frame missing")?;
        match expected {
            Some(value) => assert_eq!(state.frame.read_slot(SlotIdx::ZERO)?, &value),
            None => assert!(state.frame.read_slot(SlotIdx::ZERO).is_err()),
        }
        assert!(state.journal_bytes_written > 0);
        Ok(())
    }

    #[test]
    fn scalar_default_input_binding_matches_frame_and_retained_record() -> TestResult {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/runtime-default-binding-tests");
        std::fs::create_dir_all(&root)?;
        let directory = tempfile::Builder::new()
            .prefix("scalar-default-")
            .tempdir_in(&root)?;
        let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
        let mut shard = Shard::new_with_journal_and_artifact_store(
            one_step_config(),
            StorageRuntimeJournal::shared_journaled(journal.clone()),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        );
        let default_workflow = input_workflow(Some(SlotValue::I64(7)))?;
        let cases = [
            (
                RunId::new(741),
                default_workflow.clone(),
                Vec::new(),
                Some(SlotValue::I64(7)),
            ),
            (
                RunId::new(742),
                default_workflow,
                vec![(SlotIdx::ZERO, SlotValue::I64(9))],
                Some(SlotValue::I64(9)),
            ),
            (RunId::new(743), input_workflow(None)?, Vec::new(), None),
        ];
        for (run, workflow, supplied, expected) in cases {
            shard.handle_submit_with_inputs(run, workflow, &supplied, CapabilitySet::empty())?;
            assert_frame_input(&shard, run, expected)?;
            let expected_mapping: Vec<_> = expected
                .into_iter()
                .map(|value| (SlotIdx::ZERO, value))
                .collect();
            let expected_digest = vb_storage::inputs::compute_input_digest(&expected_mapping)?;
            let state = shard.run_state_get(run).ok_or("run state missing")?;
            let admission = state
                .admission
                .as_ref()
                .ok_or("runtime admission missing")?;
            assert_eq!(
                admission.input_digest(),
                vb_storage::RunInputBinding::Digest(expected_digest)
            );
            let retained = journal.run_inputs(run)?.ok_or("retained inputs missing")?;
            assert_eq!(retained.inputs(), expected_mapping.as_slice());
            assert_eq!(retained.input_digest(), expected_digest);
            let events = journal.events_for_run(run)?;
            let event = events
                .iter()
                .find(|event| {
                    matches!(
                        event,
                        vb_storage::JournalEvent::RunAdmission { run: event_run, .. }
                            | vb_storage::JournalEvent::RunAdmissionBudget { run: event_run, .. }
                            if *event_run == run
                    )
                })
                .ok_or("stored admission missing")?;
            assert!(matches!(
                event,
                vb_storage::JournalEvent::RunAdmission { input_digest, .. }
                    | vb_storage::JournalEvent::RunAdmissionBudget { input_digest, .. }
                    if *input_digest == vb_storage::RunInputBinding::Digest(expected_digest)
            ));
            retained.validate_admission(event)?;
        }
        Ok(())
    }

    #[test]
    fn scalar_default_input_binding_matches_volatile_frame_and_admission() -> TestResult {
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut shard = Shard::new_with_journal(one_step_config(), journal.clone());
        let run = RunId::ZERO;
        let workflow = input_workflow(Some(SlotValue::I64(7)))?;
        let digest = workflow.digest();
        shard.handle_submit(run, workflow, CapabilitySet::empty())?;
        assert_frame_input(&shard, run, Some(SlotValue::I64(7)))?;
        let expected_mapping = [(SlotIdx::ZERO, SlotValue::I64(7))];
        let expected_digest =
            vb_storage::inputs::compute_input_digest(expected_mapping.as_slice())?;
        let state = shard.run_state_get(run).ok_or("volatile run missing")?;
        let admission = state
            .admission
            .as_ref()
            .ok_or("volatile admission missing")?;
        assert_eq!(
            admission.input_digest(),
            vb_storage::RunInputBinding::Digest(expected_digest)
        );
        let frame_slot = state.frame.read_slot(SlotIdx::ZERO)?;
        assert_eq!(*frame_slot, SlotValue::I64(7));
        let events = journal.snapshot()?;
        let stored_admission = match events.as_slice() {
            [
                RuntimeJournalEvent::RunSubmitted {
                    run: e_run,
                    workflow: e_workflow,
                },
                RuntimeJournalEvent::RunAdmission { admission: stored },
                RuntimeJournalEvent::RunBudgetOrigin {
                    run: e_run2,
                    started_millis,
                },
                RuntimeJournalEvent::StepStarted {
                    run: e_run3,
                    step: e_step,
                    attempt: e_attempt,
                },
                RuntimeJournalEvent::StepSucceeded {
                    run: e_run4,
                    step: e_step2,
                    output,
                    attempt: e_attempt2,
                },
            ] if *e_run == run
                && *e_workflow == digest
                && *e_run2 == run
                && Some(*started_millis) == state.run_started_millis
                && *e_run3 == run
                && *e_step == StepIdx::ZERO
                && *e_attempt == 1
                && *e_run4 == run
                && *e_step2 == StepIdx::ZERO
                && *output == SlotIdx::ZERO
                && *e_attempt2 == 1 =>
            {
                assert_eq!(stored.run_id(), run);
                assert_eq!(
                    stored, admission,
                    "stored admission must equal in-memory admission exactly"
                );
                stored
            }
            other => {
                return Err(format!(
                    "expected exactly five ordered events including step execution, got {:?}",
                    other
                )
                .into());
            }
        };
        assert_eq!(stored_admission.artifact_digest(), digest);
        let observed_bytes: u64 = events.iter().try_fold(0_u64, |acc, event| {
            let encoded = postcard::to_allocvec(event).map_err(|_| "postcard encode failed")?;
            let event_bytes =
                u64::try_from(encoded.len()).map_err(|_| "encoded len out of range")?;
            acc.checked_add(event_bytes).ok_or("byte sum overflow")
        })?;
        assert_eq!(
            state.journal_bytes_written, observed_bytes,
            "journal_bytes_written must equal exact serialized complete header accounting"
        );
        Ok(())
    }

    // Test-local journal wrapper: unknown durability profile (returns None).
    struct UnknownProfileJournal(SharedRuntimeJournal);

    impl RuntimeJournal for UnknownProfileJournal {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            None
        }
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.0.append(event)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.0.append_sequenced(event, seq)
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.0.append_sequenced_batch(events, start_seq)
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            self.0.probe()
        }
    }

    // Test-local journal wrapper: explicitly Volatile but exposes a real FjallJournal.
    struct VolatileWithStorageJournal {
        inner: SharedRuntimeJournal,
        storage: Arc<vb_storage::FjallJournal>,
    }

    impl RuntimeJournal for VolatileWithStorageJournal {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }
        fn storage_journal(&self) -> Option<Arc<vb_storage::FjallJournal>> {
            Some(self.storage.clone())
        }
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.inner.append(event)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.inner.append_sequenced(event, seq)
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.inner.append_sequenced_batch(events, start_seq)
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            self.inner.probe()
        }
    }

    #[test]
    fn run_id_zero_rejected_for_unknown_durability_profile() -> TestResult {
        let inner = Arc::new(VolatileRuntimeJournal::new());
        let journal = UnknownProfileJournal(inner.clone());
        let shared: SharedRuntimeJournal = Arc::new(journal);
        let config = ShardConfig {
            step_budget_per_tick: 1,
            ..super::small_config()
        };
        let mut shard = Shard::new_with_journal(config, shared);
        let workflow = input_workflow(Some(SlotValue::I64(7)))?;
        let result = shard.handle_submit(RunId::ZERO, workflow, CapabilitySet::empty());
        let is_expected = match &result {
            Err(crate::RuntimeError::StorageJournalAppend { source }) => match source.as_ref() {
                vb_storage::JournalError::InvalidRunId { run } => *run == RunId::ZERO,
                _ => false,
            },
            _ => false,
        };
        assert!(
            is_expected,
            "expected StorageJournalAppend wrapping InvalidRunId ZERO, got {:?}",
            result
        );
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.run_state_get(RunId::ZERO), None);
        assert_eq!(
            inner.snapshot()?.len(),
            0,
            "no journal events after rejection"
        );
        assert_eq!(
            shard.counters().snapshot().runs_submitted,
            0,
            "no submitted counter after rejection"
        );
        assert_eq!(
            shard.global_cost_usage(),
            0,
            "no cost usage after rejection"
        );
        Ok(())
    }

    #[test]
    fn run_id_zero_rejected_for_volatile_with_storage() -> TestResult {
        let directory = tempfile::tempdir()?;
        let storage = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
        let inner = Arc::new(VolatileRuntimeJournal::new());
        let journal = VolatileWithStorageJournal {
            inner: inner.clone(),
            storage: storage.clone(),
        };
        let shared: SharedRuntimeJournal = Arc::new(journal);
        let config = ShardConfig {
            step_budget_per_tick: 1,
            ..super::small_config()
        };
        let mut shard = Shard::new_with_journal(config, shared);
        let workflow = input_workflow(Some(SlotValue::I64(7)))?;
        let result = shard.handle_submit(RunId::ZERO, workflow, CapabilitySet::empty());
        let is_expected = match &result {
            Err(crate::RuntimeError::StorageJournalAppend { source }) => match source.as_ref() {
                vb_storage::JournalError::InvalidRunId { run } => *run == RunId::ZERO,
                _ => false,
            },
            _ => false,
        };
        assert!(
            is_expected,
            "expected StorageJournalAppend wrapping InvalidRunId ZERO, got {:?}",
            result
        );
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.run_state_get(RunId::ZERO), None);
        assert_eq!(
            inner.snapshot()?.len(),
            0,
            "no journal events after rejection"
        );
        assert_eq!(
            shard.counters().snapshot().runs_submitted,
            0,
            "no submitted counter after rejection"
        );
        assert_eq!(
            shard.global_cost_usage(),
            0,
            "no cost usage after rejection"
        );
        let run_inputs_result = storage.run_inputs(RunId::ZERO);
        let is_invalid_run_id = match run_inputs_result {
            Err(vb_storage::JournalError::InvalidRunId { run }) => run == RunId::ZERO,
            _ => false,
        };
        assert!(
            is_invalid_run_id,
            "storage.run_inputs(ZERO) must return InvalidRunId for ZERO, not Ok(None)"
        );
        Ok(())
    }
}
