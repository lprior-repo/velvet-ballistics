mod default_binding {
    use std::error::Error;
    use std::sync::Arc;

    use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind, node};
    use vb_core::{SlotValue, WorkflowDigest};

    use super::{CapabilitySet, CompiledNodeKind, RunId, Shard, ShardConfig, SlotIdx, StepIdx};
    use crate::journal::{RuntimeJournalEvent, StorageRuntimeJournal, VolatileRuntimeJournal};

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
        shard.handle_submit(
            run,
            input_workflow(Some(SlotValue::I64(7)))?,
            CapabilitySet::empty(),
        )?;
        assert_frame_input(&shard, run, Some(SlotValue::I64(7)))?;
        let expected_digest =
            vb_storage::inputs::compute_input_digest(&[(SlotIdx::ZERO, SlotValue::I64(7))])?;
        let state = shard.run_state_get(run).ok_or("volatile run missing")?;
        let admission = state
            .admission
            .as_ref()
            .ok_or("volatile admission missing")?;
        assert_eq!(
            admission.input_digest(),
            vb_storage::RunInputBinding::Digest(expected_digest)
        );
        let events = journal.snapshot()?;
        let stored = events
            .iter()
            .find_map(|event| match event {
                RuntimeJournalEvent::RunAdmission { admission } if admission.run_id() == run => {
                    Some(admission)
                }
                _ => None,
            })
            .ok_or("volatile admission event missing")?;
        assert_eq!(
            stored.input_digest(),
            vb_storage::RunInputBinding::Digest(expected_digest)
        );
        Ok(())
    }
}
