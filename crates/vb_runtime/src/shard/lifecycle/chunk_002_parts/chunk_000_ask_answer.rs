#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AskAnswerTimeoutAuthority {
    NotRequired,
    Required,
}

struct AskAnswerPlan {
    run: RunId,
    timeout_authority: AskAnswerTimeoutAuthority,
    encoded_answer_value: Vec<u8>,
}

impl Shard {
    /// Handles an ask answer for a suspended run.
    ///
    /// # Flux refinement (PO-vb282my-AA-FLUX-001):
    /// Atomic journal-before-mutation ordering guarantee:
    /// SlotWritten, AskAnswered, and StepSucceeded are appended as one
    /// same-run journal batch. The shard advances the per-run sequence only
    /// after the batch returns Ok, so a failed answer append cannot leave a
    /// partial durable prefix or mutate the live run frame / pending timer.
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, answer: AskAnswer) -> RuntimeResult<()>
    ///     ensures result.is_err() || journal_has(SlotWritten{run, slot})
    /// )]
    /// ```
    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let plan = self.ask_answer_plan(answer)?;
        let run = plan.run;
        let timeout_authority = plan.timeout_authority;
        self.append_ask_answer_journal(run, answer, plan.encoded_answer_value)?;
        self.apply_ask_answer_to_frame(run, answer)?;
        self.clear_answered_ask_timer(run, timeout_authority);
        self.record_ask_answer_trace(run, answer);
        self.drive_run(run)
    }

    fn ask_answer_plan(&self, answer: AskAnswer) -> RuntimeResult<AskAnswerPlan> {
        let run = answer.ticket.run;
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let timeout_authority = self.ask_answer_timeout_authority(state, answer)?;
        let encoded_answer_value = Self::encode_ask_answer_value(answer)?;
        Ok(AskAnswerPlan {
            run,
            timeout_authority,
            encoded_answer_value,
        })
    }

    fn encode_ask_answer_value(answer: AskAnswer) -> RuntimeResult<Vec<u8>> {
        postcard::to_allocvec(&answer.value).map_err(|_| RuntimeError::EncodeFailed)
    }

    fn append_ask_answer_journal(
        &mut self,
        run: RunId,
        answer: AskAnswer,
        encoded_answer_value: Vec<u8>,
    ) -> RuntimeResult<()> {
        self.append_journal_events_atomically([
            RuntimeJournalEvent::SlotWritten {
                run,
                slot: answer.answer_slot,
                value: encoded_answer_value,
                taint: answer.taint,
                extra: None,
            },
            RuntimeJournalEvent::AskAnswered {
                run,
                step: answer.ticket.ask_step,
                slot: answer.answer_slot,
            },
            RuntimeJournalEvent::StepSucceeded {
                run,
                step: answer.ticket.ask_step,
                output: answer.answer_slot,
                attempt: 1,
            },
        ])
    }

    fn apply_ask_answer_to_frame(&mut self, run: RunId, answer: AskAnswer) -> RuntimeResult<()> {
        let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .set_pc(answer.ticket.resume_step)
            .map_err(|_| RuntimeError::RunNotFound)
    }

    fn clear_answered_ask_timer(&mut self, run: RunId, authority: AskAnswerTimeoutAuthority) {
        if authority == AskAnswerTimeoutAuthority::Required {
            let _removed_timer = self.pending_timer_remove(run);
        }
    }

    fn record_ask_answer_trace(&mut self, run: RunId, answer: AskAnswer) {
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
    }

    fn ask_answer_timeout_authority(
        &self,
        state: &RunState,
        answer: AskAnswer,
    ) -> RuntimeResult<AskAnswerTimeoutAuthority> {
        self.validate_ask_answer_payload(state, answer)?;
        let node = self.validate_ask_answer_resume_shape(state, answer)?;
        match node.kind {
            CompiledNodeKind::Ask {
                timeout_slot: Some(_),
                ..
            } => self.require_ask_timer_authority(answer),
            CompiledNodeKind::Ask {
                timeout_slot: None, ..
            } => Ok(AskAnswerTimeoutAuthority::NotRequired),
            _ => Err(RuntimeError::InvalidActionCompletion),
        }
    }

    fn validate_ask_answer_payload(
        &self,
        state: &RunState,
        answer: AskAnswer,
    ) -> RuntimeResult<()> {
        let contract = state.workflow.resource_contract();
        if answer.taint == Taint::Secret && !contract.result_taint_policy {
            return Err(RuntimeError::SecretResultNotAllowed);
        }
        if answer.encoded_len > contract.max_ipc_payload_bytes {
            return Err(RuntimeError::IpcPayloadSizeExceeded {
                size: answer.encoded_len,
                max: contract.max_ipc_payload_bytes,
            });
        }
        if answer.answer_slot.as_usize() >= usize::from(state.frame.slot_count())
            || answer.ticket.resume_step.as_usize() >= usize::from(state.frame.step_count())
        {
            return Err(RuntimeError::RunNotFound);
        }
        Ok(())
    }

    fn validate_ask_answer_resume_shape(
        &self,
        state: &RunState,
        answer: AskAnswer,
    ) -> RuntimeResult<vb_core::workflow::CompiledNode> {
        if state.frame.step_state(answer.ticket.ask_step)
            != Ok(vb_core::frame::StepState::Asking)
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let Some(node) = state.workflow.node(answer.ticket.ask_step).cloned() else {
            return Err(RuntimeError::InvalidActionCompletion);
        };
        if node.next != Some(answer.ticket.resume_step) {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        match state.workflow.node(answer.ticket.resume_step).map(|resume| &resume.kind) {
            Some(CompiledNodeKind::AskResume { answer: slot }) if *slot == answer.answer_slot => {
                Ok(node)
            }
            _ => Err(RuntimeError::InvalidActionCompletion),
        }
    }

    fn require_ask_timer_authority(
        &self,
        answer: AskAnswer,
    ) -> RuntimeResult<AskAnswerTimeoutAuthority> {
        let pending_timer = self
            .pending_timer_get(answer.ticket.run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if pending_timer.step == answer.ticket.ask_step && pending_timer.kind == PendingTimerKind::Ask
        {
            Ok(AskAnswerTimeoutAuthority::Required)
        } else {
            Err(RuntimeError::InvalidActionCompletion)
        }
    }
}
