mod recover_phases {
    use super::*;
    use crate::recovery::{RecoveredAdmissionContext, RecoveredRunBoundary};
    use crate::shard::types::{RecoverRunCommand, RecoveredAdmissionKind, RegisteredExecutionMode};
    use vb_core::WorkflowDigest;
    use vb_storage::AcceptedArtifact;

    include!("recover_state.rs");

    struct VerifiedWorkflow {
        artifact: AcceptedArtifact,
        workflow: CompiledWorkflow,
    }

    impl VerifiedWorkflow {
        fn load(shard: &Shard, command: &RecoverRunCommand) -> RuntimeResult<Self> {
            let artifact = shard.load_recovery_artifact(command.artifact_digest)?;
            let workflow = crate::admission::verify_stored_artifact_detailed(
                shard.artifact_store.as_ref(),
                &artifact,
                Some(shard.policy),
            )
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })?;
            let expected = command.workflow_digest;
            if workflow.digest() != expected {
                return Err(RuntimeError::Recovery {
                    error: format!(
                        "workflow digest mismatch: expected {expected:?}, got {:?}",
                        workflow.digest()
                    ),
                });
            }
            Ok(Self { artifact, workflow })
        }

        fn validate_authority(
            &self,
            shard: &Shard,
            command: &RecoverRunCommand,
        ) -> RuntimeResult<Option<u64>> {
            let context = || RecoveredAdmissionContext {
                run: command.run,
                expected_workflow: command.workflow_digest,
                runtime_policy: shard.policy,
            };
            let historical = self.validate_admission(&command.admission, context())?;
            let checkpoint = command.retry_checkpoint.as_ref();
            recovered_admission::validate_retry_checkpoint(
                checkpoint,
                context(),
                &self.artifact,
                &self.workflow,
            )?;
            let origin = command
                .run_started_millis
                .or_else(|| checkpoint.and_then(|value| value.run_started_millis));
            crate::recovery::validate_recovered_lifetime_at_wall_clock(Some(historical), origin)
        }

        fn validate_admission<'a>(
            &self,
            admission: &'a RecoveredAdmissionKind,
            context: RecoveredAdmissionContext,
        ) -> RuntimeResult<&'a crate::admission::RunAdmission> {
            recovered_admission::validate_recovered_admission(
                admission,
                context,
                &self.artifact,
                &self.workflow,
            )
        }

        fn validate_secrets(
            &self,
            shard: &Shard,
            command: &RecoverRunCommand,
        ) -> RuntimeResult<()> {
            shard.validate_secret_presence(&self.workflow)?;
            if command.retry_checkpoint.is_none() {
                vb_core::workflow::source_metadata::secrets::validate_runtime_roots(
                    &self.workflow,
                    &command.frame.initialized_slots()?,
                )?;
            }
            Ok(())
        }
    }

    impl Shard {
        pub(crate) fn handle_recover(&mut self, command: RecoverRunCommand) -> RuntimeResult<()> {
            // Reject existing live ownership before recovery can replace its cursor.
            // Terminal-only identity remains eligible for the existing recovery path.
            if self.run_state_contains(command.run)
                || self.checked_out_run_contains(command.run)
                || self.runtime_state_get(command.run).is_some()
            {
                return Err(RuntimeError::RunAlreadyExists);
            }
            self.restore_and_resume_unowned_recovery(command)
        }

        // Called only after handle_recover rejects existing live ownership.
        fn restore_and_resume_unowned_recovery(
            &mut self,
            mut command: RecoverRunCommand,
        ) -> RuntimeResult<()> {
            let verified = VerifiedWorkflow::load(self, &command)?;
            let origin = verified.validate_authority(self, &command)?;
            verified.validate_secrets(self, &command)?;
            let timer =
                self.recovered_retry_timer(command.boundary, command.retry_checkpoint.as_ref())?;
            let store = recovery_store::restore(
                command.restored_store.take(),
                &command.frame,
                &verified.workflow,
                command.retry_checkpoint.is_some(),
            )?;
            let next_seq = command.next_seq;
            let mut continuation = Continuation::take(&mut command, timer);
            let tenant = command.recovered_tenant;
            let state = verified.restore(self, command, origin, store)?;
            continuation.prepare_timer(&state)?;
            if let Some(timer) = continuation.timer {
                self.prepare_pending_timer(continuation.run, timer)?;
            }
            self.seed_journal_sequence(continuation.run, next_seq)?;
            self.admit_recovered_state(continuation.run, state, tenant)?;
            continuation.resume(self)
        }

        fn admit_recovered_state(
            &mut self,
            run: RunId,
            mut state: RunState,
            tenant: Option<vb_core::ids::TenantId>,
        ) -> RuntimeResult<()> {
            let rollback_frame = state.frame.clone();
            if let Err(error) = self.restore_recovered_quota(run, &mut state, tenant) {
                self.release_frame(state.frame);
                return Err(error);
            }
            self.terminal_runs_remove(run);
            if let Err(error) = self.admit_run_state(run, state, RuntimeState::Initial) {
                self.release_admission_quota(run)?;
                self.release_frame(rollback_frame);
                return Err(error);
            }
            Ok(())
        }
    }

    struct Continuation {
        run: RunId,
        boundary: RecoveredRunBoundary,
        timer: Option<PendingTimer>,
        open_gates: Vec<crate::shard::types::OpenGate>,
        bound_open_gates: Vec<crate::shard::types::BoundOpenGate>,
    }

    impl Continuation {
        fn take(command: &mut RecoverRunCommand, timer: Option<PendingTimer>) -> Self {
            Self {
                run: command.run,
                boundary: command.boundary,
                timer,
                open_gates: core::mem::take(&mut command.open_gates),
                bound_open_gates: core::mem::take(&mut command.bound_open_gates),
            }
        }

        fn resume(self, shard: &mut Shard) -> RuntimeResult<()> {
            if shard.restore_recovered_gates(
                self.run,
                &self.open_gates,
                &self.bound_open_gates,
                self.boundary.kind(),
            )? {
                return Ok(());
            }
            if shard.restore_suspended_boundary(self.run, self.boundary, self.timer)? {
                return Ok(());
            }
            shard.drive_recovered_run(self.run)
        }

        fn prepare_timer(&mut self, state: &RunState) -> RuntimeResult<()> {
            if self.timer.is_none() {
                self.timer = recovered_boundary_timer(self.boundary, state)?;
            }
            Ok(())
        }
    }

    fn recovered_boundary_timer(
        boundary: RecoveredRunBoundary,
        state: &RunState,
    ) -> RuntimeResult<Option<PendingTimer>> {
        let Some((suspension, kind)) = recovered_timer_boundary(boundary) else {
            return Ok(None);
        };
        let step = suspension.step();
        let generation = suspension
            .boundary_seq()
            .get()
            .checked_add(1)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let deadline = awaiting_deadline(state, step)?;
        Ok(Some(PendingTimer {
            step,
            kind,
            generation,
            deadline,
        }))
    }

    fn recovered_timer_boundary(
        boundary: RecoveredRunBoundary,
    ) -> Option<(crate::recovery::RecoveredTimerSuspension, PendingTimerKind)> {
        boundary
            .waiting_timer()
            .map(|timer| (timer, PendingTimerKind::Wait))
            .or_else(|| {
                boundary
                    .timed_ask()
                    .map(|timer| (timer, PendingTimerKind::Ask))
            })
    }
}
