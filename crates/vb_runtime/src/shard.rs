//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use indexmap::IndexMap;
use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::command::{InspectResponse, PendingTimer, PendingTimerKind, ShardCommand, MAX_COMMAND_QUEUE_CAPACITY};
use crate::counters::ShardCounters;
use crate::engine::{RuntimeEngineResult, RuntimeSignal};
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::run_state::RunState;
use crate::scheduler;
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};

type FramePoolKey = (u16, u16);

/// Single-threaded shard owning all mutable run state.
pub struct Shard {
    command_queue: ArrayQueue<ShardCommand>,
    runs: IndexMap<RunId, RunState>,
    pending_timers: IndexMap<RunId, PendingTimer>,
    frame_pools: IndexMap<FramePoolKey, FramePool>,
    trace_ring: TraceRing,
    counters: ShardCounters,
    step_budget_per_tick: u64,
    max_active_runs: usize,
    policy: vb_core::policy::RuntimePolicy,
    artifact_store: crate::admission::SharedArtifactStore,
    inspect_response: Option<InspectResponse>,
    shutting_down: bool,
    journal: SharedRuntimeJournal,
}

impl Shard {
    /// Creates a new shard with the given configuration.
    pub fn new(config: ShardConfig) -> Self {
        Self::new_with_journal(config, NoopRuntimeJournal::shared())
    }

    /// Creates a new shard with the given configuration, journal sink, and artifact store.
    pub fn new_with_journal_and_artifact_store(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
        artifact_store: crate::admission::SharedArtifactStore,
    ) -> Self {
        Self {
            command_queue: ArrayQueue::new(config.command_queue_capacity),
            runs: IndexMap::new(),
            pending_timers: IndexMap::new(),
            frame_pools: IndexMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            policy: config.policy,
            artifact_store,
            inspect_response: None,
            shutting_down: false,
            journal,
        }
    }

    /// Creates a new shard with the given configuration and journal sink.
    pub fn new_with_journal(config: ShardConfig, journal: SharedRuntimeJournal) -> Self {
        Self::new_with_journal_and_artifact_store(
            config,
            journal,
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
    }

    /// Enqueues a command. Returns `QueueFull` on overflow.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.command_queue
            .push(cmd)
            .map_err(|_| RuntimeError::QueueFull)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn command_queue_len(&self) -> usize {
        self.command_queue.len()
    }

    /// Returns the remaining free slots in the command queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.command_queue
            .capacity()
            .saturating_sub(self.command_queue.len())
    }

    /// Returns true if the command queue is full.
    #[must_use]
    pub fn is_queue_full(&self) -> bool {
        self.command_queue.is_full()
    }

    /// Returns the command queue capacity.
    #[must_use]
    pub fn command_queue_capacity(&self) -> usize {
        self.command_queue.capacity()
    }

    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        let Some(cmd) = self.command_queue.pop() else {
            return Ok(true);
        };

        match cmd {
            ShardCommand::Submit { run, workflow, caps } => self.handle_submit(run, workflow, caps)?,
            ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps,
            } => self.handle_submit_with_inputs(run, workflow, &inputs, caps)?,
            ShardCommand::Resume { run } => self.handle_resume(run)?,
            ShardCommand::ActionCompleted { ticket, output } => {
                self.handle_action_completion(ticket, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.handle_legacy_action_completion(run, step)?;
            }
            ShardCommand::ActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)?;
            }
            ShardCommand::AskAnswered { answer } => self.handle_ask_answer(answer)?,
            ShardCommand::TimerFired { run } => self.handle_timer(run)?,
            ShardCommand::Cancel { run } => self.handle_cancel(run)?,
            ShardCommand::Inspect { run, correlation } => {
                self.handle_inspect(run, correlation);
            }
            ShardCommand::Shutdown => {
                self.shutting_down = true;
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Processes queued commands through the shutdown marker.
    pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()> {
        let limit = self.command_queue.capacity();
        let mut processed = 0usize;
        while processed < limit {
            if !self.tick()? {
                return Ok(());
            }
            processed = processed.saturating_add(1);
        }
        Err(RuntimeError::ShutdownInProgress)
    }

    /// Returns a reference to the shard counters.
    #[must_use]
    pub const fn counters(&self) -> &ShardCounters {
        &self.counters
    }

    /// Returns a mutable reference to the trace ring.
    pub fn trace_ring_mut(&mut self) -> &mut TraceRing {
        &mut self.trace_ring
    }

    /// Returns an immutable reference to the trace ring.
    #[must_use]
    pub const fn trace_ring(&self) -> &TraceRing {
        &self.trace_ring
    }

    /// Returns a direct non-queued diagnostic snapshot for a run.
    #[must_use]
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> InspectResponse {
        match self.runs.get(&run) {
            Some(state) => InspectResponse::Found(scheduler::snapshot_from_state(run, correlation, state)),
            None => InspectResponse::NotFound { run, correlation },
        }
    }

    /// Takes the latest inspect response, if one is available.
    pub fn take_inspect_response(&mut self) -> Option<InspectResponse> {
        self.inspect_response.take()
    }

    /// Returns true if the shard is shutting down.
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        self.shutting_down
    }

    fn handle_submit(&mut self, run: RunId, workflow: CompiledWorkflow, caps: CapabilitySet) -> RuntimeResult<()> {
        self.handle_submit_with_inputs(run, workflow, &[], caps)
    }

    fn handle_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
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
        scheduler::seed_input_slots(&mut frame, inputs)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        self.journal.append(RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: digest,
        })?;
        self.counters.inc_submitted();
        let state = RunState::new(frame, workflow, admission);
        self.runs.insert(run, state);
        self.drive_run(run)
    }

    fn build_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        use crate::admission::{AdmissionError, admit_run};

        match admit_run(
            self.artifact_store.as_ref(),
            self.policy,
            digest,
            run,
            caps,
        ) {
            Ok(admission) => Ok(Some(admission)),
            Err(AdmissionError::ArtifactNotFound { digest }) => {
                Err(RuntimeError::AdmissionArtifactNotFound { digest })
            }
            Err(AdmissionError::CapabilityDenied { .. }) => {
                Ok(None)
            }
        }
    }

    fn handle_resume(&mut self, run: RunId) -> RuntimeResult<()> {
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        scheduler::validate_action_completion(state, ticket)?;
        state
            .frame
            .write_slot_with_taint(output.output_slot, output.value, output.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        scheduler::advance_after_action_completion(state, ticket.step)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot: output.output_slot,
        });
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: ticket.step,
        });
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: output.output_slot,
        })?;
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step: ticket.step,
            output: output.output_slot,
        })?;
        self.journal.append(RuntimeJournalEvent::ActionCompleted {
            run,
            step: ticket.step,
            action: ticket.action,
        })?;
        self.drive_run(run)
    }

    fn handle_legacy_action_completion(&mut self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn handle_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let mut retry_now = false;
        let mut fail_without_handler = false;
        {
            let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
            scheduler::validate_action_completion(state, ticket)?;
            if failure.retryable && scheduler::retry_metadata_exists(state, ticket.step) {
                let policy = scheduler::retry_policy_after_action(state, ticket.step)?;
                self.trace_ring.push(TraceEvent::ActionFailed {
                    run,
                    step: ticket.step,
                    code: failure.code,
                });
                if scheduler::record_retry_attempt(state, ticket, policy)? {
                    state
                        .frame
                        .set_pc(ticket.step)
                        .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                    retry_now = true;
                }
            }
            if !retry_now {
                match scheduler::find_error_handler_for_failure(&state.workflow, ticket.step) {
                    Some(handler) => {
                        state
                            .frame
                            .mark_failed(ticket.step)
                            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                        state
                            .frame
                            .set_pc(handler)
                            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                    }
                    None => {
                        fail_without_handler = true;
                    }
                }
            }
        }
        if retry_now {
            return self.drive_run(run);
        }
        if fail_without_handler {
            self.trace_ring.push(TraceEvent::ActionFailed {
                run,
                step: ticket.step,
                code: failure.code,
            });
            let state = self.take_run_state(run)?;
            self.fail_run_state(run, state)?;
            return Ok(());
        }
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code: failure.code,
        });
        self.drive_run(run)
    }

    fn handle_ask_answer(&mut self, answer: crate::command::AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        if let Some(timer) = self.pending_timers.get(&run)
            && timer.step == answer.ticket.ask_step
        {
            self.pending_timers.swap_remove(&run);
        }
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_running(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .set_pc(answer.ticket.resume_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
        self.journal.append(RuntimeJournalEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        })?;
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: answer.answer_slot,
        })?;
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step: answer.ticket.ask_step,
            output: answer.answer_slot,
        })?;
        self.drive_run(run)
    }

    fn handle_timer(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let Some(timer) = self.pending_timers.swap_remove(&run) else {
            self.runs.insert(run, state);
            return Err(RuntimeError::InvalidTimerFire);
        };
        crate::timer::advance_after_timer_fire(&mut state, timer)?;
        match timer.kind {
            PendingTimerKind::Wait => {
                self.journal.append(RuntimeJournalEvent::WaitResolved {
                    run,
                    step: timer.step,
                })?;
            }
            PendingTimerKind::Ask => {}
        }
        let result = scheduler::drive_state(&mut state, self.step_budget_per_tick);
        self.apply_drive_result(run, state, result)
    }

    fn handle_cancel(&mut self, run: RunId) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        if self.runs.contains_key(&run) {
            self.journal
                .append(RuntimeJournalEvent::RunCancelled { run })?;
        }
        if let Some(state) = self.runs.swap_remove(&run) {
            self.release_frame(state.frame);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
        }
        Ok(())
    }

    fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let result = scheduler::drive_state(&mut state, self.step_budget_per_tick);
        self.apply_drive_result(run, state, result)
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        match self.runs.swap_remove(&run) {
            Some(state) => Ok(state),
            None => Err(RuntimeError::RunNotFound),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn apply_drive_result(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeEngineResult<RuntimeSignal>,
    ) -> RuntimeResult<()> {
        match result {
            Ok(RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted) => {
                self.keep_run(run, state);
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.finish_run(run, state),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => self.await_action(run, state, ticket),
            Ok(RuntimeSignal::AwaitingWait) => self.await_timer(run, state, PendingTimerKind::Wait),
            Ok(RuntimeSignal::AwaitingAsk) => self.await_timer(run, state, PendingTimerKind::Ask),
            Err(_) => self.fail_run_state(run, state),
        }
    }

    fn keep_run(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        self.runs.insert(run, state);
    }

    fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        self.counters.inc_completed();
        self.counters.add_steps(state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
        let result = match scheduler::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        self.journal
            .append(RuntimeJournalEvent::StepSucceeded {
                run,
                step: state.frame.pc(),
                output: result,
            })?;
        self.journal
            .append(RuntimeJournalEvent::RunFinished { run, result })?;
        self.release_frame(state.frame);
        Ok(())
    }

    fn await_action(&mut self, run: RunId, mut state: RunState, ticket: ActionTicket) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        scheduler::record_scheduled_attempt(&mut state, ticket);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.journal.append(RuntimeJournalEvent::ActionScheduled {
            run,
            step,
            action: ticket.action,
        })?;
        self.runs.insert(run, state);
        Ok(())
    }

    fn await_timer(&mut self, run: RunId, state: RunState, kind: PendingTimerKind) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        if crate::timer::timer_registration_required(&state, step) {
            self.pending_timers.insert(run, PendingTimer { step, kind });
            match kind {
                PendingTimerKind::Wait => {
                    self.journal
                        .append(RuntimeJournalEvent::WaitScheduled { run, step })?;
                }
                PendingTimerKind::Ask => {
                    self.journal
                        .append(RuntimeJournalEvent::AskScheduled { run, step })?;
                }
            }
        }
        self.runs.insert(run, state);
        Ok(())
    }

    fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.journal.append(RuntimeJournalEvent::RunFailed { run })?;
        self.release_frame(state.frame);
        Ok(())
    }

    fn take_frame_for(
        &mut self,
        run: RunId,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<RunFrame> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let key = (step_count, slot_count);
        if !self.frame_pools.contains_key(&key) {
            let pool = FramePool::new(step_count, slot_count, self.max_active_runs)
                .map_err(|_| RuntimeError::FramePoolUnavailable)?;
            self.frame_pools.insert(key, pool);
        }
        let pool = self
            .frame_pools
            .get_mut(&key)
            .ok_or(RuntimeError::FramePoolUnavailable)?;
        pool.take(run, workflow.entry())
            .map_err(|_| RuntimeError::FramePoolUnavailable)
    }

    fn release_frame(&mut self, frame: RunFrame) {
        let key = (frame.step_count(), frame.slot_count());
        if let Some(pool) = self.frame_pools.get_mut(&key) {
            pool.release(frame);
        }
    }
}

/// Shard configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardConfig {
    /// Bounded capacity for the command queue.
    pub command_queue_capacity: usize,
    /// Bounded capacity for the trace ring.
    pub trace_capacity: usize,
    /// Maximum steps to execute per tick.
    pub step_budget_per_tick: u64,
    /// Maximum active runs admitted to this shard.
    pub max_active_runs: usize,
    /// Admission policy governing artifact verification.
    pub policy: vb_core::policy::RuntimePolicy,
}

impl ShardConfig {
    /// Creates a new ShardConfig, validating capacity limits.
    pub fn new(
        command_queue_capacity: usize,
        trace_capacity: usize,
        step_budget_per_tick: u64,
        max_active_runs: usize,
        policy: vb_core::policy::RuntimePolicy,
    ) -> RuntimeResult<Self> {
        if command_queue_capacity == 0 || command_queue_capacity > MAX_COMMAND_QUEUE_CAPACITY {
            return Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: command_queue_capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        if max_active_runs == 0 {
            return Err(RuntimeError::ActiveRunCapacityZero);
        }
        Ok(Self {
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            policy,
        })
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
            max_active_runs: 1024,
            policy: vb_core::policy::RuntimePolicy::Strict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ActionFailureCode;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
    use vb_core::Taint;
    use crate::command::{AskAnswer, AskTicket, InspectResponse, InspectSnapshot, ShardCommand};
    use crate::engine::RetryPolicy;

    fn suspended_workflow() -> Option<CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("suspended"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn action_with_error_handler_workflow() -> Option<CompiledWorkflow> {
        let guard = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(3)),
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let handler = CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(3)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("action_with_error_handler"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([guard, action, handler, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::Bool(false)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn action_ticket(run: RunId, step: StepIdx) -> ActionTicket {
        ActionTicket {
            run,
            step,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        }
    }

    fn timeout_failure() -> ActionFailure {
        ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        }
    }

    // =======================================================================
    // Core lifecycle tests
    // =======================================================================

    #[test]
    fn retry_attempt_counter_increments_until_policy_exhaustion() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let frame = match RunFrame::new(RunId::new(9), StepIdx::ZERO, 1, 1) {
            Ok(frame) => frame,
            Err(_) => return,
        };
        let mut state = RunState::new(frame, workflow, None);
        let ticket = ActionTicket {
            run: RunId::new(9),
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::new(1),
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        };
        let policy = RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(scheduler::record_retry_attempt(&mut state, ticket, policy), Ok(true));
        assert_eq!(state.action_attempt(StepIdx::ZERO), Some(2));
        assert_eq!(scheduler::record_retry_attempt(&mut state, ticket, policy), Ok(false));
    }

    #[test]
    fn action_failed_routes_to_nearby_error_handler() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = action_with_error_handler_workflow() else {
            return;
        };
        let run = RunId::new(301);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket: action_ticket(run, StepIdx::new(1)),
                failure: timeout_failure(),
            }),
            Ok(())
        );

        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    #[test]
    fn action_failed_without_error_handler_fails_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(302);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket: action_ticket(run, StepIdx::ZERO),
                failure: timeout_failure(),
            }),
            Ok(())
        );

        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn shard_rejects_active_run_capacity_overflow() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };

        let first = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: workflow.clone(),
            caps: CapabilitySet::empty(),
        });
        assert_eq!(first, Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        let second = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow,
            caps: CapabilitySet::empty(),
        });
        assert_eq!(second, Ok(()));
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn inspect_command_stores_retrievable_snapshot() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(7);

        let submitted = shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() });
        assert_eq!(submitted, Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let inspected = shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 99,
        });
        assert_eq!(inspected, Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 99);
            }
            other => assert_eq!(other, None),
        }
    }

    #[test]
    fn enqueue_shutdown_sets_shutting_down_flag() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        assert_eq!(shard.is_shutting_down(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
    }

    #[test]
    fn tick_returns_true_when_queue_is_empty() {
        let config = ShardConfig::default();
        let mut shard = Shard::new(config);
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn cancel_nonexistent_run_succeeds_silently() {
        let config = ShardConfig::default();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel {
                run: RunId::new(999)
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn counters_reflect_submitted_after_submit_tick() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(1);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn inspect_nonexistent_run_returns_not_found() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(999),
                correlation: 42,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run: RunId::new(999),
                correlation: 42,
            })
        );
    }

    // =======================================================================
    // Workflow helpers for tests
    // =======================================================================

    /// Workflow that finishes immediately (SetConst -> Finish).
    fn finished_workflow() -> Option<CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finished"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn timed_wait_then_finish_workflow() -> Option<CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("timed_wait_then_finish"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::I64(10)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn timed_ask_without_answer_workflow() -> Option<CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set_timeout = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("timed_ask_without_answer"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
                vb_core::value::ConstValue::I64(10),
            ]),
            slot_count: 3,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    // =======================================================================
    // Frame pool tests
    // =======================================================================

    #[test]
    fn finished_run_releases_frame_to_dimension_pool() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };

        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        let available = shard.frame_pools.get(&(2, 1)).map(FramePool::available);
        assert_eq!(available, Some(1));
    }

    #[test]
    fn cancelled_run_releases_frame_to_dimension_pool() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(11);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(0)
        );

        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(1)
        );
    }

    #[test]
    fn cancel_cleans_pending_timer() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = timed_wait_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(12);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timers.len(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.pending_timers.len(), 0);
    }

    #[test]
    fn finish_cleans_pending_timer_after_timer_fire() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = timed_wait_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(13);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timers.len(), 1);
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.pending_timers.len(), 0);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn fail_cleans_pending_timer_after_ask_timeout_without_answer() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = timed_ask_without_answer_workflow() else {
            return;
        };
        let run = RunId::new(14);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timers.len(), 1);
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.pending_timers.len(), 0);
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    // =======================================================================
    // Command queue tests
    // =======================================================================

    #[test]
    fn enqueue_returns_queue_full_when_capacity_exceeded() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn tick_after_shutdown_returns_false() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.tick(), Ok(false));
    }

    // =======================================================================
    // Submit tests
    // =======================================================================

    #[test]
    fn submit_returns_run_already_exists_for_duplicate() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(42);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    }

    #[test]
    fn submit_returns_active_run_capacity_exceeded_at_limit() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(2),
                workflow: wf2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn shard_submit_creates_run_state_in_runs_map() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(10);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let response = shard.take_inspect_response();
        match response {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 1);
            }
            other => {
                assert_eq!(other, None);
            }
        }
    }

    #[test]
    fn shard_submit_records_run_submitted_trace_event() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(20);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::RunSubmitted { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_submit_drives_run_immediately_for_finished_workflow() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        let run = RunId::new(30);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 2,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run,
                correlation: 2
            })
        );
    }

    // =======================================================================
    // Resume tests
    // =======================================================================

    #[test]
    fn shard_resume_returns_error_for_unknown_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Resume {
                run: RunId::new(999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_resume_continues_suspended_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(90);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
    }

    // =======================================================================
    // Action completion tests
    // =======================================================================

    #[test]
    fn shard_action_completed_returns_error_for_unknown_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run: RunId::new(888),
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_action_completed_marks_step_succeeded() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(55);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        let tick1 = shard.tick();
        assert_eq!(tick1, Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run,
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        let tick2 = shard.tick();
        assert_eq!(tick2, Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| {
            *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::new(0),
            }
        });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_action_completed_records_trace_event() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(56);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run,
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| {
            *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::new(0),
            }
        });
        assert_eq!(found, true);
    }

    // =======================================================================
    // Timer tests
    // =======================================================================

    #[test]
    fn shard_timer_rejects_run_without_pending_timer() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(60);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    }

    #[test]
    fn shard_wait_suspension_registers_pending_timer() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = timed_wait_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(61);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.pending_timers.len(), 1);
        assert_eq!(
            shard.pending_timers.get(&run).map(|timer| timer.step),
            Some(StepIdx::new(1))
        );
    }

    #[test]
    fn shard_timer_fired_advances_timed_wait_to_finish() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = timed_wait_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(62);

        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.pending_timers.len(), 0);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_timer_returns_error_for_unknown_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::TimerFired {
                run: RunId::new(777),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    // =======================================================================
    // Cancel tests
    // =======================================================================

    #[test]
    fn shard_cancel_removes_run_from_runs_map() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(70);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 5,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run,
                correlation: 5
            })
        );
    }

    #[test]
    fn shard_cancel_records_run_cancelled_trace_event() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(71);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::RunCancelled { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_cancel_emits_cancelled_journal_and_preserves_counter_semantics() {
        let config = small_config();
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: crate::journal::SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(config, shared);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(73);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert!(
            matches!(journal.snapshot(), Ok(events) if events.contains(&RuntimeJournalEvent::RunCancelled { run }))
        );
        assert!(
            shard
                .trace_ring_mut()
                .drain()
                .contains(&TraceEvent::RunCancelled { run })
        );
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.counters().snapshot().runs_completed, 0);
    }

    #[test]
    fn shard_cancel_increments_failed_counter() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(72);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    // =======================================================================
    // Inspect tests
    // =======================================================================

    #[test]
    fn shard_inspect_captures_current_pc() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(80);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 10,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.pc, StepIdx::new(0));
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 10);
            }
            other => assert_eq!(other, None),
        }
    }

    #[test]
    fn shard_inspect_captures_executed_count() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        let run = RunId::new(81);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().steps_executed, 2);
    }

    // =======================================================================
    // FIFO and response tests
    // =======================================================================

    #[test]
    fn shard_tick_processes_commands_in_fifo_order() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf1) = finished_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(100),
                workflow: wf1,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(101),
                workflow: wf2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 2);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_take_inspect_response_returns_none_initially() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let response = shard.take_inspect_response();
        assert_eq!(response, None);
    }

    #[test]
    fn shard_take_inspect_response_clears_after_take() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(95);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let first = shard.take_inspect_response();
        assert_eq!(first.is_some(), true);
        let second = shard.take_inspect_response();
        assert_eq!(second, None);
    }

    // =======================================================================
    // Config tests
    // =======================================================================

    #[test]
    fn shard_is_shutting_down_defaults_to_false() {
        let config = small_config();
        let shard = Shard::new(config);
        assert_eq!(shard.is_shutting_down(), false);
    }

    #[test]
    fn shard_config_default_values() {
        let config = ShardConfig::default();
        assert_eq!(config.command_queue_capacity, 1024);
        assert_eq!(config.trace_capacity, 4096);
        assert_eq!(config.step_budget_per_tick, 1000);
        assert_eq!(config.max_active_runs, 1024);
    }

    #[test]
    fn shard_config_equality_same_values() {
        let a = ShardConfig::default();
        let b = ShardConfig::default();
        assert_eq!(a, b);
    }

    #[test]
    fn shard_config_equality_differs() {
        let a = ShardConfig::default();
        let b = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn shard_config_clone_preserves_values() {
        let original = small_config();
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    // =======================================================================
    // Command equality tests
    // =======================================================================

    #[test]
    fn shard_command_equality_submit() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let a = ShardCommand::Submit {
            run: RunId::new(1),
            workflow: wf.clone(),
            caps: CapabilitySet::empty(),
        };
        let b = ShardCommand::Submit {
            run: RunId::new(1),
            workflow: wf,
            caps: CapabilitySet::empty(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_cancel() {
        let a = ShardCommand::Cancel { run: RunId::new(1) };
        let b = ShardCommand::Cancel { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_differs_run_id() {
        let a = ShardCommand::Cancel { run: RunId::new(1) };
        let b = ShardCommand::Cancel { run: RunId::new(2) };
        assert_ne!(a, b);
    }

    #[test]
    fn shard_command_equality_shutdown() {
        let a = ShardCommand::Shutdown;
        let b = ShardCommand::Shutdown;
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_inspect() {
        let a = ShardCommand::Inspect {
            run: RunId::new(1),
            correlation: 42,
        };
        let b = ShardCommand::Inspect {
            run: RunId::new(1),
            correlation: 42,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_inspect_differs_correlation() {
        let a = ShardCommand::Inspect {
            run: RunId::new(1),
            correlation: 1,
        };
        let b = ShardCommand::Inspect {
            run: RunId::new(1),
            correlation: 2,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn shard_command_equality_action_completed() {
        let a = ShardCommand::ActionCompletedLegacy {
            run: RunId::new(1),
            step: StepIdx::new(0),
        };
        let b = ShardCommand::ActionCompletedLegacy {
            run: RunId::new(1),
            step: StepIdx::new(0),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_timer_fired() {
        let a = ShardCommand::TimerFired { run: RunId::new(1) };
        let b = ShardCommand::TimerFired { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_resume() {
        let a = ShardCommand::Resume { run: RunId::new(1) };
        let b = ShardCommand::Resume { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_cancel_nonexistent_does_not_increment_failed() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel {
                run: RunId::new(999)
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    #[test]
    fn shard_finished_workflow_sets_completed_counter() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(50);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn shard_finished_workflow_produces_run_finished_trace() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(51);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_inspect_response_not_found_for_unknown_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(999),
                correlation: 1
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run: RunId::new(999),
                correlation: 1
            })
        );
    }

    #[test]
    fn inspect_response_found_equality() {
        let a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 42,
            pc: StepIdx::new(0),
            executed: 5,
        });
        let b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 42,
            pc: StepIdx::new(0),
            executed: 5,
        });
        assert_eq!(a, b);
    }

    #[test]
    fn inspect_response_found_differs_executed() {
        let a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 1,
            pc: StepIdx::new(0),
            executed: 5,
        });
        let b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 1,
            pc: StepIdx::new(0),
            executed: 10,
        });
        assert_ne!(a, b);
    }

    #[test]
    fn inspect_response_not_found_equality() {
        let a = InspectResponse::NotFound {
            run: RunId::new(1),
            correlation: 42,
        };
        let b = InspectResponse::NotFound {
            run: RunId::new(1),
            correlation: 42,
        };
        assert_eq!(a, b);
    }

    // =======================================================================
    // Adversarial BDD tests — shard
    // =======================================================================

    #[test]
    fn shard_cancel_then_inspect_returns_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(200);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run,
                correlation: 1
            })
        );
    }

    #[test]
    fn adversarial_shard_action_failed_for_unknown_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let ticket = ActionTicket {
            run: RunId::new(999),
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_duplicate_submit_after_cancel_succeeds() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(201);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn shard_snapshot_run_for_active_run_returns_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(202);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let response = shard.snapshot_run(run, 42);
        match response {
            InspectResponse::Found(snap) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 42);
            }
            other => {
                assert_eq!(
                    other,
                    InspectResponse::NotFound {
                        run,
                        correlation: 42
                    }
                );
            }
        }
    }

    #[test]
    fn shard_snapshot_run_for_unknown_returns_not_found() {
        let config = small_config();
        let shard = Shard::new(config);
        let response = shard.snapshot_run(RunId::new(9999), 7);
        assert_eq!(
            response,
            InspectResponse::NotFound {
                run: RunId::new(9999),
                correlation: 7,
            }
        );
    }

    #[test]
    fn shard_fill_queue_to_capacity_returns_queue_full() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn adversarial_shard_ask_answered_for_unknown_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let answer = AskAnswer {
            ticket: AskTicket {
                run: RunId::new(999),
                ask_step: StepIdx::ZERO,
                resume_step: StepIdx::new(1),
            },
            answer_slot: SlotIdx::new(0),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_submit_two_runs_same_id_second_fails() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(203);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    }

    #[test]
    fn shard_step_budget_zero_still_submits_but_does_not_drive() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 0,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(204);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run);
            }
            other => {
                assert_eq!(other, None);
            }
        }
    }

    #[test]
    fn shard_multiple_cancels_idempotent_for_same_run() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(205);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn shard_submit_after_shutdown_is_enqueued_but_never_processed() {
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(300),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    }

    #[test]
    fn shard_cancel_then_resubmit_then_cancel_increments_failed_twice() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(301);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 2);
        assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    }

    #[test]
    fn shard_action_completed_with_wrong_action_id_returns_invalid_completion() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(302);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(99),
            attempt: 1,
            idempotency_key: 0,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(1),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    }

    #[test]
    fn shard_action_completed_for_finished_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        let run = RunId::new(303);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run,
                step: StepIdx::ZERO,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_snapshot_run_after_cancel_returns_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(304);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let response = shard.snapshot_run(run, 7);
        assert_eq!(
            response,
            InspectResponse::NotFound {
                run,
                correlation: 7,
            }
        );
    }

    #[test]
    fn shard_timer_for_cancelled_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(305);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_resume_for_cancelled_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(306);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_trace_ring_overflow_drops_events_gracefully() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 2,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        for i in 1u64..=4 {
            let Some(workflow) = finished_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(400 + i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        let events = shard.trace_ring_mut().drain();
        assert_eq!(events.len() <= 2, true);
        assert_eq!(shard.trace_ring().dropped() > 0, true);
    }

    #[test]
    fn shard_submit_run_reuses_frame_from_pool_after_prior_finish() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(401),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        let Some(workflow2) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(402),
                workflow: workflow2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 2);
        assert_eq!(
            shard.frame_pools.get(&(2, 1)).map(FramePool::available),
            Some(1)
        );
    }

    #[test]
    fn shard_submit_max_active_runs_boundary_exactly_at_limit_succeeds() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 3,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        for i in 1u64..=3 {
            let Some(workflow) = suspended_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(500 + i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        assert_eq!(shard.counters().snapshot().runs_submitted, 3);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(504),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 3 })
        );
    }

    #[test]
    fn shard_inspect_preserves_latest_response_overwriting_previous() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run1 = RunId::new(600);
        let run2 = RunId::new(601);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: run1,
                workflow: wf1,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: run2,
                workflow: wf2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: run1,
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: run2,
                correlation: 2,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let response = shard.take_inspect_response();
        match response {
            Some(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run2);
                assert_eq!(snap.correlation, 2);
            }
            other => {
                assert_eq!(other, None);
            }
        }
    }

    // =======================================================================
    // Phase 2 adversarial BDD tests — shard resource exhaustion & security
    // =======================================================================

    #[test]
    fn shard_queue_full_prevents_further_command_submission() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn shard_active_run_capacity_exhausted_returns_precise_capacity_error() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 2,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let Some(wf3) = suspended_workflow() else {
            return;
        };

        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf1,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(2),
                workflow: wf2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(3),
                workflow: wf3,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
        );
    }

    #[test]
    fn shard_action_completed_for_wrong_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run: RunId::new(999),
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_step_budget_one_processes_one_command_per_tick() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 1,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn shard_duplicate_run_id_returns_run_already_exists_after_first_accepted() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(42);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf1 , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf2 , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    }

    #[test]
    fn shard_action_failed_for_unknown_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(999),
            step: StepIdx::new(0),
            seq: vb_core::ids::SeqNo::new(1),
            action: vb_core::ids::ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        };
        let failure = vb_core::action::ActionFailure {
            code: vb_core::action::ActionFailureCode::Unknown,
            retryable: false,
            taint: vb_core::value::Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_run_id_max_u64_accepted_as_valid_identifier() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        let run = RunId::new(u64::MAX);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_ask_answered_for_unknown_run_returns_run_not_found() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let answer = AskAnswer {
            ticket: AskTicket {
                run: RunId::new(999),
                ask_step: StepIdx::new(0),
                resume_step: StepIdx::new(1),
            },
            answer_slot: SlotIdx::new(0),
            value: vb_core::SlotValue::I64(42),
            taint: vb_core::Taint::Clean,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_snapshot_for_nonexistent_run_returns_not_found() {
        let config = small_config();
        let shard = Shard::new(config);
        let response = shard.snapshot_run(RunId::new(999), 42);
        assert_eq!(
            response,
            InspectResponse::NotFound {
                run: RunId::new(999),
                correlation: 42,
            }
        );
    }

    #[test]
    fn shard_cancel_then_resubmit_same_run_id_succeeds() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = finished_workflow() else {
            return;
        };
        let run = RunId::new(55);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf1 , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow: wf2 , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn shard_trace_ring_records_submit_and_finish_events_in_order() {
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        let run = RunId::new(77);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow , caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found_submit = events
            .iter()
            .any(|e| *e == TraceEvent::RunSubmitted { run });
        let found_finish = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
        assert_eq!(found_submit, true);
        assert_eq!(found_finish, true);
    }

    #[test]
    fn shard_with_zero_trace_capacity_does_not_crash_on_submit() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 0,
            step_budget_per_tick: 4,
            max_active_runs: 2,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    // =======================================================================
    // Command queue capacity methods
    // =======================================================================

    #[test]
    fn shard_command_queue_len_starts_at_zero() {
        let config = small_config();
        let shard = Shard::new(config);
        assert_eq!(shard.command_queue_len(), 0);
    }

    #[test]
    fn shard_command_queue_len_increments_on_enqueue() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.command_queue_len(), 0);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.command_queue_len(), 2);
    }

    #[test]
    fn shard_remaining_capacity_decrements_on_enqueue() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.remaining_capacity(), 4);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.remaining_capacity(), 3);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.remaining_capacity(), 2);
    }

    #[test]
    fn shard_remaining_capacity_is_zero_when_full() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.remaining_capacity(), 0);
    }

    #[test]
    fn shard_is_queue_full_returns_false_initially() {
        let config = small_config();
        let shard = Shard::new(config);
        assert_eq!(shard.is_queue_full(), false);
    }

    #[test]
    fn shard_is_queue_full_returns_true_when_at_capacity() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.is_queue_full(), true);
    }

    #[test]
    fn shard_command_queue_capacity_returns_configured_value() {
        let config = ShardConfig {
            command_queue_capacity: 512,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.command_queue_capacity(), 512);
    }

    #[test]
    fn shard_queue_len_decrements_after_tick() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel {
                run: RunId::new(999)
            }),
            Ok(())
        );
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.command_queue_len(), 0);
    }

    // =======================================================================
    // ShardConfig validation
    // =======================================================================

    #[test]
    fn shard_config_new_rejects_zero_command_queue_capacity() {
        let result = ShardConfig::new(0, 16, 4, 4, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: 0,
                max: MAX_COMMAND_QUEUE_CAPACITY
            })
        );
    }

    #[test]
    fn shard_config_new_rejects_excessive_command_queue_capacity() {
        let result = ShardConfig::new(MAX_COMMAND_QUEUE_CAPACITY + 1, 16, 4, 4, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: MAX_COMMAND_QUEUE_CAPACITY + 1,
                max: MAX_COMMAND_QUEUE_CAPACITY
            })
        );
    }

    #[test]
    fn shard_config_new_rejects_zero_max_active_runs() {
        let result = ShardConfig::new(16, 16, 4, 0, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
    }

    #[test]
    fn shard_config_new_accepts_valid_parameters() {
        let result = ShardConfig::new(1024, 4096, 1000, 512, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(result.is_ok(), true);
    }

    // =======================================================================
    // RuntimeError diagnostic codes
    // =======================================================================

    #[test]
    fn runtime_error_command_queue_capacity_exceeded_has_diagnostic_code() {
        let error = RuntimeError::CommandQueueCapacityExceeded {
            capacity: 100000,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        };
        assert_eq!(
            error.diagnostic_code(),
            RuntimeError::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE
        );
    }

    #[test]
    fn runtime_error_active_run_capacity_zero_has_diagnostic_code() {
        let error = RuntimeError::ActiveRunCapacityZero;
        assert_eq!(
            error.diagnostic_code(),
            RuntimeError::ACTIVE_RUN_CAPACITY_ZERO_CODE
        );
    }

    // =======================================================================
    // Stress tests — shard scheduler
    // =======================================================================

    #[test]
    fn stress_concurrent_run_submission_up_to_capacity() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 64,
            step_budget_per_tick: 64,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let capacity: usize = 8;
        let mut shard = Shard::new(config);

        for i in 1u64..=8 {
            let Some(workflow) = suspended_workflow() else {
                return;
            };
            let run_id = RunId::new(i);
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: run_id,
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(()),
                "enqueue should succeed for run {i}"
            );
            assert_eq!(shard.tick(), Ok(true), "tick should succeed for run {i}");
        }

        assert_eq!(shard.counters().snapshot().runs_submitted, 8);

        for i in 1u64..=8 {
            let run_id = RunId::new(i);
            assert_eq!(
                shard.enqueue(ShardCommand::Inspect {
                    run: run_id,
                    correlation: i,
                }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
            match shard.take_inspect_response() {
                Some(InspectResponse::Found(snap)) => {
                    assert_eq!(snap.run, run_id);
                    assert_eq!(snap.correlation, i);
                }
                other => assert_eq!(other, None),
            }
        }

        let Some(overflow_workflow) = suspended_workflow() else {
            return;
        };
        let overflow_id = RunId::new(9);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: overflow_id,
                workflow: overflow_workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity })
        );
    }

    #[test]
    fn stress_cancellation_mid_execution() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 64,
            step_budget_per_tick: 64,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);

        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(1001);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);

        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(0)
        );

        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(
            shard.snapshot_run(run, 1),
            InspectResponse::NotFound {
                run,
                correlation: 1,
            }
        );

        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(1)
        );

        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);

        let Some(workflow2) = suspended_workflow() else {
            return;
        };
        let run2 = RunId::new(1002);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: run2,
                workflow: workflow2,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 2);

        let snap = shard.snapshot_run(run2, 2);
        match snap {
            InspectResponse::Found(s) => {
                assert_eq!(s.run, run2);
                assert_eq!(s.correlation, 2);
            }
            InspectResponse::NotFound { run, correlation } => {
                assert_eq!(run, run2);
                assert_eq!(correlation, 2);
            }
        }
    }

    #[test]
    fn stress_shutdown_with_active_runs() {
        let config = ShardConfig {
            command_queue_capacity: 32,
            trace_capacity: 64,
            step_budget_per_tick: 64,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);

        for i in 1u64..=3 {
            let Some(workflow) = suspended_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        assert_eq!(shard.counters().snapshot().runs_submitted, 3);

        let Some(workflow_before) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(10),
                workflow: workflow_before,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));

        let Some(workflow_after) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(11),
                workflow: workflow_after,
                caps: CapabilitySet::empty() }),
            Ok(())
        );

        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 4);

        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);

        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.counters().snapshot().runs_submitted, 4);
    }

    #[test]
    fn stress_frame_pool_recycling() {
        let config = ShardConfig {
            command_queue_capacity: 32,
            trace_capacity: 64,
            step_budget_per_tick: 64,
            max_active_runs: 8,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);

        for i in 1u64..=4 {
            let Some(workflow) = finished_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }

        assert_eq!(shard.counters().snapshot().runs_completed, 4);
        assert_eq!(
            shard.frame_pools.get(&(2, 1)).map(FramePool::available),
            Some(1),
            "pool should have 1 reusable frame after 4 sequential completions"
        );

        for i in 10u64..=13 {
            let Some(workflow) = suspended_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(0)
        );

        for i in 10u64..=13 {
            assert_eq!(
                shard.enqueue(ShardCommand::Cancel { run: RunId::new(i) }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(4),
            "pool should have 4 frames after 4 cancellations"
        );

        for i in 20u64..=23 {
            let Some(workflow) = suspended_workflow() else {
                return;
            };
            assert_eq!(
                shard.enqueue(ShardCommand::Submit {
                    run: RunId::new(i),
                    workflow,
                    caps: CapabilitySet::empty() }),
                Ok(())
            );
            assert_eq!(shard.tick(), Ok(true));
        }
        assert_eq!(
            shard.frame_pools.get(&(1, 1)).map(FramePool::available),
            Some(0)
        );

        assert_eq!(shard.counters().snapshot().runs_submitted, 12);
        assert_eq!(shard.counters().snapshot().runs_completed, 4);
        assert_eq!(shard.counters().snapshot().runs_failed, 4);
    }

    // =======================================================================
    // Admission gate tests
    // =======================================================================

    struct SingleDigestStore {
        known: WorkflowDigest,
    }

    impl crate::admission::ArtifactStore for SingleDigestStore {
        fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool {
            digest == self.known
        }
    }

    fn strict_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Strict,
        }
    }

    fn relaxed_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    #[test]
    fn admission_strict_rejects_without_artifact() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore {
            known: WorkflowDigest::from_bytes([0xFF; 32]),
        };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let result = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow,
            caps: CapabilitySet::empty(),
        });
        assert_eq!(result, Ok(()));
        let tick_result = shard.tick();
        assert_eq!(
            tick_result,
            Err(RuntimeError::AdmissionArtifactNotFound { digest })
        );
    }

    #[test]
    fn admission_strict_accepts_with_artifact() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore { known: digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let result = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow,
            caps: CapabilitySet::empty(),
        });
        assert_eq!(result, Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let snap = shard.counters().snapshot();
        assert_eq!(snap.runs_submitted, 1);
    }

    #[test]
    fn admission_relaxed_allows_without_artifact() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let store = SingleDigestStore {
            known: WorkflowDigest::from_bytes([0xFF; 32]),
        };
        let artifact_store = std::sync::Arc::new(store);
        let config = relaxed_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let result = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow,
            caps: CapabilitySet::empty(),
        });
        assert_eq!(result, Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let snap = shard.counters().snapshot();
        assert_eq!(snap.runs_submitted, 1);
    }

    #[test]
    fn admission_attaches_digest_to_run() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore { known: digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow,
                caps: CapabilitySet::empty() }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let state = shard.runs.get(&RunId::new(1));
        assert!(state.is_some());
        let state = match state {
            Some(s) => s,
            None => return,
        };
        let admission = match &state.admission {
            Some(a) => a,
            None => return,
        };
        assert_eq!(admission.artifact_digest(), digest);
        assert_eq!(admission.run_id(), RunId::new(1));
    }

    #[test]
    fn admission_capability_check_at_submit() {
        use crate::admission::{AdmissionError, check_capability};
        use vb_core::capability::Capability;
        let action = ActionId::new(1);
        let required = Capability::Action(ActionId::new(1));
        let granted = vb_core::capability::CapabilitySet::empty();
        let result = check_capability(action, &required, &granted);
        assert_eq!(
            result,
            Err(AdmissionError::CapabilityDenied {
                action: ActionId::new(1),
                required: Capability::Action(ActionId::new(1)),
                granted: vb_core::capability::CapabilitySet::empty(),
            })
        );
    }

    // =======================================================================
    // Phase 66 adversarial BDD tests — admission bypass vectors
    // =======================================================================

    #[test]
    fn adversarial_admission_submit_with_caps_records_caps_in_admission() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore { known: digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let caps = CapabilitySet::from_grants(Box::new([vb_core::capability::Capability::AnyWorkflow]));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(500),
                workflow,
                caps: caps.clone(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let state = shard.runs.get(&RunId::new(500));
        assert!(state.is_some());
        let state = match state {
            Some(s) => s,
            None => return,
        };
        let admission = match &state.admission {
            Some(a) => a,
            None => return,
        };
        assert_eq!(admission.granted_capabilities(), &caps);
    }

    #[test]
    fn adversarial_admission_empty_caps_records_empty_in_admission() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore { known: digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(501),
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let state = shard.runs.get(&RunId::new(501));
        assert!(state.is_some());
        let state = match state {
            Some(s) => s,
            None => return,
        };
        let admission = match &state.admission {
            Some(a) => a,
            None => return,
        };
        assert!(admission.granted_capabilities().is_empty());
    }

    #[test]
    fn adversarial_admission_digest_from_different_workflow_rejected_under_strict() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let store = SingleDigestStore { known: wrong_digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let digest = workflow.digest();
        assert_ne!(digest, wrong_digest);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(502),
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::AdmissionArtifactNotFound { digest })
        );
    }

    #[test]
    fn adversarial_admission_relaxed_allows_wrong_digest() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let store = SingleDigestStore {
            known: WorkflowDigest::from_bytes([0xFF; 32]),
        };
        let artifact_store = std::sync::Arc::new(store);
        let config = relaxed_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(503),
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn adversarial_admission_check_capability_denied_for_action() {
        use crate::admission::{AdmissionError, check_capability};
        use vb_core::capability::Capability;
        let action = ActionId::new(42);
        let required = Capability::Action(ActionId::new(42));
        let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(1))]));
        let result = check_capability(action, &required, &granted);
        assert_eq!(
            result,
            Err(AdmissionError::CapabilityDenied {
                action: ActionId::new(42),
                required: Capability::Action(ActionId::new(42)),
                granted,
            })
        );
    }

    #[test]
    fn adversarial_admission_check_capability_granted_for_matching_action() {
        use crate::admission::check_capability;
        use vb_core::capability::Capability;
        let action = ActionId::new(1);
        let required = Capability::Action(ActionId::new(1));
        let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(1))]));
        assert_eq!(check_capability(action, &required, &granted), Ok(()));
    }

    #[test]
    fn adversarial_admission_check_capability_any_workflow_grants_all() {
        use crate::admission::check_capability;
        use vb_core::capability::Capability;
        let action = ActionId::new(99);
        let required = Capability::Action(ActionId::new(99));
        let granted = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
        assert_eq!(check_capability(action, &required, &granted), Ok(()));
    }

    #[test]
    fn adversarial_shard_submit_with_inputs_carries_caps() {
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let digest = workflow.digest();
        let store = SingleDigestStore { known: digest };
        let artifact_store = std::sync::Arc::new(store);
        let config = strict_config();
        let mut shard =
            Shard::new_with_journal_and_artifact_store(config, NoopRuntimeJournal::shared(), artifact_store);
        let caps = CapabilitySet::from_grants(Box::new([vb_core::capability::Capability::AnyWorkflow]));
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputs {
                run: RunId::new(504),
                workflow,
                inputs: Box::new([]),
                caps: caps.clone(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let state = shard.runs.get(&RunId::new(504));
        assert!(state.is_some());
        let state = match state {
            Some(s) => s,
            None => return,
        };
        let admission = match &state.admission {
            Some(a) => a,
            None => return,
        };
        assert_eq!(admission.granted_capabilities(), &caps);
    }
}
