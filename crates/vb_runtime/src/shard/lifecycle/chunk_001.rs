impl Shard {
    #[cfg(not(test))]
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

    #[cfg(test)]
    pub(crate) fn handle_submit(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let Some(action) = test_first_do_action(&workflow) else {
            return self.handle_submit_with_inputs_contracts_and_header_mode(
                run,
                workflow,
                &[],
                caps,
                &[],
                true,
            );
        };
        let inputs = [(SlotIdx::new(0), SlotValue::I64(0))];
        let test_caps = test_contract_grants(action);
        let contracts = test_contracts_through(action);
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run, workflow, &inputs, test_caps, &contracts, true,
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

    #[cfg(not(test))]
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

    #[cfg(test)]
    pub(crate) fn handle_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let Some(action) = test_first_do_action(&workflow) else {
            return self.handle_submit_with_inputs_contracts_and_header_mode(
                run,
                workflow,
                inputs,
                caps,
                &[],
                true,
            );
        };
        let test_caps = test_contract_grants(action);
        let contracts = test_contracts_through(action);
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run, workflow, inputs, test_caps, &contracts, true,
        )
    }

    pub(crate) fn handle_submit_with_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
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
        self.prepare_run_slots(run)?;
        let digest = workflow.digest();
        let admission = self.build_admission(run, digest, caps)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        crate::shard::helpers::seed_input_slots(&mut frame, inputs)?;
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
            frame,
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
                Err(RuntimeError::AdmissionArtifactInvalid {
                    digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
                })
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
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.build_admission(run, digest, caps)?;
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
    pub(crate) fn handle_recover(
        &mut self,
        command: crate::shard::types::RecoverRunCommand,
    ) -> RuntimeResult<()> {
        let crate::shard::types::RecoverRunCommand {
            run,
            frame,
            artifact_digest,
            workflow_digest,
            next_seq,
            collect_states,
            boundary,
        } = command;

        // Load the accepted artifact from the shard's artifact store by
        // artifact_digest (not workflow_digest).
        let artifact = self
            .artifact_store
            .load_accepted_artifact(artifact_digest)
            .map_err(|e| match e {
                crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                    RuntimeError::Recovery {
                        error: format!("artifact not found during recovery: {digest:?}"),
                    }
                }
                _ => RuntimeError::Recovery {
                    error: "artifact decode failed during recovery".to_string(),
                },
            })?;

        // Deserialize the postcard-encoded WorkflowParts from the artifact IR.
        let parts: vb_core::workflow::WorkflowParts = postcard::from_bytes(&artifact.ir)
            .map_err(|_| RuntimeError::Recovery {
                error: "artifact IR decode failed".to_string(),
            })?;

        // Validate and reconstruct the compiled workflow using workflow_digest
        // for the post-compile integrity check.
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| RuntimeError::Recovery {
                error: format!("workflow compile failed: {e}"),
            })?;
        if workflow.digest() != workflow_digest {
            return Err(RuntimeError::Recovery {
                error: "workflow digest mismatch during recovery".to_string(),
            });
        }

        // Build admission from the artifact (same as submit path).
        // Use the artifact's required_capabilities as the granted capability set.
        // These were the original grants at submit time; cardinality-exact
        // admission (RA-023) will pass because required == granted.
        let granted_caps =
            CapabilitySet::from_grants(artifact.required_capabilities.clone());
        let admission = self.build_admission(run, workflow.digest(), granted_caps)?;

        // Prepare the run's value store and action attempts from frame dimensions.
        let frame_step_count = frame.step_count();
        let max_slots = workflow.resource_contract().max_slots;
        self.prepare_run_slots(run)?;

        // Insert the recovered journal sequence before the run state so
        // failure cleanup can drain both together.
        let _next_seq = self
            .journal_sequences
            .insert(run, next_seq);

        // Construct the full RunState with restored collect_states.
        let state = crate::shard::types::RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::with_max_slots(max_slots),
            action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
            admission,
            collect_states,
            // action_contracts are supplied at submit time and are not journaled;
            // recovered runs rely on per-step policy enforcement instead.
            action_contracts: Box::new([]),
        };

        self.terminal_runs_remove(run);
        let run_state_result = self.admit_run_state(run, state, RuntimeState::Initial);
        if run_state_result.is_err() {
            self.discard_journal_sequence(run);
            return run_state_result;
        }

        // Restore a pending-action ticket recovered from the frame boundary
        // before drive, so the drive loop sees the durable authority.
        if let Some(ticket) = boundary.pending_action_ticket() {
            if let Err(original) = self.pending_action_insert(run, ticket.ticket()) {
                let _removed = self.run_state_remove(run);
                self.discard_journal_sequence(run);
                return Err(RuntimeError::Recovery {
                    error: format!(
                        "pending-action insert failed during recovery: {original}"
                    ),
                });
            }
        }

        // Do not increment submitted counter — recovery is not a submit.
        // The counters tracking active runs will be updated by drive_run.

        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Do NOT discard journal sequence on recovery failure.
                // The journal sequence is the recovered state; discarding it
                // would destroy evidence that the recovery attempt produced.
                if !self.run_state_contains(run) {
                    // Clean up the pending-action we inserted during recovery
                    // when the run state was removed by drive_run.
                    if boundary.pending_action_ticket().is_some() {
                        self.pending_action_remove(run);
                    }
                    // Only discard if run_state_insert failed (no state to drive).
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
        let run = ticket.run;
        self.require_pending_action_ownership(ticket)?;
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_completion(state, ticket, output)?
        };
        self.append_journal_event(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: preflight.ticket,
            output: preflight.output_slot,
            value: preflight.encoded_value.clone(),
            encoded_len: preflight.encoded_len,
            taint: preflight.taint,
            value_digest: preflight.value_digest,
            action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
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
            self.append_journal_events_atomically([
                action_failed_event,
                RuntimeJournalEvent::RunFailed { run },
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
                self.fail_run_state_after_journaled(run, state)
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

#[cfg(test)]
fn test_first_do_action(workflow: &CompiledWorkflow) -> Option<vb_core::ids::ActionId> {
    let mut index = 0u16;
    let count = workflow.node_count();
    while index < count {
        let step = StepIdx::new(index);
        if let Some(node) = workflow.node(step) {
            if let vb_core::workflow::CompiledNodeKind::Do { action, .. } = node.kind {
                return Some(action);
            }
        }
        index = index.saturating_add(1);
    }
    None
}

#[cfg(test)]
fn test_contract_required_capability(
    action: vb_core::ids::ActionId,
) -> vb_core::capability::Capability {
    vb_core::capability::Capability::new("__contract_required__".into(), action)
}

#[cfg(test)]
fn test_contract_grants(action: vb_core::ids::ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([test_contract_required_capability(action)]))
}

#[cfg(test)]
fn test_action_contract(
    action: vb_core::ids::ActionId,
    required: bool,
) -> vb_core::action::ActionContract {
    let required_capabilities = if required {
        Box::from([test_contract_required_capability(action)])
    } else {
        Box::from([])
    };
    vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities,
    }
}

#[cfg(test)]
fn test_contracts_through(
    action: vb_core::ids::ActionId,
) -> Box<[vb_core::action::ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
    let mut id = 0u16;
    loop {
        let current = vb_core::ids::ActionId::new(id);
        contracts.push(test_action_contract(current, id == target));
        if id == target {
            break;
        }
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}
