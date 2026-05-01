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
        } => primitives::collect::collect_next(run, store, *collector_slot, *body, *done)
            .map_err(RuntimeEngineError::Core)
            .map(runtime_from_core),

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
            if contracts.is_empty() {
                execute_do_without_contract(run, node.id, *action, *input, seq)
            } else {
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

fn execute_do_without_contract(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    _input: SlotIdx,
    seq: SeqNo,
) -> RuntimeEngineResult<RuntimeSignal> {
    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };
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
}
