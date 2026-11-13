impl Shard {
    /// Recovers a run from a pre-hydrated `RunFrame` by loading the compiled workflow
    /// from the artifact store and constructing a full `RunState`.
    ///
    /// The workflow digest comes from the journal recovery summary and is
    /// used to look up the `AcceptedArtifact` whose `ir` field contains the
    /// postcard-encoded `WorkflowParts`. The parts are deserialized and
    /// validated via `CompiledWorkflow::try_from_parts`.
    ///
    /// Recovery differs from submit in three ways:
    /// - No `prepare_run_slots` / `take_frame_for` — the frame is pre-hydrated.
    /// - The artifact store is queried by digest rather than the workflow being
    ///   passed directly.
    /// - Admission is built with empty capabilities (caps are not persisted
    ///   per-run in the journal; runs requiring non-empty caps will fail
    ///   admission during recovery, which is the correct fail-closed behavior).
    ///
    /// Recovers a run from full recovery context provided by journal replay.
    ///
    /// The typed `RecoverRunCommand` carries all recovery evidence: artifact
    /// digest for verification, workflow digest for identity validation,
    /// journal sequence for sequencing, collect states for pagination
    /// resumption, and boundary classification for suspended operations.
    ///
    /// Recovery flow:
    /// 1. Load artifact by artifact_digest (verification)
    /// 2. Deserialize and compile workflow from artifact IR
    /// 3. Verify workflow identity matches command workflow_digest
    /// 4. Seed journal sequence from command next_seq
    /// 5. Build admission with artifact's required capabilities
    /// 6. Construct RunState restoring collect_states
    /// 7. Handle boundary (pending action / open ask / recovered timer)
    pub(crate) fn handle_recover(
        &mut self,
        command: crate::shard::types::RecoverRunCommand,
    ) -> RuntimeResult<()> {
        let run = command.run;
        let frame = command.frame;
        let artifact_digest = command.artifact_digest;
        let workflow_digest = command.workflow_digest;
        let next_seq = command.next_seq;
        let collect_states = command.collect_states;
        let retry_checkpoint = command.retry_checkpoint;
        let boundary = command.boundary;
        let admission = command.admission;
        let restored_store = command.restored_store;

        // Step 1: Load artifact by artifact_digest for verification
        let artifact = self
            .artifact_store
            .load_accepted_artifact(artifact_digest)
            .map_err(|e| match e {
                crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                    RuntimeError::Recovery {
                        error: format!("artifact not found during recovery: {digest:?}"),
                    }
                }
                other => RuntimeError::Recovery {
                    error: format!("artifact load failed: {other}"),
                },
            })?;

        // Step 2: Deserialize and compile workflow from artifact IR
        let parts: vb_core::workflow::WorkflowParts =
            postcard::from_bytes(&artifact.ir).map_err(|_| RuntimeError::Recovery {
                error: "artifact IR decode failed".to_string(),
            })?;

        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).map_err(|e| {
            RuntimeError::Recovery {
                error: format!("workflow compile failed: {e}"),
            }
        })?;

        // Step 3: Verify workflow identity matches command workflow_digest
        if workflow.digest() != workflow_digest {
            return Err(RuntimeError::Recovery {
                error: format!(
                    "workflow digest mismatch: expected {workflow_digest:?}, got {:?}",
                    workflow.digest()
                ),
            });
        }

        // Phase A: Validate the command's admission evidence.
        // Runs for all recovery paths (full, raw, retry) because the
        // raw command path bypasses the Runtime prefix validation.
        recovered_admission::validate_recovered_admission(
            &admission,
            crate::recovery::RecoveredAdmissionContext {
                run,
                expected_workflow: workflow_digest,
                runtime_policy: self.policy,
            },
            &artifact,
            &workflow,
        )?;

        // Phase B: Validate the full retry checkpoint before any state mutation.
        // Checks nested identities against the compiled workflow, and if the
        // checkpoint carries an admission, validates the full recovered
        // admission evidence. Applies to both Historical and Unverified
        // command paths because the public new(...).with_retry_checkpoint(...)
        // API can attach a checkpoint to an Unverified command without Runtime
        // prefix validation.
        recovered_admission::validate_retry_checkpoint(
            retry_checkpoint.as_ref(),
            crate::recovery::RecoveredAdmissionContext {
                run,
                expected_workflow: workflow_digest,
                runtime_policy: self.policy,
            },
            &artifact,
            &workflow,
        )?;

        // Phase C: Build unverified admission for legacy recovery.
        // For Unverified kind, build admission using the artifact's required
        // capabilities and current policy, then force the Unverified pin so
        // recovery never promotes to Pinned.
        let unverified_admission = if matches!(
            admission,
            crate::shard::types::RecoveredAdmissionKind::Unverified { .. }
        ) {
            recovered_admission::build_unverified_admission(self, run, artifact_digest, &artifact)?
        } else {
            None
        };

        self.validate_secret_presence(&workflow)?;
        if retry_checkpoint.is_none() {
            vb_core::workflow::source_metadata::secrets::validate_runtime_roots(
                &workflow,
                &frame.initialized_slots()?,
            )?;
        }
        let recovered_timer = self.recovered_retry_timer(boundary, retry_checkpoint.as_ref())?;
        let initial_store = recovery_store::restore(
            restored_store,
            &frame,
            &workflow,
            retry_checkpoint.is_some(),
        )?;

        // Step 5: Seed journal sequence from recovered next_seq
        self.seed_journal_sequence(run, next_seq)?;

        // Step 6: Restore the full validated checkpoint when this is a retry.
        // The checkpoint's accepted artifact digest and action registry are
        // checked against the independently loaded artifact before run state mutates.
        let registered_execution =
            if boundary.kind() == crate::recovery::RecoveredRunBoundaryKind::PendingAction {
                crate::shard::types::RegisteredExecutionMode::ReconcileOnly
            } else {
                crate::shard::types::RegisteredExecutionMode::Live
            };
        let state = if let Some(checkpoint) = retry_checkpoint {
            let (mut state, _attempt, _retry_step, _prior_seq) = checkpoint
                .restore(workflow, artifact_digest, &artifact.action_contracts)
                .map_err(|error| RuntimeError::Recovery {
                    error: format!("retry checkpoint validation failed: {error}"),
                })?;
            if state.frame.run_id() != run {
                return Err(RuntimeError::Recovery {
                    error: "retry checkpoint run identity mismatch".to_string(),
                });
            }
            state.registered_execution = registered_execution;
            self.restore_retry_boundary_frame(&mut state, boundary)?;
            self.prepare_run_slots(run)?;
            state
        } else {
            // Step 7: Restore the original historical admission validated by the
            // recovery prefix flow (validate_recovery_admission) rather than
            // rebuilding admission from current policy. Rebuilding could upgrade
            // a legacy Unverified admission to Pinned, authorizing external
            // effects the original run never had authority for.
            let frame_step_count = frame.step_count();
            let max_slots = workflow.resource_contract().max_slots;
            let store = match initial_store {
                Some(store) => store,
                None => {
                    let mut store = vb_core::value_store::ValueStore::with_max_slots(max_slots);
                    if let Some(metadata) = workflow.source_metadata() {
                        vb_core::workflow::source_metadata::seed_source_symbols(
                            metadata, &mut store,
                        )?;
                    }
                    store
                }
            };
            self.prepare_run_slots(run)?;
            let state_admission = match admission {
                crate::shard::types::RecoveredAdmissionKind::Historical(h) => Some(*h),
                crate::shard::types::RecoveredAdmissionKind::Unverified { .. } => {
                    unverified_admission
                }
            };
            crate::shard::types::RunState {
                registered_execution,
                frame,
                run_attempt: core::num::NonZeroU16::MIN,
                workflow,
                store,
                action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
                admission: state_admission,
                run_started_millis: None,
                collect_states,
                journal_bytes_written: 0,
                action_contracts: artifact.action_contracts.clone(),
            }
        };

        self.terminal_runs_remove(run);
        self.admit_run_state(run, state, crate::shard::types::RuntimeState::Initial)?;

        // Step 7a: Restore reconstructed open gates before any boundary
        // handling. A gated run is suspended on its gate, never on another
        // boundary: a recovered gate together with a pending-action or
        // open-ask boundary is inconsistent journal evidence, so recovery
        // fails instead of driving a gated run or dropping the gate. The
        // run stays suspended (AwaitAction) and is not driven; the gate
        // decision resumes or fails it through the normal submission flow.
        if !command.open_gates.is_empty() {
            if boundary.kind() != crate::recovery::RecoveredRunBoundaryKind::None {
                return Err(RuntimeError::Recovery {
                    error: "recovered run has both an open gate and a suspended boundary"
                        .to_string(),
                });
            }
            self.restore_open_gates(run, &command.open_gates)?;
            self.apply(run, crate::shard::types::RuntimeEvent::AwaitAction)?;
            return Ok(());
        }

        // Step 7: Handle boundary for suspended operations
        match boundary.kind() {
            crate::recovery::RecoveredRunBoundaryKind::RetryTimer => {
                let timer = recovered_timer.ok_or(RuntimeError::InvalidTimerFire)?;
                self.pending_timer_insert(run, timer)?;
                self.apply(run, RuntimeEvent::AwaitTimer)?;
                return Ok(());
            }
            crate::recovery::RecoveredRunBoundaryKind::None => {}
            crate::recovery::RecoveredRunBoundaryKind::PendingAction => {
                // Re-inject the durable pending-action ticket and leave the run
                // suspended at the pending action. Recovery MUST NOT re-drive the
                // Do node: the action is already durably scheduled
                // (ActionScheduledTicket) and its input slot may not be re-readable
                // (legacy unbound admission does not retain inputs). Re-driving would
                // emit a duplicate StepStarted and, for legacy runs, fail closed on
                // the un-rehydrated input slot. The recovered run resumes only on the
                // external action completion/failure signal.
                if let Some(t) = boundary.pending_action_ticket().map(|t| t.ticket()) {
                    // Restore the per-step attempt counter so a later completion or
                    // failure can validate the recovered ticket's attempt
                    // (validate_ticket_attempt requires action_attempts[step] to
                    // equal the scheduled ticket's attempt).
                    if let Some(state) = self.run_state_get_mut(run) {
                        crate::shard::helpers::record_scheduled_attempt(state, t);
                    }
                    self.reinject_pending_action(run, t)?;
                    self.apply(run, crate::shard::types::RuntimeEvent::AwaitAction)?;
                    return Ok(());
                }
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenAsk => {
                if let Some(ask) = boundary.open_ask() {
                    self.record_pending_ask_authority(run, ask.step(), ask.boundary_seq());
                    self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
                    return Ok(());
                }
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenWaitEvent => {
                if self.restore_open_wait_event(run, boundary)? {
                    return Ok(());
                }
            }
            crate::recovery::RecoveredRunBoundaryKind::WaitingTimer => {
                // Re-arms the durable wait schedule as a live timer and leaves
                // the run suspended. Recovery MUST NOT drive the wait node:
                // re-driving would emit a duplicate WaitScheduledEvent.
                if let Some(timer) = boundary.waiting_timer() {
                    self.restore_recovered_timer_boundary(run, timer, PendingTimerKind::Wait)?;
                    self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
                    return Ok(());
                }
            }
            crate::recovery::RecoveredRunBoundaryKind::TimedAsk => {
                // Restores both the ask authority and the timer the timeout
                // slot armed. The timer is required: without it the pending
                // ask ticket lookup refuses (ask_timer_matches) and the
                // answer path cannot validate the ticket.
                if let Some(ask) = boundary.timed_ask() {
                    self.restore_recovered_timer_boundary(run, ask, PendingTimerKind::Ask)?;
                    self.record_pending_ask_authority(run, ask.step(), ask.boundary_seq());
                    self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
                    return Ok(());
                }
            }
        }

        self.drive_recovered_run(run)
    }
}
