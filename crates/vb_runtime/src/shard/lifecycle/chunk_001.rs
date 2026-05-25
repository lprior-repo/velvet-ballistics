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
        if self.runs.contains_key(&run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if self.runs.len() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        let digest = workflow.digest();
        let admission = self.build_admission(run, digest, caps)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        crate::shard::helpers::seed_input_slots(&mut frame, inputs)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        if persist_header {
            match self.append_journal_event(RuntimeJournalEvent::RunSubmitted {
                run,
                workflow: digest,
            }) {
                Ok(()) => {}
                Err(error) => {
                    self.discard_journal_sequence(run);
                    return Err(error);
                }
            }
        }
        if let Some(admission) = admission.as_ref() {
            match self.append_journal_event(RuntimeJournalEvent::RunAdmission {
                admission: admission.clone(),
            }) {
                Ok(()) => {}
                Err(error) => {
                    self.discard_journal_sequence(run);
                    return Err(error);
                }
            }
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
        self.terminal_runs.swap_remove(&run);
        self.runs.insert(run, state);
        self.apply(run, RuntimeEvent::Submit);
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                if !self.runs.contains_key(&run) {
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }

    fn build_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        use crate::admission::{admit_artifact_run, AdmissionError};

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
        }
    }

    pub(crate) fn validate_submit_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.build_admission(run, digest, caps).map(|_| ())
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

    fn validate_run_exists(&self, run: RunId) -> Result<(), ResumeError> {
        if !self.runs.contains_key(&run) {
            return Err(ResumeError::RunIdNotFound { run_id: run });
        }
        Ok(())
    }

    fn get_runtime_state_or_running(&self, run: RunId) -> RuntimeState {
        self.runtime_states
            .get(&run)
            .copied()
            .unwrap_or(RuntimeState::Running)
    }

    fn append_resumed_event(&mut self, run: RunId) -> Result<u64, ResumeError> {
        if !self.is_run_tracked(run) {
            return Err(ResumeError::IncompleteHydration { run_id: run });
        }
        self.apply(run, RuntimeEvent::Resume);
        let timestamp = current_timestamp();
        let resumed_event = RuntimeJournalEvent::Resumed { run, timestamp };
        if let Err(source) = self.append_journal_event(resumed_event) {
            self.apply(run, RuntimeEvent::ResumeRollback);
            return Err(ResumeError::journal_append_failed_with_source(source));
        }
        Ok(timestamp)
    }

    fn is_run_tracked(&self, run: RunId) -> bool {
        self.runtime_states.contains_key(&run)
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
        self.apply(run, RuntimeEvent::ResumeRollback);
        ResumeError::journal_append_failed_with_source(source)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let preflight = {
            let state = self.runs.get(&run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_completion(state, ticket, output)?
        };
        self.append_journal_event(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: preflight.ticket,
            output: preflight.output_slot,
            value: preflight.encoded_value.clone(),
            encoded_len: preflight.encoded_len,
            taint: preflight.taint,
            value_digest: preflight.value_digest,
        })?;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(preflight.output_slot, preflight.value, preflight.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::advance_after_action_completion(state, preflight.ticket.step)?;
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
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information.
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: SlotIdx::ZERO,
            attempt: 1,
        })?;
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
        let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
        let outcome = self.apply_action_failure_to_state(ticket, failure)?;
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code,
        });
        self.append_journal_event(RuntimeJournalEvent::ActionFailed {
            run,
            step: ticket.step,
            action: ticket.action,
            attempt: ticket.attempt,
        })?;
        match outcome {
            ActionFailureOutcome::RetryNow | ActionFailureOutcome::DriveHandler => {
                self.drive_run(run)
            }
            ActionFailureOutcome::FailRun => {
                let state = self.take_run_state(run)?;
                // apply() handles runtime_states mutation; fail_run_state handles cleanup only
                self.apply(run, RuntimeEvent::Fail);
                self.fail_run_state(run, state)
            }
        }
    }

    pub fn ticket_with_retry_capacity(
        &self,
        ticket: ActionTicket,
        retry_policy: VbCoreRetryPolicy,
    ) -> RuntimeResult<ActionTicket> {
        let Some(state) = self.runs.get(&ticket.run) else {
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

    fn apply_action_failure_to_state(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<ActionFailureOutcome> {
        let state = self
            .runs
            .get_mut(&ticket.run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::validate_action_completion(state, ticket)?;
        if retry_is_available(state, ticket, failure.retry_policy)? {
            state
                .frame
                .set_pc(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            return Ok(ActionFailureOutcome::RetryNow);
        }
        apply_error_handler(state, ticket)
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
