type ActionAbiDigestRegistry = Box<[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)]>;
type SubmitAdmissionAuthority = (
    Option<crate::admission::RunAdmission>,
    ActionAbiDigestRegistry,
);

struct RecoveredRunStateParts {
    run: RunId,
    frame: vb_core::frame::RunFrame,
    workflow: CompiledWorkflow,
    granted_caps: CapabilitySet,
    action_contracts: Box<[vb_core::action::ActionContract]>,
    collect_states: CollectStates,
}

struct RecoveredWorkflowParts {
    workflow: CompiledWorkflow,
    granted_caps: CapabilitySet,
    action_contracts: Box<[vb_core::action::ActionContract]>,
}

struct RecoveredRunAuthorities {
    state: RunState,
    action_abi_digests: ActionAbiDigestRegistry,
}

struct PreparedRecoveredRun {
    run: RunId,
    state: RunState,
    next_seq: vb_storage::EventSeq,
    action_abi_digests: ActionAbiDigestRegistry,
    boundary: crate::recovery::RecoveredRunBoundary,
}

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
        let digest = workflow.digest();
        let (admission, action_abi_digests) =
            self.build_admission_with_action_abi_authority(run, digest, caps, action_contracts)?;
        Self::validate_workflow_action_abi_authority(&workflow, &action_abi_digests)?;
        self.prepare_run_slots(run)?;
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
        self.run_state_insert(run, state)?;
        self.action_abi_digests_store(run, action_abi_digests);
        if let Err(error) = self.apply(run, RuntimeEvent::Submit) {
            let removed = self.run_state_remove(run);
            if let Some(state) = removed {
                self.release_frame(state.frame);
            }
            self.action_abi_digests_remove(run);
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

    fn validate_workflow_action_abi_authority(
        workflow: &CompiledWorkflow,
        digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
    ) -> RuntimeResult<()> {
        let mut index = 0u16;
        let count = workflow.node_count();
        while index < count {
            let step = StepIdx::new(index);
            let Some(node) = workflow.node(step) else {
                return Err(Self::action_abi_authority_missing());
            };
            if let CompiledNodeKind::Do { action, .. } = node.kind {
                Self::validate_action_abi_authority(action, digests)?;
            }
            index = index
                .checked_add(1)
                .ok_or_else(Self::action_abi_authority_missing)?;
        }
        Ok(())
    }

    fn validate_action_abi_authority(
        action: vb_core::ids::ActionId,
        digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
    ) -> RuntimeResult<()> {
        let Some((_, digest)) = digests.iter().find(|(id, _)| *id == action) else {
            return Err(Self::action_abi_authority_missing());
        };
        if digest.as_bytes() == [0u8; 32] {
            return Err(Self::action_abi_authority_missing());
        }
        Ok(())
    }

    fn action_abi_authority_missing() -> RuntimeError {
        RuntimeError::RecoveryCannotResume {
            reason: String::from("action_abi_digests_missing"),
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

    fn build_admission_with_action_abi_authority(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<SubmitAdmissionAuthority> {
        let action_abi_digests =
            crate::recovery::action_abi_digests_from_contracts(action_contracts)?;
        let admission = self.build_admission(run, digest, caps)?;
        self.validate_artifact_bound_action_abi_authority(
            digest,
            action_contracts,
            &action_abi_digests,
        )?;
        Ok((admission, action_abi_digests))
    }

    fn validate_artifact_bound_action_abi_authority(
        &self,
        digest: vb_core::ids::WorkflowDigest,
        action_contracts: &[vb_core::action::ActionContract],
        action_abi_digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
    ) -> RuntimeResult<()> {
        match self.policy {
            vb_core::policy::RuntimePolicy::Strict | vb_core::policy::RuntimePolicy::Journaled => {
                let artifact = self.load_submit_artifact(digest)?;
                let accepted_abi_digests = crate::recovery::action_abi_digests_from_contracts(
                    &artifact.action_contracts,
                )?;
                Self::validate_action_registry_shape(
                    artifact.digest,
                    &artifact.action_contracts,
                    action_contracts,
                    &accepted_abi_digests,
                    action_abi_digests,
                )?;
                Self::validate_action_abi_digest_registry(
                    artifact.digest,
                    &accepted_abi_digests,
                    action_abi_digests,
                )
            }
            vb_core::policy::RuntimePolicy::Relaxed => Ok(()),
            _ => Err(RuntimeError::AdmissionArtifactInvalid { digest }),
        }
    }

    fn load_submit_artifact(
        &self,
        digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
        self.artifact_store
            .load_accepted_artifact(digest)
            .map_err(|error| match error {
                crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                    RuntimeError::AdmissionArtifactNotFound { digest }
                }
                crate::admission::ArtifactEnvelopeError::ArtifactDigestMismatch {
                    requested,
                    found,
                } => RuntimeError::AdmissionArtifactDigestMismatch { requested, found },
                _ => RuntimeError::AdmissionArtifactInvalid { digest },
            })
    }

    fn validate_action_registry_shape(
        digest: vb_core::ids::WorkflowDigest,
        accepted_contracts: &[vb_core::action::ActionContract],
        submitted_contracts: &[vb_core::action::ActionContract],
        accepted_digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
        submitted_digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
    ) -> RuntimeResult<()> {
        if accepted_contracts.len() != submitted_contracts.len()
            || accepted_digests.len() != submitted_digests.len()
        {
            return Err(RuntimeError::AdmissionActionRegistryMismatch {
                digest,
                expected_count: accepted_contracts.len(),
                submitted_count: submitted_contracts.len(),
            });
        }
        Ok(())
    }

    fn validate_action_abi_digest_registry(
        digest: vb_core::ids::WorkflowDigest,
        accepted_digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
        submitted_digests: &[(vb_core::ids::ActionId, vb_core::ids::WorkflowDigest)],
    ) -> RuntimeResult<()> {
        for ((accepted_action, expected), (submitted_action, submitted)) in
            accepted_digests.iter().zip(submitted_digests.iter())
        {
            if accepted_action != submitted_action {
                return Err(RuntimeError::AdmissionActionRegistryMismatch {
                    digest,
                    expected_count: accepted_digests.len(),
                    submitted_count: submitted_digests.len(),
                });
            }
            if expected != submitted {
                return Err(RuntimeError::AdmissionActionAbiDigestMismatch {
                    action: *accepted_action,
                    expected: *expected,
                    submitted: *submitted,
                });
            }
        }
        Ok(())
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

    pub(crate) fn validate_submit_admission_with_contracts(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        self.build_admission_with_action_abi_authority(run, digest, caps, action_contracts)?;
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
    /// The accepted-artifact digest comes from durable `RunAdmission` and is
    /// used to look up the `AcceptedArtifact`; the workflow/source digest from
    /// the journal summary is retained as a binding check. The artifact `ir`
    /// field contains postcard-encoded `WorkflowParts`, which are deserialized
    /// and validated via `CompiledWorkflow::try_from_parts`.
    ///
    /// Recovery differs from submit in three ways:
    /// - No `prepare_run_slots` / `take_frame_for` — the frame is pre-hydrated.
    /// - The artifact store is queried by digest rather than the workflow being
    ///   passed directly.
    /// - Admission is rebuilt from the accepted artifact's required grants.
    /// - Pending-action recovery restores the durable ticket side table and
    ///   parks the run in `Resumable` until the action boundary is resolved.
    pub(crate) fn handle_recover(
        &mut self,
        command: crate::shard::types::RecoverRunCommand,
    ) -> RuntimeResult<()> {
        let prepared = self.prepare_recovered_run(command)?;
        self.install_recovered_run(prepared)
    }

    fn prepare_recovered_run(
        &mut self,
        command: crate::shard::types::RecoverRunCommand,
    ) -> RuntimeResult<PreparedRecoveredRun> {
        if self.run_state_contains(command.run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        let run = command.run;
        let next_seq = command.next_seq;
        let boundary = command.boundary;
        let recovered =
            self.load_recovered_workflow_parts(command.artifact_digest, command.workflow_digest)?;
        let authorities = self.recovered_run_authorities(command, recovered)?;
        Ok(PreparedRecoveredRun {
            run,
            state: authorities.state,
            next_seq,
            action_abi_digests: authorities.action_abi_digests,
            boundary,
        })
    }

    fn load_recovered_workflow_parts(
        &self,
        artifact_digest: vb_core::ids::WorkflowDigest,
        workflow_digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<RecoveredWorkflowParts> {
        let (workflow, granted_caps, action_contracts) =
            self.load_recovered_workflow(artifact_digest, workflow_digest)?;
        Ok(RecoveredWorkflowParts {
            workflow,
            granted_caps,
            action_contracts,
        })
    }

    fn recovered_run_authorities(
        &mut self,
        command: crate::shard::types::RecoverRunCommand,
        recovered: RecoveredWorkflowParts,
    ) -> RuntimeResult<RecoveredRunAuthorities> {
        let action_abi_digests =
            crate::recovery::action_abi_digests_from_contracts(&recovered.action_contracts)?;
        self.reserve_action_abi_digest_slot(command.run)?;
        let state = self.recovered_run_state_from_command(command, recovered)?;
        Ok(RecoveredRunAuthorities {
            state,
            action_abi_digests,
        })
    }

    fn recovered_run_state_from_command(
        &self,
        command: crate::shard::types::RecoverRunCommand,
        recovered: RecoveredWorkflowParts,
    ) -> RuntimeResult<RunState> {
        self.recovered_run_state(RecoveredRunStateParts {
            run: command.run,
            frame: command.frame,
            workflow: recovered.workflow,
            granted_caps: recovered.granted_caps,
            action_contracts: recovered.action_contracts,
            collect_states: command.collect_states,
        })
    }

    fn install_recovered_run(&mut self, prepared: PreparedRecoveredRun) -> RuntimeResult<()> {
        self.terminal_runs_remove(prepared.run);
        self.restore_journal_sequence(prepared.run, prepared.next_seq)?;
        self.action_abi_digests_store(prepared.run, prepared.action_abi_digests);
        match prepared.boundary.kind() {
            crate::recovery::RecoveredRunBoundaryKind::PendingAction => {
                self.recover_pending_action_with_sequence_rollback(
                    prepared.run,
                    prepared.state,
                    prepared.boundary,
                )
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenAsk => {
                self.recover_open_ask_with_sequence_rollback(
                    prepared.run,
                    prepared.state,
                    prepared.boundary,
                )
            }
            crate::recovery::RecoveredRunBoundaryKind::None => {
                self.insert_and_drive_recovered_run(prepared.run, prepared.state)
            }
        }
    }

    fn insert_and_drive_recovered_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        if let Err(error) = self.run_state_insert(run, state) {
            self.action_abi_digests_remove(run);
            self.discard_journal_sequence(run);
            return Err(error);
        }
        self.drive_inserted_recovered_run(run)
    }

    fn drive_inserted_recovered_run(&mut self, run: RunId) -> RuntimeResult<()> {
        // Do not increment submitted counter — recovery is not a submit.
        // The counters tracking active runs will be updated by drive_run.
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Do NOT discard journal sequence on recovery failure.
                // The journal sequence is the recovered state; discarding it
                // would destroy evidence that the recovery attempt produced.
                if !self.run_state_contains(run) {
                    // Only discard if run_state_insert failed (no state to drive).
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }

    fn recover_pending_action_with_sequence_rollback(
        &mut self,
        run: RunId,
        state: RunState,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<()> {
        match self.recover_pending_action(run, state, boundary) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.action_abi_digests_remove(run);
                self.discard_journal_sequence(run);
                Err(error)
            }
        }
    }

    fn recover_open_ask_with_sequence_rollback(
        &mut self,
        run: RunId,
        state: RunState,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<()> {
        match self.recover_open_ask(run, state, boundary) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.action_abi_digests_remove(run);
                self.discard_journal_sequence(run);
                Err(error)
            }
        }
    }

    fn load_recovered_workflow(
        &self,
        artifact_digest: vb_core::ids::WorkflowDigest,
        workflow_digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<(CompiledWorkflow, CapabilitySet, Box<[vb_core::action::ActionContract]>)> {
        let artifact = self.load_recovered_artifact(artifact_digest)?;
        let parts: vb_core::workflow::WorkflowParts = postcard::from_bytes(&artifact.ir)
            .map_err(|_| RuntimeError::Recovery {
                error: "artifact IR decode failed".to_string(),
            })?;
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| RuntimeError::Recovery {
                error: format!("workflow compile failed: {e}"),
            })?;
        self.validate_recovered_artifact_binding(&artifact, artifact_digest, workflow_digest)?;
        Ok((
            workflow,
            CapabilitySet::from_grants(artifact.required_capabilities.clone()),
            artifact.action_contracts,
        ))
    }

    fn validate_recovered_artifact_binding(
        &self,
        artifact: &vb_storage::admission::AcceptedArtifact,
        artifact_digest: vb_core::ids::WorkflowDigest,
        workflow_digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<()> {
        if artifact.digest != artifact_digest {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("artifact_digest_mismatch"),
            });
        }
        if artifact.source_digest != workflow_digest {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("workflow_digest_mismatch"),
            });
        }
        Ok(())
    }

    fn load_recovered_artifact(
        &self,
        artifact_digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
        self.artifact_store
            .load_accepted_artifact(artifact_digest)
            .map_err(|e| match e {
                crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                    let _ = digest;
                    RuntimeError::RecoveryCannotResume {
                        reason: String::from("artifact_missing"),
                    }
                }
                _ => RuntimeError::RecoveryCannotResume {
                    reason: String::from("artifact_decode_failed"),
                },
            })
    }

    fn recovered_run_state(
        &self,
        parts: RecoveredRunStateParts,
    ) -> RuntimeResult<RunState> {
        let admission = self.build_admission(
            parts.run,
            parts.workflow.digest(),
            parts.granted_caps,
        )?;
        let frame_step_count = parts.frame.step_count();
        let max_slots = parts.workflow.resource_contract().max_slots;
        Ok(RunState {
            frame: parts.frame,
            workflow: parts.workflow,
            store: ValueStore::with_max_slots(max_slots),
            action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
            admission,
            collect_states: parts.collect_states,
            action_contracts: parts.action_contracts,
        })
    }

    fn recover_pending_action(
        &mut self,
        run: RunId,
        mut state: RunState,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<()> {
        let Some(evidence) = boundary.pending_action_ticket() else {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_actions"),
            });
        };
        let ticket = evidence.ticket();
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        crate::shard::helpers::validate_action_completion(&state, ticket)?;
        self.validate_recovered_action_schedule(run, &state, evidence)?;
        crate::engine::action::resolve_contract(ticket.action, &state.action_contracts).map_err(
            |_| RuntimeError::RecoveryCannotResume {
                reason: String::from("action_contracts_missing"),
            },
        )?;
        self.pending_action_insert(run, ticket)?;
        if let Err(error) = self.run_state_insert(run, state) {
            let _removed = self.pending_action_remove(run);
            return Err(error);
        }
        self.apply_recovered_pending_action_state(run)
    }

    fn recover_open_ask(
        &mut self,
        run: RunId,
        state: RunState,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<()> {
        let Some(open_ask) = boundary.open_ask() else {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            });
        };
        self.validate_recovered_open_ask(&state, open_ask)?;
        if self.pending_timer_contains(run) {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            });
        }
        self.run_state_insert(run, state)?;
        self.apply_recovered_open_ask_state(run)
    }

    fn validate_recovered_open_ask(
        &self,
        state: &RunState,
        open_ask: crate::recovery::RecoveredOpenAsk,
    ) -> RuntimeResult<()> {
        let step = open_ask.step();
        Self::validate_recovered_open_ask_frame(state, step)?;
        let resume_step = Self::validate_recovered_open_ask_node(state, step)?;
        Self::validate_recovered_open_ask_resume(state, resume_step)
    }

    fn validate_recovered_open_ask_frame(
        state: &RunState,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        if state.frame.pc() != step
            || state.frame.step_state(step) != Ok(vb_core::frame::StepState::Asking)
        {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            });
        }
        Ok(())
    }

    fn validate_recovered_open_ask_node(
        state: &RunState,
        step: StepIdx,
    ) -> RuntimeResult<Option<StepIdx>> {
        let Some(node) = state.workflow.node(step) else {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            });
        };
        let CompiledNodeKind::Ask {
            timeout_slot: None, ..
        } = node.kind
        else {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            });
        };
        Ok(node.next)
    }

    fn validate_recovered_open_ask_resume(
        state: &RunState,
        resume_step: Option<StepIdx>,
    ) -> RuntimeResult<()> {
        match resume_step.and_then(|resume| state.workflow.node(resume)) {
            Some(resume) if matches!(resume.kind, CompiledNodeKind::AskResume { .. }) => Ok(()),
            _ => Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_asks"),
            }),
        }
    }

    fn validate_recovered_action_schedule(
        &self,
        run: RunId,
        state: &RunState,
        evidence: crate::recovery::RecoveredPendingActionTicket,
    ) -> RuntimeResult<()> {
        let ticket = evidence.ticket();
        let input = crate::shard::helpers::action_input_slot(state, ticket.step)?;
        let output = crate::shard::helpers::action_output_slot(state, ticket.step)?;
        if input != evidence.input() || output != evidence.output() {
            return Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("pending_actions"),
            });
        }
        let action_abi_digest = self.action_abi_digest_for_run_action(run, ticket.action)?;
        if action_abi_digest == evidence.action_abi_digest() {
            Ok(())
        } else {
            Err(RuntimeError::RecoveryCannotResume {
                reason: String::from("action_abi_digest_mismatch"),
            })
        }
    }

    fn apply_recovered_pending_action_state(&mut self, run: RunId) -> RuntimeResult<()> {
        if let Err(error) = self.apply(run, RuntimeEvent::AwaitAction) {
            let _removed = self.pending_action_remove(run);
            if let Some(state) = self.run_state_remove(run) {
                self.release_frame(state.frame);
            }
            self.action_abi_digests_remove(run);
            return Err(error);
        }
        Ok(())
    }

    fn apply_recovered_open_ask_state(&mut self, run: RunId) -> RuntimeResult<()> {
        if let Err(error) = self.apply(run, RuntimeEvent::AwaitTimer) {
            if let Some(state) = self.run_state_remove(run) {
                self.release_frame(state.frame);
            }
            self.action_abi_digests_remove(run);
            return Err(error);
        }
        Ok(())
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
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_completion(state, ticket, output)?
        };
        let action_abi_digest =
            self.action_abi_digest_for_run_action(run, preflight.ticket.action)?;
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
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .step_state(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information.
        // Journal append FIRST so a journal failure does not diverge frame and journal.
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: SlotIdx::ZERO,
            attempt: 1,
        })?;
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
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
