impl Shard {
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
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        self.validate_secret_presence(&workflow)?;
        let supplied_inputs = inputs;
        let filled_inputs = self.prepare_secret_inputs(&workflow, inputs)?;
        let inputs = filled_inputs.as_slice();
        if let Some(metadata) = workflow.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        let digest = workflow.digest();
        let admission = self.build_admission_with_budget(run, &workflow, caps)?;
        execution_binding::validate(
            self.artifact_store.as_ref(),
            self.policy,
            &workflow,
            action_contracts,
        )?;
        let prepared_store = fresh_inputs::prepare(&workflow, supplied_inputs)?;
        let store = prepared_store.map_or(store, |prepared| prepared);
        self.prepare_run_slots(run)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        crate::shard::helpers::seed_input_slots(&mut frame, inputs, &workflow)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        let ir_compat_decision = admission
            .as_ref()
            .and_then(|admission| admission.ir_compat_decision());
        let tenant_admission = admission
            .as_ref()
            .and_then(|admission| crate::journal::tenant_admission_event(admission, run));
        let mut header_bytes: u64 = 0;
        if let (true, Some(admission)) = (persist_header, admission.as_ref()) {
            let mut events = vec![
                RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: digest,
                },
                RuntimeJournalEvent::RunAdmission {
                    admission: admission.clone(),
                },
            ];
            if let Some(decision) = ir_compat_decision {
                events.push(RuntimeJournalEvent::CompatibilityDecision { run, decision });
            }
            if let Some(event) = tenant_admission {
                events.push(event);
            }
            header_bytes = Self::journal_batch_bytes(&events)?;
            let header_result = self.append_journal_event_batch(&events);
            if let Err(error) = header_result {
                self.discard_journal_sequence(run);
                return Err(RuntimeError::admission_header_persistence_failed(error));
            }
        } else {
            if persist_header {
                let event = RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: digest,
                };
                header_bytes = header_bytes
                    .checked_add(Self::journal_event_bytes(&event)?)
                    .ok_or(RuntimeError::EncodeFailed)?;
                self.append_admission_header_journal_event(run, event)?;
            }
            if let Some(admission) = admission.as_ref() {
                let event = RuntimeJournalEvent::RunAdmission {
                    admission: admission.clone(),
                };
                header_bytes = header_bytes
                    .checked_add(Self::journal_event_bytes(&event)?)
                    .ok_or(RuntimeError::EncodeFailed)?;
                self.append_admission_header_journal_event(run, event)?;
                if let Some(decision) = ir_compat_decision {
                    let event = RuntimeJournalEvent::CompatibilityDecision { run, decision };
                    header_bytes = header_bytes
                        .checked_add(Self::journal_event_bytes(&event)?)
                        .ok_or(RuntimeError::EncodeFailed)?;
                    self.append_admission_header_journal_event(run, event)?;
                }
                if let Some(event) = tenant_admission {
                    header_bytes = header_bytes
                        .checked_add(Self::journal_event_bytes(&event)?)
                        .ok_or(RuntimeError::EncodeFailed)?;
                    self.append_admission_header_journal_event(run, event)?;
                }
            }
        }
        self.counters.inc_submitted();
        let frame_step_count = frame.step_count();
        let state = RunState {
            registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
            frame,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow,
            store,
            action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
            admission,
            run_started_millis: crate::shard::helpers::wall_clock_millis_since_epoch(),
            collect_states: CollectStates::new(),
            journal_bytes_written: header_bytes,
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
}
