impl Shard {
    // =============================================================================
    // Submit lifecycle methods
    // =============================================================================

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
        self.run_state_insert(run, state);
        self.apply(run, RuntimeEvent::Submit);
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
        }
    }

    #[allow(dead_code)]
    pub(crate) fn validate_submit_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.build_admission(run, digest, caps)?;
        Ok(())
    }
}
