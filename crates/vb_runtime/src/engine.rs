#![forbid(unsafe_code)]

//! Enhanced engine with full iteration/compound primitive dispatch.

use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket,
    propagate_action_taint,
};
use vb_core::engine::{EngineSignal, StepBudget, step_once};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::primitives;

/// Result type for runtime engine operations.
pub type RuntimeEngineResult<T> = Result<T, RuntimeEngineError>;

/// Errors from the runtime engine's action-aware execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEngineError {
    /// Core engine error.
    Core(EngineError),
    /// Action subsystem error.
    Action(ActionError),
    /// Retry policy exhausted all attempts.
    RetryExhausted {
        /// Action that exhausted retries.
        action: ActionId,
        /// Number of attempts made.
        attempts: u16,
    },
    /// Taint propagation rejected a clean result from tainted input.
    TaintViolation {
        /// Step where the violation occurred.
        step: StepIdx,
    },
}

impl From<EngineError> for RuntimeEngineError {
    fn from(error: EngineError) -> Self {
        Self::Core(error)
    }
}

impl From<ActionError> for RuntimeEngineError {
    fn from(error: ActionError) -> Self {
        Self::Action(error)
    }
}

/// Retry policy for action invocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts before giving up.
    pub max_attempts: u16,
    /// Base delay between attempts in milliseconds.
    pub base_delay_ms: u64,
    /// Whether to use exponential backoff.
    pub exponential_backoff: bool,
}

impl RetryPolicy {
    /// Policy that never retries.
    pub const NEVER: Self = Self {
        max_attempts: 1,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    /// Policy with up to 3 attempts and no backoff.
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: false,
    };
}

/// Extended engine signal returned by the action-aware execution loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSignal {
    /// Deterministic execution can continue.
    Continue,
    /// Run finished with a result value.
    Finished(SlotValue),
    /// Step budget was exhausted before completion.
    StepBudgetExhausted,
    /// Run is awaiting action completion with the given ticket.
    AwaitingAction(ActionTicket),
    /// Run is awaiting a wait condition.
    AwaitingWait,
    /// Run is awaiting external input (ask).
    AwaitingAsk,
}

/// Executes one compiled node with full primitive dispatch.
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &vb_core::workflow::CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => primitives::for_each::for_each_start(
            run,
            store,
            *input,
            *item_slot,
            *limit,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => primitives::for_each::for_each_next(
            run,
            store,
            *iterator_slot,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ForEachJoin { output: _ } => {
            let step = node.id;
            match primitives::for_each::for_each_join(run, node.output, node.next, step) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            primitives::together::together_start(run, store, branches, *join, node.output)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => primitives::together::together_branch(
            run,
            store,
            *branch,
            *entry,
            *join,
            *accumulator,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            let step = node.id;
            match primitives::together::together_join(
                run,
                store,
                *branch_count,
                *accumulator,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => primitives::collect::collect_start(
            run,
            store,
            *source,
            *limit,
            *page_size,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => primitives::collect::collect_page(run, store, *collector_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => {
            let source = *collector_slot;
            let limit = 0;
            let page_size = 100;
            primitives::collect::collect_next(
                run,
                store,
                source,
                limit,
                page_size,
                *collector_slot,
                *body,
                *done,
            )
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core)
        }

        CompiledNodeKind::CollectFinish { collector_slot } => {
            let step = node.id;
            match primitives::collect::collect_finish(
                run,
                *collector_slot,
                node.output,
                node.next,
                step,
            ) {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => primitives::reduce::reduce_start(
            plan,
            run,
            store,
            *input,
            *accumulator,
            *initial,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => primitives::reduce::reduce_next(
            run,
            store,
            *iterator_slot,
            *accumulator,
            *body,
            *done,
            node.output,
        )
        .map_err(RuntimeEngineError::Core)
        .map(runtime_from_core),

        CompiledNodeKind::ReduceFinish { accumulator } => {
            let step = node.id;
            match primitives::reduce::reduce_finish(run, *accumulator, node.output, node.next, step)
            {
                Ok(signal) => Ok(runtime_from_core(signal)),
                Err(e) => Err(RuntimeEngineError::Core(e)),
            }
        }

        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => primitives::repeat::repeat_start(run, *max_attempts, *body, *done, node.output)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => primitives::repeat::repeat_attempt(run, *attempt_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            primitives::repeat::repeat_check(run, *attempt_slot, *done, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::RepeatFinish { result } => {
            primitives::repeat::repeat_finish(run, *result, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitUntil { deadline_slot } => {
            primitives::wait_ask::wait_until(run, *deadline_slot)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => primitives::wait_ask::wait_event(run, *event, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => primitives::wait_ask::ask(run, *prompt, *timeout_slot)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

        CompiledNodeKind::AskResume { answer } => {
            primitives::wait_ask::ask_resume(run, *answer, node.output, node.next, node.id)
                .map_err(RuntimeEngineError::Core)
                .map(runtime_from_core)
        }

        CompiledNodeKind::Do { action, input } => {
            let seq = SeqNo::new(run.executed());
            execute_do(
                run,
                node.id,
                *action,
                *input,
                seq,
                resolve_contract(*action, contracts)?,
                contracts,
            )
        }

        CompiledNodeKind::RetryCheck {
            policy_slot: _,
            body,
            exhausted,
        } => {
            let target = execute_retry_check(1, retry_policy, *body, *exhausted);
            run.set_pc(target);
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        CompiledNodeKind::ErrorHandler {
            body: handler_body,
            handler: _,
        } => {
            run.set_pc(*handler_body);
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        _ => {
            let core_signal = step_once(plan, run, store).map_err(RuntimeEngineError::Core)?;
            Ok(runtime_from_core(core_signal))
        }
    }
}

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
pub fn drive_deterministic_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    loop {
        if !budget.try_take().map_err(RuntimeEngineError::Core)? {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        }

        let pc = run.pc();
        let node = plan
            .node(pc)
            .ok_or(EngineError::InvalidProgramCounter { step: pc })?;

        run.mark_running(pc).map_err(RuntimeEngineError::Core)?;

        let signal = match execute_node_full(plan, run, store, node, contracts, retry_policy) {
            Ok(signal) => signal,
            Err(error) => {
                run.mark_failed(pc).map_err(RuntimeEngineError::Core)?;
                return Err(error);
            }
        };

        match mark_step_after_signal(run, pc, &signal) {
            Ok(()) => {}
            Err(e) => return Err(RuntimeEngineError::Core(e)),
        }

        match signal {
            RuntimeSignal::Continue => continue,
            other => return Ok(other),
        }
    }
}

/// Backward-compatible drive loop matching the original drive_with_actions signature.
pub fn drive_with_actions(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let mut store = ValueStore::new();
    drive_deterministic_full(plan, run, budget, &mut store, contracts, retry_policy)
}

/// Backward-compatible execute_do.
pub fn execute_do(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    _contract: &ActionContract,
    registry_contracts: &[ActionContract],
) -> RuntimeEngineResult<RuntimeSignal> {
    let action_index = usize::from(action.get());
    let resolved = registry_contracts
        .get(action_index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })?;

    let input_taint = run.read_taint(input).map_err(RuntimeEngineError::Core)?;
    let output_taint = propagate_action_taint(resolved.idempotency, input_taint);

    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };

    if output_taint == Taint::Clean && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    Ok(RuntimeSignal::AwaitingAction(ticket))
}

/// Backward-compatible execute_retry_check.
pub fn execute_retry_check(
    current_attempt: u16,
    policy: RetryPolicy,
    body: StepIdx,
    exhausted: StepIdx,
) -> StepIdx {
    if current_attempt < policy.max_attempts {
        body
    } else {
        exhausted
    }
}

/// Backward-compatible execute_error_handler.
pub fn execute_error_handler(failure: &ActionFailure, handler: StepIdx, body: StepIdx) -> StepIdx {
    if failure.retryable || failure.code != ActionFailureCode::Unknown {
        handler
    } else {
        body
    }
}

/// Resumes an action outcome into the run frame.
pub fn resume_action_outcome(
    run: &mut RunFrame,
    outcome: &ActionOutcome,
) -> RuntimeEngineResult<RuntimeSignal> {
    match outcome {
        ActionOutcome::Ready(ready) => {
            run.write_slot_with_taint(ready.output_slot, ready.value, ready.taint)
                .map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        ActionOutcome::Suspended(ticket) => Ok(RuntimeSignal::AwaitingAction(*ticket)),
        ActionOutcome::Failed(failure) => {
            let step = run.pc();
            if failure.retryable {
                Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                    run: run.run_id(),
                    step,
                    seq: SeqNo::new(0),
                    action: ActionId::new(0),
                    attempt: 1,
                    idempotency_key: 0,
                }))
            } else {
                Err(RuntimeEngineError::Core(
                    EngineError::UnsupportedPrimitive {
                        primitive: "action_failed_non_retryable",
                    },
                ))
            }
        }
    }
}

fn runtime_from_core(signal: EngineSignal) -> RuntimeSignal {
    match signal {
        EngineSignal::Continue => RuntimeSignal::Continue,
        EngineSignal::Finished(value) => RuntimeSignal::Finished(value),
        EngineSignal::StepBudgetExhausted => RuntimeSignal::StepBudgetExhausted,
        EngineSignal::AwaitingAction => RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::ZERO,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        }),
        EngineSignal::AwaitingWait => RuntimeSignal::AwaitingWait,
        EngineSignal::AwaitingAsk => RuntimeSignal::AwaitingAsk,
    }
}

fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &RuntimeSignal,
) -> Result<(), EngineError> {
    match signal {
        RuntimeSignal::AwaitingWait => run.mark_waiting(step),
        RuntimeSignal::AwaitingAsk => run.mark_asking(step),
        RuntimeSignal::AwaitingAction(_) => Ok(()),
        RuntimeSignal::Continue | RuntimeSignal::Finished(_) => run.mark_succeeded(step),
        RuntimeSignal::StepBudgetExhausted => Ok(()),
    }
}

fn resolve_contract(
    action: ActionId,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<&ActionContract> {
    let index = usize::from(action.get());
    contracts
        .get(index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })
        .map_err(RuntimeEngineError::Action)
}

fn compute_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.as_u64());
    let seq_part = u128::from(seq.as_u64()) << 64;
    let action_part = u128::from(u32::from(action.get())) << 80;
    match run_part.checked_add(seq_part) {
        Some(combined) => match combined.checked_add(action_part) {
            Some(key) => key,
            None => run_part,
        },
        None => run_part,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::Idempotency;
    use vb_core::engine::EngineSignal;
    use vb_core::errors::EngineError;
    use vb_core::value::SlotValue;

    #[test]
    fn retry_check_routes_to_body_when_attempts_remain() {
        let policy = RetryPolicy::DEFAULT;
        let target = execute_retry_check(1, policy, StepIdx::new(5), StepIdx::new(10));
        assert_eq!(target, StepIdx::new(5));
    }

    #[test]
    fn retry_check_routes_to_exhausted_when_attempts_spent() {
        let policy = RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
        assert_eq!(target, StepIdx::new(10));
    }

    #[test]
    fn never_retry_always_exhausts_after_one() {
        let policy = RetryPolicy::NEVER;
        let target = execute_retry_check(1, policy, StepIdx::new(3), StepIdx::new(7));
        assert_eq!(target, StepIdx::new(7));
    }

    #[test]
    fn error_handler_routes_to_handler_on_retryable_failure() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
        assert_eq!(target, StepIdx::new(5));
    }

    #[test]
    fn error_handler_routes_to_body_on_non_retryable_unknown() {
        let failure = ActionFailure {
            code: ActionFailureCode::Unknown,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
        assert_eq!(target, StepIdx::new(3));
    }

    #[test]
    fn compute_idempotency_key_is_deterministic() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
        assert_eq!(key1, key2);
    }

    #[test]
    fn compute_idempotency_key_differs_for_different_runs() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(2), SeqNo::new(0), ActionId::new(0));
        assert_ne!(key1, key2);
    }

    #[test]
    fn runtime_signal_from_core_maps_all_variants() {
        assert_eq!(
            runtime_from_core(EngineSignal::Continue),
            RuntimeSignal::Continue
        );
        assert_eq!(
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42))),
            RuntimeSignal::Finished(SlotValue::I64(42))
        );
        assert_eq!(
            runtime_from_core(EngineSignal::StepBudgetExhausted),
            RuntimeSignal::StepBudgetExhausted
        );
        assert_eq!(
            runtime_from_core(EngineSignal::AwaitingAction),
            RuntimeSignal::AwaitingAction(ActionTicket {
                run: RunId::ZERO,
                step: StepIdx::ZERO,
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
                attempt: 1,
                idempotency_key: 0,
            })
        );
        assert_eq!(
            runtime_from_core(EngineSignal::AwaitingWait),
            RuntimeSignal::AwaitingWait
        );
        assert_eq!(
            runtime_from_core(EngineSignal::AwaitingAsk),
            RuntimeSignal::AwaitingAsk
        );
    }

    #[test]
    fn execute_do_returns_awaiting_action_for_known_action() {
        let run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
        };
        let registry_contracts: Vec<ActionContract> = vec![
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(0),
            contract_ref,
            &registry_contracts,
        );
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(1));
                assert_eq!(ticket.run, RunId::new(1));
                assert_eq!(ticket.step, StepIdx::new(0));
            }
            other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
        }
    }

    #[test]
    fn execute_do_propagates_taint_from_secret_input_without_violation() {
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let write_result = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
        assert_eq!(write_result.map(|_| ()), Ok(()));
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
        };
        let registry_contracts: Vec<ActionContract> = vec![
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(0),
            contract_ref,
            &registry_contracts,
        );
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(1));
                assert_eq!(ticket.run, RunId::new(1));
            }
            other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
        }
    }

    #[test]
    fn execute_do_returns_unknown_action_for_unregistered_action() {
        let run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let empty_contracts: Vec<ActionContract> = Vec::new();
        let dummy_contract = ActionContract {
            id: ActionId::new(0),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
        };
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(5),
            SlotIdx::new(0),
            SeqNo::new(0),
            &dummy_contract,
            &empty_contracts,
        );
        assert_eq!(
            result,
            Err(RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(5),
            }))
        );
    }

    #[test]
    fn resume_action_outcome_ready_continues_execution() {
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ready = vb_core::action::ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        let outcome = ActionOutcome::Ready(ready);
        let result = resume_action_outcome(&mut run, &outcome);
        assert_eq!(result, Ok(RuntimeSignal::Continue));
    }

    #[test]
    fn resume_action_outcome_failed_non_retryable_returns_error() {
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Unknown,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let outcome = ActionOutcome::Failed(failure);
        let result = resume_action_outcome(&mut run, &outcome);
        assert_eq!(
            result,
            Err(RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
                primitive: "action_failed_non_retryable",
            }))
        );
    }

    #[test]
    fn resume_action_outcome_suspended_returns_awaiting() {
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(5),
            action: ActionId::new(3),
            attempt: 2,
            idempotency_key: 99,
        };
        let outcome = ActionOutcome::Suspended(ticket);
        let result = resume_action_outcome(&mut run, &outcome);
        assert_eq!(
            result,
            Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(5),
                action: ActionId::new(3),
                attempt: 2,
                idempotency_key: 99,
            }))
        );
    }

    #[test]
    fn retry_policy_never_has_max_attempts_one() {
        assert_eq!(RetryPolicy::NEVER.max_attempts, 1);
    }

    #[test]
    fn retry_policy_default_has_max_attempts_three() {
        assert_eq!(RetryPolicy::DEFAULT.max_attempts, 3);
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use vb_core::engine::StepBudget;

        proptest! {
            #[test]
            fn step_budget_never_allows_more_than_n_steps(n in 1u64..1000u64) {
                let mut budget = StepBudget::new(n);
                let mut taken = 0u64;
                let mut drained = false;
                while !drained && taken <= n + 1 {
                    match budget.try_take() {
                        Ok(true) => taken += 1,
                        Ok(false) => drained = true,
                        Err(_) => drained = true,
                    }
                }
                prop_assert_eq!(taken, n);
            }
        }

        proptest! {
            #[test]
            fn idempotency_key_differs_for_different_tuples(
                run1 in 1u64..100u64,
                run2 in 1u64..100u64,
                seq1 in 0u64..100u64,
                seq2 in 0u64..100u64,
                action1 in 0u16..10u16,
                action2 in 0u16..10u16,
            ) {
                prop_assume!(run1 != run2 || seq1 != seq2 || action1 != action2);
                let key1 = super::compute_idempotency_key(RunId::new(run1), SeqNo::new(seq1), ActionId::new(action1));
                let key2 = super::compute_idempotency_key(RunId::new(run2), SeqNo::new(seq2), ActionId::new(action2));
                prop_assert_ne!(key1, key2);
            }
        }
    }
}
