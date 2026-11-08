// Operator-facing manual gate submission flow (MASTER §77.15, V1-MIN).
//
// Pure evaluation lives in [`crate::manual_gate`]; the durable journal
// append port lives in `impl_parts/gate_decision.rs`. This module is the
// imperative shell joining them: open a gate and suspend its run, submit
// an operator decision and journal the outcome, or escalate a timed-out
// gate. Outcomes are applied only after the decision record is durably
// journaled, so a journal failure leaves the gate open and the run
// suspended. A decision is final once journaled: the gate is removed
// before the run is resumed or failed, and a later resume failure does
// not reopen it; the operator retries with a plain resume.
//
// Open gates live in shard memory only. The durable record of a gate is
// its outcome record; a gate that is opened but never decided leaves no
// journal trace, and a crash loses it. A durable gate-open event with
// recovery reconstruction is a follow-up batch, not part of this flow.
//
// A gated run cannot be driven by any other command while its gate is
// open: [`Shard::handle_resume`] rejects gated runs, and a gated run
// holds no pending action, timer, or ask authority, so no completion
// command can drive it either.

use vb_core::manual_gate::{
    ApproverId, GateDecision, GateDecisionInput, GateId, GateOutcomeRecord, ManualGate,
};

use crate::manual_gate::{evaluate_decision, evaluate_timeout};
use crate::shard::types::OpenGate;

impl Shard {
    /// Opens a manual gate for a run and suspends the run until decided.
    ///
    /// The run must exist and be suspendable, the gate id must not
    /// already be open, and the shard-wide open-gate registry must have
    /// room. The run is suspended through the awaiting-action transition,
    /// so the drive loop leaves it alone until a decision resumes or
    /// fails it. The admission budget's `max_open_gates` is not enforced
    /// here: the verifier derives it from reachable Ask nodes only, so
    /// it is zero for Ask-less workflows and cannot bound
    /// operator-opened gates. The registry cap below is the bound; a
    /// distinct operator gate budget is a policy decision for John.
    pub(crate) fn open_gate(
        &mut self,
        gate: ManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
    ) -> RuntimeResult<()> {
        let run = gate.run;
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.open_gates.contains_key(&gate.id) {
            return Err(RuntimeError::GateAlreadyOpen { gate: gate.id });
        }
        self.check_gate_suspendable(run)?;
        self.reserve_open_gate_slot()?;
        self.apply(run, RuntimeEvent::AwaitAction)?;
        let open = OpenGate {
            gate,
            opened_at_ms,
            requester,
            delegated_to: None,
        };
        self.open_gates.insert(open.gate.id, open);
        Ok(())
    }

    /// Submits an operator decision to an open manual gate and applies
    /// the outcome.
    ///
    /// The decision is evaluated against the open gate, then journaled
    /// through the strict gate port before any state changes. Approvals
    /// resume the run, denials fail it, more-evidence requests leave the
    /// gate open, and delegations leave the gate open with the delegate
    /// recorded as the only approver for later submissions.
    pub(crate) fn submit_gate_decision(
        &mut self,
        input: GateDecisionInput,
    ) -> RuntimeResult<GateOutcomeRecord> {
        let open = self
            .open_gates
            .get(&input.gate)
            .ok_or(RuntimeError::GateNotOpen { gate: input.gate })?
            .clone();
        let run = open.gate.run;
        if !self.run_state_contains(run) {
            self.open_gates.swap_remove(&input.gate);
            return Err(RuntimeError::RunNotFound);
        }
        if let Some(delegate) = &open.delegated_to
            && input.approver != *delegate
        {
            return Err(RuntimeError::GateWrongDelegate {
                gate: input.gate,
                expected: delegate.clone(),
                actual: input.approver.clone(),
            });
        }
        let record = evaluate_decision(&open.gate, open.opened_at_ms, &input)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        self.append_gate_decision_strict(record.clone())?;
        match input.decision {
            GateDecision::Approve | GateDecision::BreakGlass => {
                self.open_gates.swap_remove(&input.gate);
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            GateDecision::Deny => {
                self.open_gates.swap_remove(&input.gate);
                self.fail_gated_run(run)?;
            }
            GateDecision::RequestMoreEvidence => {}
            GateDecision::Delegate => {
                let delegated_to = record.delegated_to.clone();
                if let Some(open) = self.open_gates.get_mut(&input.gate) {
                    open.delegated_to = delegated_to;
                }
            }
        }
        Ok(record)
    }

    /// Escalates an open manual gate whose deadline has passed.
    ///
    /// The escalation record is journaled through the strict gate port,
    /// the gate is removed, and the run is failed: an unanswered gate
    /// must not let the run proceed.
    pub(crate) fn escalate_timed_out_gate(
        &mut self,
        gate: GateId,
        now_ms: u64,
    ) -> RuntimeResult<GateOutcomeRecord> {
        let open = self
            .open_gates
            .get(&gate)
            .ok_or(RuntimeError::GateNotOpen { gate })?
            .clone();
        let run = open.gate.run;
        if !self.run_state_contains(run) {
            self.open_gates.swap_remove(&gate);
            return Err(RuntimeError::RunNotFound);
        }
        let record = evaluate_timeout(&open.gate, &open.requester, open.opened_at_ms, now_ms)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        self.append_gate_decision_strict(record.clone())?;
        self.open_gates.swap_remove(&gate);
        self.fail_gated_run(run)?;
        Ok(record)
    }

    /// Returns the first open gate id for a run, in opening order, if any.
    pub(crate) fn open_gate_for(&self, run: RunId) -> Option<GateId> {
        self.open_gates
            .values()
            .find(|open| open.gate.run == run)
            .map(|open| open.gate.id)
    }

    /// Rejects opening a gate on a run that cannot be suspended.
    ///
    /// Only a run that can proceed may be gated: initial or running, with
    /// no pending action, timer, or ask authority. A run already suspended
    /// must resolve its existing suspension first, so a gate can never
    /// hide behind or bypass another suspension.
    fn check_gate_suspendable(&self, run: RunId) -> RuntimeResult<()> {
        let state = self.get_runtime_state_or_running(run);
        if !matches!(state, RuntimeState::Initial | RuntimeState::Running) {
            return Err(RuntimeError::GateRunNotSuspendable { run, state });
        }
        if self.pending_action_get(run).is_some()
            || self.pending_timer_get(run).is_some()
            || self.pending_ask_authorities.contains_key(&run)
        {
            return Err(RuntimeError::GateRunNotSuspendable { run, state });
        }
        Ok(())
    }

    /// Reserves one slot in the shard-wide open-gate registry.
    ///
    /// Total open gates are capped at the shard's active-run capacity:
    /// every open gate belongs to an active run, and the registry must
    /// stay bounded even for runs admitted without a budget.
    fn reserve_open_gate_slot(&mut self) -> RuntimeResult<()> {
        if self.open_gates.len() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        self.open_gates
            .try_reserve(1)
            .map_err(|_| RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            })
    }

    /// Fails a gated run after its decision record is journaled.
    ///
    /// Mirrors the empty-evidence drive failure path: journal `RunFailed`,
    /// capture the retry checkpoint, and clear the runtime state. When
    /// the failure event itself would exceed the journal budget, the
    /// single-event fallback fails the run instead.
    fn fail_gated_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let state = self.take_run_state(run)?;
        match self.fail_run_state(run, state) {
            Ok(TransitionOutcome::Applied) => {}
            Ok(TransitionOutcome::JournalBudgetExceeded(state)) => {
                return self
                    .fail_run_on_journal_budget(run, *state, Vec::new())
                    .map_err(|failure| match failure {
                        DriveApplyFailure::BeforeCommit(error)
                        | DriveApplyFailure::AfterCommit(error) => error,
                    });
            }
            Err(error) => return Err(error),
        }
        self.apply(run, RuntimeEvent::Fail)
    }
}
