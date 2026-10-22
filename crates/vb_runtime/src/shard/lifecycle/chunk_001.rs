impl Shard {
    pub(crate) fn handle_submit(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            &[],
            true,
        )
    }

    pub(crate) fn handle_submit_pre_persisted(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            &[],
            false,
        )
    }

    pub(crate) fn handle_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            inputs,
            caps,
            &[],
            true,
        )
    }

    pub(crate) fn handle_submit_with_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        self.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            action_contracts,
        )?;
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            action_contracts,
            true,
        )
    }

    pub(crate) fn handle_submit_with_inputs_and_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        self.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            action_contracts,
        )?;
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
            true,
        )
    }

    fn handle_submit_with_inputs_contracts_and_header_mode(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
        persist_header: bool,
    ) -> RuntimeResult<()> {
        if self.run_state_contains(run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if self.active_run_count() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        let digest = workflow.digest();
        let admission = self.build_admission(run, digest, caps)?;
        execution_binding::validate(
            self.artifact_store.as_ref(),
            self.policy,
            &workflow,
            action_contracts,
        )?;
        self.prepare_run_slots(run)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        crate::shard::helpers::seed_input_slots(&mut frame, inputs, &workflow)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        if persist_header {
            self.append_admission_header_journal_event(
                run,
                RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: digest,
                },
            )?;
        }
        if let Some(admission) = admission.as_ref() {
            self.append_admission_header_journal_event(
                run,
                RuntimeJournalEvent::RunAdmission {
                    admission: admission.clone(),
                },
            )?;
        }
        self.counters.inc_submitted();
        let frame_step_count = frame.step_count();
        let max_slots = workflow.resource_contract().max_slots;
        let state = RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
            frame,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow,
            store: ValueStore::with_max_slots(max_slots),
            action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
            admission,
            collect_states: CollectStates::new(),
            action_contracts: action_contracts.to_vec().into_boxed_slice(),
        };
        self.terminal_runs_remove(run);
        if let Err(error) = self.admit_run_state(run, state, RuntimeState::Initial) {
            let removed = self.run_state_remove(run);
            if let Some(state) = removed {
                self.release_frame(state.frame);
            }
            self.discard_journal_sequence(run);
            return Err(error);
        }
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                if !self.run_state_contains(run) {
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }

    fn append_admission_header_journal_event(
        &mut self,
        run: RunId,
        event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        match self.append_journal_event(event) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.discard_journal_sequence(run);
                Err(RuntimeError::admission_header_persistence_failed(error))
            }
        }
    }

    fn build_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        use crate::admission::{AdmissionError, admit_artifact_run};

        match admit_artifact_run(self.artifact_store.as_ref(), self.policy, run, digest, caps) {
            Ok(admission) => Ok(Some(admission)),
            Err(AdmissionError::ArtifactNotFound { digest }) => {
                Err(RuntimeError::AdmissionArtifactNotFound { digest })
            }
            Err(AdmissionError::CapabilityDenied {
                action,
                required,
                granted,
            }) => Err(RuntimeError::AdmissionCapabilityDenied {
                action,
                required,
                granted,
            }),
            Err(AdmissionError::ResourceCapacityExceeded { available, .. }) => {
                Err(RuntimeError::ActiveRunCapacityExceeded {
                    capacity: usize::try_from(available).map_or(usize::MAX, |value| value),
                })
            }
            Err(AdmissionError::BudgetPolicyExceeded { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ResourceBudgetOverflow { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ResourceBudgetUnderflow { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ResourceBudgetInvalidCapacity { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ResourceStepCeilingExceeded { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ResourcePerTickCeilingExceeded { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ArtifactEnvelopeDecodeFailed) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ArtifactInvalidGateCount { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ArtifactInvalidProofFlag { .. }) => {
                Err(RuntimeError::AdmissionArtifactInvalid { digest })
            }
            Err(AdmissionError::ArtifactDigestMismatch { requested, found }) => {
                Err(RuntimeError::AdmissionArtifactDigestMismatch { requested, found })
            }
            Err(AdmissionError::ArtifactCertificateStale { digest, .. }) => {
                Err(RuntimeError::AdmissionArtifactStale { digest })
            }
            Err(AdmissionError::CapabilityCountMismatch {
                required_count,
                granted_count,
            }) => Err(RuntimeError::AdmissionCapabilityCountMismatch {
                required_count,
                granted_count,
            }),
        }
    }

    pub(crate) fn validate_submit_admission(
        &self,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.build_admission(run, workflow.digest(), caps)?;
        execution_binding::validate(self.artifact_store.as_ref(), self.policy, workflow, &[])?;
        Ok(())
    }

    /// Validates admission requirements for a submitted run with action contracts.
    ///
    /// Checks: contract ID matches its registry index, duplicate action IDs,
    /// well-formed contract names, that granted capabilities cover the required
    /// capabilities declared by the contracts, and that each workflow Do action
    /// has a corresponding contract at its registry index.
    pub(crate) fn validate_submit_admission_with_contracts(
        &self,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        let digest = workflow.digest();
        self.build_admission(run, digest, caps.clone())?;
        super::contract_validation::validate_contract_ids(action_contracts)?;
        super::contract_validation::validate_contract_capabilities(action_contracts, &caps)?;
        super::contract_validation::validate_workflow_action_binding(workflow, action_contracts)?;
        execution_binding::validate(
            self.artifact_store.as_ref(),
            self.policy,
            workflow,
            action_contracts,
        )?;
        Ok(())
    }
    pub fn handle_resume(&mut self, run: RunId) -> Result<ResumeResult, ResumeError> {
        self.validate_run_exists(run)?;
        let current_state = self.get_runtime_state_or_running(run);
        if current_state == RuntimeState::Running {
            return Ok(ResumeResult {
                run_id: run,
                status: ResumeStatus::AlreadyRunning,
                timestamp: current_timestamp(),
            });
        }
        if current_state != RuntimeState::Resumable {
            return Err(ResumeError::NotResumable {
                run_id: run,
                current_state,
            });
        }
        let timestamp = self.append_resumed_event(run)?;
        let drive_result = self.drive_run(run);
        self.observe_resume_drive_result(run, drive_result)?;
        Ok(ResumeResult {
            run_id: run,
            status: ResumeStatus::Resumed,
            timestamp,
        })
    }

    /// Recovers a run from a pre-hydrated `RunFrame` by loading the
    /// compiled workflow from the artifact store and constructing a full
    /// `RunState`.
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
    /// 7. Handle boundary (pending action / open ask)
    /// 8. Drive run from recovered state
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

        // Step 4: Validate the command's admission against the loaded
        // artifact and workflow before any state or journal mutation.
        // This runs for all recovery paths (full, raw, retry) because the
        // raw command path bypasses the Runtime prefix validation.
        //
        // For checkpoint recovery, the checkpoint's own admission is used.
        // For legacy/raw recovery without a checkpoint, the command carries
        // the Unverified enum variant and validation checks run identity
        // and artifact compatibility only.
        match &admission {
            crate::shard::types::RecoveredAdmissionKind::Historical(historical) => {
                crate::recovery::validate_recovered_admission_evidence(
                    crate::recovery::RecoveredAdmissionEvidence {
                        context: crate::recovery::RecoveredAdmissionContext {
                            run,
                            expected_workflow: workflow_digest,
                            runtime_policy: self.policy,
                        },
                        admission: historical,
                        artifact: &artifact,
                        workflow: &workflow,
                    },
                )?;
            }
            crate::shard::types::RecoveredAdmissionKind::Unverified {
                run_id,
                artifact_digest: cmd_digest,
            } => {
                // Validate run identity and artifact compatibility only.
                // Cannot validate policy pin without historical admission.
                if *run_id != run {
                    return Err(RuntimeError::Recovery {
                        error: format!(
                            "admission run identity mismatch: expected {run:?}, got {run_id:?}"
                        ),
                    });
                }
                if *cmd_digest != artifact_digest {
                    return Err(RuntimeError::Recovery {
                        error: format!(
                            "admission artifact digest mismatch: expected {artifact_digest:?}, got {cmd_digest:?}"
                        ),
                    });
                }
            }
        }

        // Step 4b: Validate policy pin for checkpoint recovery.
        // When a retry checkpoint carries an admission, its policy pin must
        // match the loaded artifact's policy before any state or journal mutation.
        // This applies to both Historical and Unverified command paths because
        // the public new(...).with_retry_checkpoint(...) API can attach a
        // checkpoint to an Unverified command without Runtime prefix validation.
        if let Some(checkpoint) = retry_checkpoint.as_ref() {
            if let Some(admission) = checkpoint.admission.as_ref() {
                crate::recovery::validate_recovered_policy_pin(admission, &artifact)?;
            }
        }

        // Step 4c: Compute Unverified fallback admission before state mutation.
        // For legacy/raw recovery without historical admission, build admission
        // using the artifact's required capabilities and current policy, then
        // force the Unverified pin so recovery never promotes to Pinned.
        // This must happen before any journal or run state mutation so failures
        // (capability denial, resource capacity) do not leave orphaned state.
        let unverified_admission = if matches!(
            admission,
            crate::shard::types::RecoveredAdmissionKind::Unverified { .. }
        ) {
            let caps = vb_core::CapabilitySet::from_grants(artifact.required_capabilities.clone());
            match self.build_admission(run, artifact_digest, caps)? {
                Some(adm) => Some(adm.with_policy_pin(vb_storage::RunPolicyPin::Unverified)),
                None => None,
            }
        } else {
            None
        };

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
            let (state, _attempt, _retry_step, _prior_seq) = checkpoint
                .restore(workflow, artifact_digest, &artifact.action_contracts)
                .map_err(|error| RuntimeError::Recovery {
                    error: format!("retry checkpoint validation failed: {error}"),
                })?;
            if state.frame.run_id() != run {
                return Err(RuntimeError::Recovery {
                    error: "retry checkpoint run identity mismatch".to_string(),
                });
            }
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
            self.prepare_run_slots(run)?;
            let state_admission = match admission {
                crate::shard::types::RecoveredAdmissionKind::Historical(h) => Some(h),
                crate::shard::types::RecoveredAdmissionKind::Unverified { .. } => {
                    unverified_admission
                }
            };
            crate::shard::types::RunState {
                registered_execution,
                frame,
                run_attempt: core::num::NonZeroU16::MIN,
                workflow,
                store: vb_core::value_store::ValueStore::with_max_slots(max_slots),
                action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
                admission: state_admission,
                collect_states,
                action_contracts: artifact.action_contracts.clone(),
            }
        };

        self.terminal_runs_remove(run);
        self.admit_run_state(run, state, crate::shard::types::RuntimeState::Initial)?;

        // Step 7: Handle boundary for suspended operations
        match boundary.kind() {
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
        }

        // Step 8: Drive run from recovered state
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                if !self.run_state_contains(run) {
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }

    fn validate_run_exists(&self, run: RunId) -> Result<(), ResumeError> {
        if !self.run_state_contains(run) {
            return Err(ResumeError::RunIdNotFound { run_id: run });
        }
        Ok(())
    }

    fn get_runtime_state_or_running(&self, run: RunId) -> RuntimeState {
        self.runtime_state_get(run).unwrap_or(RuntimeState::Running)
    }

    fn append_resumed_event(&mut self, run: RunId) -> Result<u64, ResumeError> {
        if !self.is_run_tracked(run) {
            return Err(ResumeError::IncompleteHydration { run_id: run });
        }
        self.apply(run, RuntimeEvent::Resume)
            .map_err(ResumeError::journal_append_failed_with_source)?;
        let timestamp = current_timestamp();
        let resumed_event = RuntimeJournalEvent::Resumed { run, timestamp };
        if let Err(source) = self.append_journal_event(resumed_event) {
            self.apply(run, RuntimeEvent::ResumeRollback)
                .map_err(ResumeError::journal_append_failed_with_source)?;
            return Err(ResumeError::journal_append_failed_with_source(source));
        }
        Ok(timestamp)
    }

    fn is_run_tracked(&self, run: RunId) -> bool {
        self.runtime_state_get(run).is_some()
    }

    fn observe_resume_drive_result(
        &mut self,
        run: RunId,
        result: RuntimeResult<()>,
    ) -> Result<(), ResumeError> {
        if let Err(source) = result {
            return Err(self.restore_resumable_after_drive_failure(run, source));
        }
        Ok(())
    }

    fn restore_resumable_after_drive_failure(
        &mut self,
        run: RunId,
        source: RuntimeError,
    ) -> ResumeError {
        match self.apply(run, RuntimeEvent::ResumeRollback) {
            Ok(()) => ResumeError::journal_append_failed_with_source(source),
            Err(rollback_error) => ResumeError::journal_append_failed_with_source(rollback_error),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        self.reject_unregistered_completion(ticket)?;
        self.apply_action_completion(ticket, output)
    }

    pub(crate) fn apply_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        self.require_pending_action_ownership(ticket)?;
        let (preflight, action_abi_digest) = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            let pf = preflight_action_completion(state, ticket, output)?;
            let abi = schedule_action_abi_digest(state, ticket.action)?;
            (pf, abi)
        };
        self.append_journal_event(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: preflight.ticket,
            output: preflight.output_slot,
            value: preflight.encoded_value.clone(),
            encoded_len: preflight.encoded_len,
            taint: preflight.taint,
            value_digest: preflight.value_digest,
            action_abi_digest,
        })?;
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(preflight.output_slot, preflight.value, preflight.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::advance_after_action_completion(state, preflight.ticket.step)?;
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot: preflight.output_slot,
            value: preflight.encoded_value,
        });
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: preflight.ticket.step,
        });
        self.drive_run(run)
    }

    pub(crate) fn handle_legacy_action_completion(
        &mut self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        let ticket = self.require_legacy_pending_action_ownership(run, step)?;
        self.reject_unregistered_completion(ticket)?;
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .step_state(ticket.step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information.
        // Journal append FIRST so a journal failure does not diverge frame and journal.
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step: ticket.step,
            output: SlotIdx::ZERO,
            attempt: ticket.attempt,
        })?;
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(ticket.step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        crate::shard::helpers::advance_after_action_completion(state, ticket.step)?;
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: ticket.step,
        });
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let code = failure.code;
        self.require_pending_action_ownership(ticket)?;
        let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
        // 1. Preflight (read-only): validate and decide the failure outcome.
        //    No mutation has occurred yet, so a journal append failure
        //    below cannot diverge memory-only state from durable evidence.
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_failure(state, ticket, &failure)?
        };
        // 2. Journal append FIRST. FailRun needs both ActionFailed and
        //    RunFailed to append as one same-run batch before any pending
        //    action, trace, frame, counter, or terminal-state mutation. If
        //    the batch is rejected by a bounded queued journal, no partial
        //    ActionFailed prefix can consume the last queue slot.
        let action_failed_event = RuntimeJournalEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            action: preflight.ticket.action,
            attempt: preflight.ticket.attempt,
        };
        if preflight.outcome == ActionFailureOutcome::FailRun {
            let attempt = self
                .run_state_get(run)
                .map(|state| state.run_attempt.get())
                .ok_or(RuntimeError::RunNotFound)?;
            self.append_journal_events_atomically([
                action_failed_event,
                RuntimeJournalEvent::RunFailed { run, attempt },
            ])?;
        } else {
            self.append_journal_event(action_failed_event)?;
        }
        // 3. Apply preflight mutations to the run frame / action_attempts.
        //    Held in a narrow scope so `pending_action_remove` and
        //    `trace_ring.push` can borrow `self` after the mutable
        //    state borrow ends.
        {
            let state = self
                .run_state_get_mut(run)
                .ok_or(RuntimeError::RunNotFound)?;
            apply_action_failure_preflight(state, &preflight)?;
        }
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            code,
        });
        // 4. Drive the run state machine forward using the preflighted outcome.
        match preflight.outcome {
            ActionFailureOutcome::RetryNow | ActionFailureOutcome::DriveHandler => {
                self.drive_run(run)
            }
            ActionFailureOutcome::FailRun => {
                let state = self.take_run_state(run)?;
                self.fail_run_state_with_retry_checkpoint(run, preflight.ticket.step, state)
            }
        }
    }

    pub fn ticket_with_retry_capacity(
        &self,
        ticket: ActionTicket,
        retry_policy: VbCoreRetryPolicy,
    ) -> RuntimeResult<ActionTicket> {
        let Some(state) = self.run_state_get(ticket.run) else {
            return Err(RuntimeError::RunNotFound);
        };
        if retry_policy != VbCoreRetryPolicy::Retryable
            || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
        {
            return Ok(ticket);
        }
        let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
        Ok(ActionTicket {
            capacity: ticket.capacity.max(policy.max_attempts),
            ..ticket
        })
    }
}
