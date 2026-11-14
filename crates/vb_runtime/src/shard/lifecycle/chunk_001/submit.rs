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
    /// Returns true when the journal is explicitly Volatile with no storage backend.
    /// Only that profile may skip retained-input admission for RunId-zero compatibility.
    fn is_volatile_admission(&self) -> bool {
        self.journal.durability_profile() == Some(vb_storage::DurabilityProfile::Volatile)
            && self.journal.storage_journal().is_none()
    }

    /// Builds the same-run admission header events without a retained input record.
    /// Used exclusively by the explicitly Volatile storage-free admission path.
    fn build_volatile_admission_events(
        &self,
        run: RunId,
        admission: &crate::admission::RunAdmission,
        digest: vb_core::WorkflowDigest,
        ir_compat_decision: Option<vb_storage::compat::CompatDecision>,
        tenant_admission: Option<RuntimeJournalEvent>,
        has_budget: bool,
        run_started_millis: u64,
    ) -> RuntimeResult<Vec<RuntimeJournalEvent>> {
        let mut events = vec![
            RuntimeJournalEvent::RunSubmitted { run, workflow: digest },
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
        if has_budget {
            events.push(RuntimeJournalEvent::RunBudgetOrigin {
                run,
                started_millis: run_started_millis,
            });
        }
        Ok(events)
    }

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
        let input_digest =
            vb_storage::inputs::compute_input_digest(inputs).map_err(RuntimeError::from)?;
        let run_started_millis = crate::shard::helpers::wall_clock_millis_since_epoch()
            .ok_or_else(|| RuntimeError::Core {
                source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "wall_clock_unavailable".into(),
                }),
            })?;
        let admission = self
            .build_admission_with_budget(run, &workflow, caps, options.lifetime_cap)?
            .map(|admission| {
                let admission =
                    admission.with_input_digest(vb_storage::RunInputBinding::Digest(input_digest));
                match options.tenant {
                    Some(tenant) => admission.with_tenant(tenant),
                    None => admission,
                }
            });
        execution_binding::validate(
            self.artifact_store.as_ref(),
            self.policy,
            &workflow,
            action_contracts,
        )?;
        self.prepare_run_slots(run)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        if let Err(error) = crate::shard::helpers::seed_input_slots(&mut frame, inputs, &workflow) {
            self.release_frame(frame);
            return Err(error);
        }
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
            if self.is_volatile_admission() {
                // Explicitly Volatile storage-free journal: use sequenced batch
                // directly. No retained inputs record constructed. RunId zero is valid.
                // Input validation already completed above; admission carries validated
                // input digest in RunInputBinding::Digest.
                let events = self.build_volatile_admission_events(
                    run,
                    admission,
                    digest,
                    ir_compat_decision,
                    tenant_admission,
                    admission.budget().is_some(),
                    run_started_millis,
                )?;
                let batch_result = self.append_journal_event_batch(&events);
                if let Err(error) = batch_result {
                    self.release_frame(frame);
                    self.release_admission_quota(run)?;
                    self.discard_journal_sequence(run);
                    return Err(error);
                }
            } else {
                // Durable/unknown profile: retained inputs required. RunId zero
                // rejected by storage key constructors (run_header_key).
                let retained_inputs = match vb_storage::RunInputsRecord::new(
                    run,
                    admission.artifact_digest(),
                    digest,
                    &workflow,
                    filled_inputs.into_boxed_slice(),
                ) {
                    Ok(record) => record,
                    Err(error) => {
                        self.release_frame(frame);
                        self.release_admission_quota(run)?;
                        return Err(RuntimeError::from(error));
                    }
                };
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
                header_bytes = match self.journal_batch_bytes(&events) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        self.release_frame(frame);
                        self.release_admission_quota(run)?;
                        return Err(error);
                    }
                };
                let header_result = self.append_admission_journal_event_batch(&events, retained_inputs);
                if let Err(error) = header_result {
                    self.release_frame(frame);
                    self.release_admission_quota(run)?;
                    self.discard_journal_sequence(run);
                    return Err(RuntimeError::admission_header_persistence_failed(error));
                }
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
