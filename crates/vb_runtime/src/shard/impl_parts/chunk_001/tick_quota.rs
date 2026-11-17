impl Shard {
    fn finish_queued_quota(
        &mut self,
        submit_run: Option<RunId>,
        result: RuntimeResult<bool>,
    ) -> RuntimeResult<bool> {
        let pending = self.pending_quota_reservation.take();
        match result {
            Ok(alive) if pending.is_none() => Ok(alive),
            Ok(_) => {
                let (_, reservation) =
                    pending.ok_or(RuntimeError::ResourceAccountingUnderflow {
                        resource: "pending_quota_reservation_missing",
                    })?;
                self.quota
                    .release(reservation.tenant, reservation.cost_units)?;
                Err(RuntimeError::ResourceAccountingUnderflow {
                    resource: "pending_quota_reservation_unclaimed",
                })
            }
            Err(error) => self.rollback_queued_quota(submit_run, pending, error),
        }
    }

    fn rollback_queued_quota(
        &mut self,
        submit_run: Option<RunId>,
        pending: Option<(RunId, crate::shard::types::PendingQuotaReservation)>,
        error: RuntimeError,
    ) -> RuntimeResult<bool> {
        let rollback = self.release_queued_quota_reservation(submit_run, pending);
        match rollback {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(RuntimeError::rollback_failed(
                "queued_submission_quota",
                error,
                rollback_error,
            )),
        }
    }

    fn release_queued_quota_reservation(
        &mut self,
        submit_run: Option<RunId>,
        pending: Option<(RunId, crate::shard::types::PendingQuotaReservation)>,
    ) -> RuntimeResult<()> {
        match pending {
            Some((_, reservation)) => self
                .quota
                .release(reservation.tenant, reservation.cost_units),
            None => match submit_run {
                Some(run)
                    if !self.run_state_contains(run)
                        && !self.checked_out_run_contains(run)
                        && self.runtime_state_get(run).is_none() =>
                {
                    self.release_admission_quota(run)
                }
                _ => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod recover_duplicate_guard_tests {
    use super::*;
    use crate::shard::types::{PendingQuotaReservation, RecoverRunCommand};
    use crate::shard::{RuntimeState, ShardConfig};
    use vb_core::{RunFrame, StepIdx, WorkflowDigest};
    use vb_storage::EventSeq;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    struct MissingArtifact;
    impl crate::admission::AcceptedArtifactStore for MissingArtifact {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<vb_storage::AcceptedArtifact, crate::admission::ArtifactEnvelopeError> {
            Err(crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest })
        }
    }

    fn test_shard() -> TestResult<Shard> {
        Ok(Shard::new_with_journal_and_artifact_store(
            ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 64,
                step_budget_per_tick: 4,
                max_active_runs: 8,
                max_open_gates: 8,
                policy: vb_core::RuntimePolicy::Relaxed,
            },
            std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new()),
            std::sync::Arc::new(MissingArtifact),
        )?)
    }

    fn raw_command(run: RunId) -> TestResult<RecoverRunCommand> {
        Ok(RecoverRunCommand::new(
            run,
            RunFrame::new(run, StepIdx::ZERO, 1, 1)?,
            WorkflowDigest::from_bytes([0; 32]),
            WorkflowDigest::from_bytes([0; 32]),
            EventSeq::new(1),
            crate::primitives::collect::CollectStates::new(),
            crate::recovery::RecoveredRunBoundary::none(),
        ))
    }

    fn insert_live_state(shard: &mut Shard, run: RunId) -> TestResult {
        use vb_core::{CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract, SlotIdx, WorkflowParts};
        use vb_core::workflow::node;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: "duplicate-recovery-guard".into(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            semantic_source_identity: None,
            source_metadata: None,
            nodes: Box::new([
                node(StepIdx::ZERO, CompiledNodeKind::SetConst { value: ConstIdx::new(0) }, Some(SlotIdx::ZERO), Some(StepIdx::new(1))),
                node(StepIdx::new(1), CompiledNodeKind::Finish { result: SlotIdx::ZERO }, None, None),
            ]),
            expressions: Box::default(),
            accessors: Box::default(),
            constants: Box::new([ConstValue::I64(1)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new(["value".into(), "done".into()]),
            input_slots: Box::default(),
        })?;
        let state = crate::shard::RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
            frame: RunFrame::new(run, StepIdx::ZERO, 2, 1)?,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow,
            store: vb_core::value_store::ValueStore::new(),
            action_attempts: vec![0; 2].into_boxed_slice(),
            admission: None,
            run_started_millis: None,
            collect_states: crate::primitives::collect::CollectStates::new(),
            journal_bytes_written: 0,
            action_contracts: Box::default(),
        };
        shard.admit_run_state(run, state, RuntimeState::Initial)?;
        // Isolate the live-map guard instead of relying on runtime-state overlap.
        shard.runtime_state_terminal_clear(run);
        Ok(())
    }

    // Synthetic ownership tests, not a valid-artifact recovery reproduction.
    // No persistent Store, recovery replay, driver tick or process is invoked.
    #[test]
    fn duplicate_recovery_guard_and_queue_cleanup_preserve_owned_quota() -> TestResult {
        for ownership in [0, 1, 2] {
            for pending in [false, true] {
                let mut shard = test_shard()?;
                let run = RunId::new(71_901);
                if ownership == 0 {
                    insert_live_state(&mut shard, run)?;
                } else {
                    shard.checked_out_run_insert(run)?;
                    if ownership == 2 {
                        shard.runtime_state_insert(run, RuntimeState::Running)?;
                        shard.checked_out_run_remove(run);
                    }
                }
                let live = shard.run_state_get(run).cloned();
                let tenant = Some(vb_core::ids::TenantId::new(91));
                shard.charge_admission_quota(run, tenant, 7)?;
                shard.seed_journal_sequence(run, EventSeq::new(100))?;
                let charges = shard.charged_quotas.clone();
                let usage = shard.usage_snapshot();
                let states = shard.runtime_state_get(run);
                let checked_out = shard.checked_out_run_contains(run);
                if pending {
                    shard.quota.reserve(tenant, 3)?;
                    shard.pending_quota_reservation = Some((
                        run,
                        PendingQuotaReservation { tenant, cost_units: 3 },
                    ));
                }
                let result = shard.handle_recover(raw_command(run)?);
                assert_eq!(result, Err(RuntimeError::RunAlreadyExists));
                assert_eq!(shard.journal_seq_get(run), Some(EventSeq::new(100)));
                assert_eq!(shard.charged_quotas, charges);
                assert_eq!(shard.global_cost_usage(), if pending { 10 } else { 7 });
                assert_eq!(
                    shard.finish_queued_quota(Some(run), result.map(|()| true)),
                    Err(RuntimeError::RunAlreadyExists)
                );
                assert_eq!(shard.pending_quota_reservation, None);
                assert_eq!(shard.global_cost_usage(), 7);
                assert_eq!(shard.usage_snapshot(), usage);
                assert_eq!(shard.charged_quotas, charges);
                assert_eq!(shard.journal_seq_get(run), Some(EventSeq::new(100)));
                assert_eq!(shard.run_state_get(run), live.as_ref());
                assert_eq!(shard.runtime_state_get(run), states);
                assert_eq!(shard.checked_out_run_contains(run), checked_out);
            }
        }
        Ok(())
    }

    #[test]
    fn failed_unowned_submission_still_releases_its_claimed_quota() -> TestResult {
        let mut shard = test_shard()?;
        let run = RunId::new(71_902);
        shard.charge_admission_quota(run, None, 7)?;
        assert_eq!(shard.global_cost_usage(), 7);
        assert_eq!(
            shard.finish_queued_quota(Some(run), Err(RuntimeError::RunAlreadyExists)),
            Err(RuntimeError::RunAlreadyExists)
        );
        assert_eq!(shard.global_cost_usage(), 0);
        assert!(!shard.charged_quotas.contains_key(&run));
        Ok(())
    }

    #[test]
    fn terminal_only_recovery_still_reaches_artifact_validation() -> TestResult {
        let mut shard = test_shard()?;
        let run = RunId::new(71_903);
        shard.checked_out_run_insert(run)?;
        shard.terminal_runs_insert(run)?;
        assert!(shard.terminal_runs_contains(run));
        assert!(!shard.checked_out_run_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        assert_eq!(
            shard.handle_recover(raw_command(run)?),
            Err(RuntimeError::Recovery {
                error: format!("artifact not found during recovery: {:?}", WorkflowDigest::from_bytes([0; 32])),
            })
        );
        assert!(shard.terminal_runs_contains(run));
        Ok(())
    }
}
