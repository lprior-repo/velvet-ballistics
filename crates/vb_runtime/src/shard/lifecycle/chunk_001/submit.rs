/// Submission header options shared by every submit wrapper.
#[derive(Debug, Clone, Copy)]
struct SubmitOptions {
    tenant: Option<vb_core::ids::TenantId>,
    lifetime_cap: Option<std::num::NonZeroU64>,
    persist_header: bool,
}

impl SubmitOptions {
    const fn persist(tenant: Option<vb_core::ids::TenantId>) -> Self {
        Self {
            tenant,
            lifetime_cap: None,
            persist_header: true,
        }
    }

    const fn pre_persisted() -> Self {
        Self {
            tenant: None,
            lifetime_cap: None,
            persist_header: false,
        }
    }
}

impl Shard {
    fn handle_submit_with_inputs_contracts_and_header_mode(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
        options: SubmitOptions,
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
        let filled_inputs = self.prepare_secret_inputs(&workflow, inputs)?;
        let inputs = filled_inputs.as_slice();
        if let Some(metadata) = workflow.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        let digest = workflow.digest();
        let run_started_millis = crate::shard::helpers::wall_clock_millis_since_epoch()
            .ok_or_else(|| RuntimeError::Core {
                source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "wall_clock_unavailable".into(),
                }),
            })?;
        let admission = self
            .build_admission_with_budget(run, &workflow, caps, options.lifetime_cap)?
            .map(|admission| match options.tenant {
                Some(tenant) => admission.with_tenant(tenant),
                None => admission,
            });
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
        let ir_compat_decision = admission
            .as_ref()
            .and_then(|admission| admission.ir_compat_decision());
        let tenant_admission = admission
            .as_ref()
            .and_then(|admission| crate::journal::tenant_admission_event(admission, run));
        let quota_charge = admission.as_ref().and_then(|admission| {
            admission
                .budget()
                .map(|budget| (admission.tenant(), budget.max_cost_units))
        });
        if let Some((tenant, cost_units)) = quota_charge
            && let Err(error) = self.reserve_admission_quota(run, tenant, cost_units)
        {
            self.release_frame(frame);
            return Err(error);
        }
        let mut header_bytes: u64 = 0;
        if let (true, Some(admission)) = (options.persist_header, admission.as_ref()) {
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
            if admission.budget().is_some() {
                events.push(RuntimeJournalEvent::RunBudgetOrigin {
                    run,
                    started_millis: run_started_millis,
                });
            }
            header_bytes = self.journal_batch_bytes(&events)?;
            let header_result = self.append_journal_event_batch(&events);
            if let Err(error) = header_result {
                self.release_frame(frame);
                self.release_admission_quota(run)?;
                self.discard_journal_sequence(run);
                return Err(RuntimeError::admission_header_persistence_failed(error));
            }
        } else {
            if options.persist_header {
                let event = RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: digest,
                };
                header_bytes = header_bytes
                    .checked_add(self.journal_event_bytes(&event)?)
                    .ok_or(RuntimeError::EncodeFailed)?;
                self.append_admission_header_journal_event(run, event)?;
            }
            if let Some(admission) = admission.as_ref() {
                let event = RuntimeJournalEvent::RunAdmission {
                    admission: admission.clone(),
                };
                header_bytes = header_bytes
                    .checked_add(self.journal_event_bytes(&event)?)
                    .ok_or(RuntimeError::EncodeFailed)?;
                self.append_admission_header_journal_event(run, event)?;
                if let Some(decision) = ir_compat_decision {
                    let event = RuntimeJournalEvent::CompatibilityDecision { run, decision };
                    header_bytes = header_bytes
                        .checked_add(self.journal_event_bytes(&event)?)
                        .ok_or(RuntimeError::EncodeFailed)?;
                    self.append_admission_header_journal_event(run, event)?;
                }
                if let Some(event) = tenant_admission {
                    header_bytes = header_bytes
                        .checked_add(self.journal_event_bytes(&event)?)
                        .ok_or(RuntimeError::EncodeFailed)?;
                    self.append_admission_header_journal_event(run, event)?;
                }
                if admission.budget().is_some() {
                    let event = RuntimeJournalEvent::RunBudgetOrigin {
                        run,
                        started_millis: run_started_millis,
                    };
                    header_bytes = header_bytes
                        .checked_add(self.journal_event_bytes(&event)?)
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
            run_started_millis: Some(run_started_millis),
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
            self.release_admission_quota(run)?;
            self.discard_journal_sequence(run);
            return Err(error);
        }
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                if !self.run_state_contains(run) {
                    self.release_admission_quota(run)?;
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }
}
