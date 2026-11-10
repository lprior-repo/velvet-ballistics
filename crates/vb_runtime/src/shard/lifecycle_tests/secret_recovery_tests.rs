mod secret_authority {
    use crate::{
        RuntimeError,
        admission::{AcceptedArtifactStore, ArtifactEnvelopeError},
        journal::VolatileRuntimeJournal,
        primitives::collect::CollectStates,
        recovery::RecoveredRunBoundary,
        shard::{Shard, ShardCommand, ShardConfig, types::RecoverRunCommand},
    };
    use std::sync::Arc;
    use vb_core::workflow::{
        node,
        source_metadata::{CompiledSecretRequirement, WorkflowSourceMetadata},
    };
    use vb_core::{
        CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind as K, CompiledWorkflow,
        ConstIdx, ConstValue, InputDefaultValue, InputSlotKind, ResourceContract, RunFrame, RunId,
        SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest, WorkflowParts,
    };

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn workflow() -> TestResult<CompiledWorkflow> {
        let mut parts = WorkflowParts {
            name: "raw-secret-recovery".into(),
            digest: WorkflowDigest::from_bytes([31; 32]),
            semantic_source_identity: None,
            source_metadata: Some(Box::new(WorkflowSourceMetadata::try_new(
                Box::new(["token".into(), "TOKEN".into()]),
                Box::new([CompiledSecretRequirement::with_slot(
                    vb_core::SymbolId::new(0),
                    vb_core::SymbolId::new(1),
                    SlotIdx::ZERO,
                )]),
            )?)),
            nodes: Box::new([
                node(
                    StepIdx::ZERO,
                    K::SetConst {
                        value: ConstIdx::new(0),
                    },
                    Some(SlotIdx::new(1)),
                    Some(StepIdx::new(1)),
                ),
                node(
                    StepIdx::new(1),
                    K::Finish {
                        result: SlotIdx::new(1),
                    },
                    None,
                    None,
                ),
            ]),
            expressions: Box::default(),
            accessors: Box::default(),
            constants: Box::new([ConstValue::I64(42)]),
            slot_count: 2,
            symbols_count: 2,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new(["seed".into(), "done".into()]),
            input_slots: Box::new([CompiledInputSlot {
                slot: SlotIdx::ZERO,
                kind: InputSlotKind::Symbol,
                constraint: CompiledInputConstraint {
                    secret: true,
                    default_cold: Some(InputDefaultValue::Text("TOKEN".into())),
                    ..CompiledInputConstraint::required()
                },
            }]),
        };
        parts.digest = WorkflowDigest::from_bytes([0; 32]);
        parts.digest =
            WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
        Ok(CompiledWorkflow::try_from_parts(parts)?)
    }

    #[derive(Debug)]
    struct Store(vb_storage::admission::AcceptedArtifact);
    impl AcceptedArtifactStore for Store {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            if digest != self.0.digest {
                return Err(ArtifactEnvelopeError::ArtifactNotFound { digest });
            }
            Ok(self.0.clone())
        }
    }

    #[test]
    fn raw_recovery_requires_existing_immutable_secret_root() -> TestResult {
        let workflow = workflow()?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            vb_core::RuntimePolicy::Relaxed,
            &[],
        )?;
        let digest = artifact.digest;
        let store = Arc::new(Store(artifact));
        let config = ShardConfig {
            policy: vb_core::RuntimePolicy::Relaxed,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        };
        let run = RunId::new(31);
        let source_journal = Arc::new(VolatileRuntimeJournal::new());
        let mut source = Shard::new_with_journal_and_artifact_store(
            config,
            source_journal.clone(),
            store.clone(),
        );
        source.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
        source.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::CapabilitySet::empty(),
        })?;
        source.tick()?;
        let frame = source
            .run_state_get(run)
            .ok_or("live source")?
            .frame
            .checkpoint_v1();
        assert_eq!(frame.pc, StepIdx::new(1));
        let next_seq = vb_storage::EventSeq::new(u64::try_from(source_journal.snapshot()?.len())?);
        for change in 0..4 {
            let mut frame = frame.clone();
            match change {
                0 => {}
                1 => {
                    *frame.slots.get_mut(0).ok_or("slot")? = None;
                    *frame.taint.get_mut(0).ok_or("taint")? = Taint::Clean;
                }
                2 => *frame.taint.get_mut(0).ok_or("taint")? = Taint::Clean,
                _ => {
                    *frame.slots.get_mut(0).ok_or("slot")? =
                        Some(SlotValue::Symbol(vb_core::SymbolId::new(0)))
                }
            }
            let journal = Arc::new(VolatileRuntimeJournal::new());
            let mut target =
                Shard::new_with_journal_and_artifact_store(config, journal.clone(), store.clone());
            target.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
            target.enqueue(ShardCommand::Recover {
                command: Box::new(RecoverRunCommand::new(
                    run,
                    RunFrame::from_checkpoint_v1(frame).map_err(|error| format!("{error:?}"))?,
                    digest,
                    workflow.digest(),
                    next_seq,
                    CollectStates::new(),
                    RecoveredRunBoundary::none(),
                )),
            })?;
            if change == 0 {
                assert_eq!(target.tick(), Ok(true));
                assert_eq!(
                    journal
                        .snapshot()?
                        .iter()
                        .filter(|event| matches!(
                            event,
                            crate::journal::RuntimeJournalEvent::RunFinished { .. }
                        ))
                        .count(),
                    1
                );
            } else {
                assert_eq!(
                    target.tick(),
                    Err(RuntimeError::Core {
                        source: Box::new(vb_core::CoreError::InternalInvariantViolation {
                            reason: "secret_input_binding"
                        })
                    })
                );
                assert_eq!(target.active_run_count(), 0);
                assert_eq!(journal.snapshot()?, Vec::new());
            }
        }
        Ok(())
    }

    struct CheckpointFixture {
        journal: Arc<vb_storage::FjallJournal>,
        workflow: CompiledWorkflow,
        checkpoint: crate::run_checkpoint::RunCheckpointV1,
        config: ShardConfig,
        next_seq: vb_storage::EventSeq,
        _directory: tempfile::TempDir,
    }

    fn checkpoint_fixture() -> TestResult<CheckpointFixture> {
        let mut parts = workflow()?.to_parts();
        let finish = parts.nodes.get_mut(1).ok_or("finish")?;
        finish.kind = K::Nop;
        finish.routing.next = Some(StepIdx::new(2)).into();
        let mut nodes = parts.nodes.into_vec();
        nodes.push(node(
            StepIdx::new(2),
            K::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        ));
        parts.nodes = nodes.into_boxed_slice();
        parts.step_names = Box::new(["seed".into(), "resume".into(), "done".into()]);
        parts.digest = WorkflowDigest::from_bytes([0; 32]);
        parts.digest =
            WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
        let workflow = CompiledWorkflow::try_from_parts(parts)?;
        let directory = tempfile::tempdir()?;
        let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
        let artifact = vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            vb_core::RuntimePolicy::Strict,
            &[],
        )?;
        assert_eq!(
            vb_storage::admission::verify_accepted_artifact_for_policy(
                &artifact,
                vb_core::RuntimePolicy::Strict
            )?
            .to_parts(),
            workflow.to_parts()
        );
        let config = ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        };
        let run = RunId::new(32);
        let mut source = Shard::new_with_journal(
            config,
            crate::journal::StorageRuntimeJournal::shared_strict(journal.clone()),
        );
        source.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
        source.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::CapabilitySet::empty(),
        })?;
        assert_eq!(source.tick(), Ok(true));
        let next_seq = *source
            .journal_sequences
            .get(&run)
            .ok_or("source sequence")?;
        let mut failed = source.run_state_get(run).ok_or("source state")?.clone();
        assert_eq!(
            failed
                .admission
                .as_ref()
                .ok_or("admission")?
                .artifact_digest(),
            artifact.digest
        );
        assert_eq!(failed.frame.pc(), StepIdx::new(1));
        // Capture uses the real admitted state and checked frame transitions.
        failed.frame.mark_running(StepIdx::new(1))?;
        failed.frame.mark_failed(StepIdx::new(1))?;
        let retired = failed.store.insert_symbol("retained checkpoint payload")?;
        failed.store.insert_list_with_taint(
            Box::new([SlotValue::Symbol(retired)]),
            Box::new([Taint::Secret]),
        )?;
        let checkpoint = crate::run_checkpoint::RunCheckpointV1::capture(
            run,
            artifact.digest,
            core::num::NonZeroU16::new(2).ok_or("attempt")?,
            StepIdx::new(1),
            next_seq.get().checked_sub(1).ok_or("prior sequence")?,
            &failed,
        )?;
        Ok(CheckpointFixture {
            _directory: directory,
            journal,
            workflow,
            checkpoint,
            config,
            next_seq,
        })
    }

    #[test]
    fn full_checkpoint_restores_exact_secret_roots_and_arenas_instead_of_loose_frame() -> TestResult
    {
        let fixture = checkpoint_fixture()?;
        let checkpoint = &fixture.checkpoint;
        let (restored, _, _, _) = checkpoint.clone().restore(
            fixture.workflow.clone(),
            checkpoint.artifact_digest,
            &[],
        )?;
        assert_eq!(restored.frame.checkpoint_v1(), checkpoint.frame);
        assert_eq!(restored.store.checkpoint_v1(), checkpoint.value_store);

        let mut command = RecoverRunCommand::for_retry(checkpoint.clone(), fixture.next_seq)?;
        let mut loose = command.frame.checkpoint_v1();
        *loose.slots.first_mut().ok_or("loose slot")? = None;
        *loose.taint.first_mut().ok_or("loose taint")? = Taint::Clean;
        command.frame =
            RunFrame::from_checkpoint_v1(loose).map_err(|error| format!("{error:?}"))?;
        let mut target = Shard::new_with_journal(
            fixture.config,
            crate::journal::StorageRuntimeJournal::shared_strict(fixture.journal.clone()),
        );
        target.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
        target.enqueue(ShardCommand::Recover {
            command: Box::new(command),
        })?;
        assert_eq!(target.tick(), Ok(true));
        let recovered = target
            .run_state_get(checkpoint.run_id)
            .ok_or("recovered state")?;
        assert_eq!(recovered.frame.pc(), StepIdx::new(2));
        let frame = recovered.frame.checkpoint_v1();
        assert_eq!(frame.slots, checkpoint.frame.slots);
        assert_eq!(frame.taint, checkpoint.frame.taint);
        assert_eq!(recovered.store.checkpoint_v1(), checkpoint.value_store);
        assert_eq!(
            recovered.frame.read_slot(SlotIdx::ZERO)?,
            &SlotValue::Symbol(vb_core::SymbolId::new(1))
        );
        assert_eq!(recovered.frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);
        assert_eq!(recovered.run_attempt.get(), 2);
        assert_eq!(target.tick(), Ok(true));
        assert_eq!(
            fixture
                .journal
                .events_for_run_full(checkpoint.run_id)?
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn full_checkpoint_missing_wrong_taint_or_wrong_handle_rejects_without_repair() -> TestResult {
        let fixture = checkpoint_fixture()?;
        let checkpoint = &fixture.checkpoint;
        let before = fixture.journal.events_for_run_full(checkpoint.run_id)?;
        for change in 0..3 {
            let mut tampered = checkpoint.clone();
            match change {
                0 => {
                    *tampered.frame.slots.first_mut().ok_or("slot")? = None;
                    *tampered.frame.taint.first_mut().ok_or("taint")? = Taint::Clean;
                }
                1 => *tampered.frame.taint.first_mut().ok_or("taint")? = Taint::Clean,
                _ => {
                    *tampered.frame.slots.first_mut().ok_or("slot")? =
                        Some(SlotValue::Symbol(vb_core::SymbolId::new(0)))
                }
            }
            assert_eq!(
                tampered.validate_against(&fixture.workflow, checkpoint.artifact_digest, &[]),
                Err(crate::run_checkpoint::RunCheckpointV1Error::InvalidNestedState)
            );
            // The loose frame remains valid; the attached full checkpoint is authoritative.
            let command = RecoverRunCommand::for_retry(checkpoint.clone(), fixture.next_seq)?
                .with_retry_checkpoint(tampered);
            let mut target = Shard::new_with_journal(
                fixture.config,
                crate::journal::StorageRuntimeJournal::shared_strict(fixture.journal.clone()),
            );
            target.set_available_secret_names(Arc::from([Box::<str>::from("TOKEN")]));
            let pending = target.pending_boundary_snapshot(0, 8);
            target.enqueue(ShardCommand::Recover {
                command: Box::new(command),
            })?;
            assert_eq!(
                target.tick(),
                Err(RuntimeError::Recovery {
                    error:
                        "retry checkpoint validation failed: nested checkpoint validation failed"
                            .into()
                })
            );
            assert_eq!(target.active_run_count(), 0);
            assert_eq!(target.journal_sequences.get(&checkpoint.run_id), None);
            assert_eq!(target.pending_boundary_snapshot(0, 8), pending);
            assert_eq!(
                fixture.journal.events_for_run_full(checkpoint.run_id)?,
                before
            );
        }
        Ok(())
    }
}
