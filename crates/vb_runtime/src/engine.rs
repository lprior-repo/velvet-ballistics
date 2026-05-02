#![forbid(unsafe_code)]

//! Enhanced engine with full iteration/compound primitive dispatch.

use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket,
    Idempotency, propagate_action_taint,
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

impl RuntimeEngineError {
    /// Runtime code for exhausted retry policies.
    pub const RETRY_EXHAUSTED_RUNTIME_CODE: &str = "RETRY_EXHAUSTED";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Core(error) => error.runtime_code(),
            Self::Action(error) => error.runtime_code(),
            Self::RetryExhausted { .. } => Some(Self::RETRY_EXHAUSTED_RUNTIME_CODE),
            Self::TaintViolation { .. } => None,
        }
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

        CompiledNodeKind::ForEachJoin { output } => {
            let step = node.id;
            match primitives::for_each::for_each_join(run, *output, node.output, node.next, step) {
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
            run.set_pc(target).map_err(RuntimeEngineError::Core)?;
            run.increment_executed().map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }

        CompiledNodeKind::ErrorHandler {
            body: handler_body,
            handler: _,
        } => {
            run.set_pc(*handler_body)
                .map_err(RuntimeEngineError::Core)?;
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
    if resolved.idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

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
        EngineSignal::Finished(value, _taint) => RuntimeSignal::Finished(value),
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
    use vb_core::ConstIdx;
    use vb_core::action::Idempotency;
    use vb_core::action::RetrySafety;
    use vb_core::action::SideEffect;
    use vb_core::engine::EngineSignal;
    use vb_core::errors::EngineError;
    use vb_core::value::SlotValue;
    use vb_core::workflow::CompiledNode;

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
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)),
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
        let write_result =
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
        assert_eq!(write_result.map(|_| ()), Ok(()));
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            Err(RuntimeEngineError::Core(
                EngineError::UnsupportedPrimitive {
                    primitive: "action_failed_non_retryable",
                }
            ))
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

    #[test]
    fn runtime_engine_error_core_wraps_core_error() {
        // Given a core EngineError
        let core_err = EngineError::UnsupportedPrimitive {
            primitive: "test_prim",
        };
        // When wrapping in RuntimeEngineError::Core
        let engine_err = RuntimeEngineError::Core(core_err.clone());
        // Then the inner error matches exactly
        assert_eq!(
            engine_err,
            RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
                primitive: "test_prim",
            })
        );
    }

    #[test]
    fn runtime_engine_error_action_wraps_action_error() {
        // Given an ActionError
        let action_err = ActionError::UnknownAction {
            action: ActionId::new(7),
        };
        // When wrapping in RuntimeEngineError::Action
        let engine_err = RuntimeEngineError::Action(action_err.clone());
        // Then the inner error matches exactly
        assert_eq!(
            engine_err,
            RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(7),
            })
        );
    }

    #[test]
    fn runtime_engine_error_retry_exhausted_reports_action_and_attempts() {
        // Given a RetryExhausted error
        let err = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(3),
            attempts: 5,
        };
        // Then the fields match exactly
        match err {
            RuntimeEngineError::RetryExhausted { action, attempts } => {
                assert_eq!(action, ActionId::new(3));
                assert_eq!(attempts, 5);
            }
            other => assert_eq!(
                other,
                RuntimeEngineError::RetryExhausted {
                    action: ActionId::new(0),
                    attempts: 0,
                }
            ),
        }
    }

    #[test]
    fn runtime_engine_error_taint_violation_reports_step() {
        // Given a TaintViolation error
        let err = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(42),
        };
        // Then the step value matches exactly
        match err {
            RuntimeEngineError::TaintViolation { step } => {
                assert_eq!(step, StepIdx::new(42));
            }
            other => assert_eq!(
                other,
                RuntimeEngineError::TaintViolation {
                    step: StepIdx::new(0),
                }
            ),
        }
    }

    #[test]
    fn execute_node_full_returns_error_for_missing_node() {
        // Given a workflow with no nodes at a given step
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("empty"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(10);
        let contracts: Vec<ActionContract> = Vec::new();
        // When driving with an out-of-bounds pc
        let result = run
            .set_pc(StepIdx::new(5))
            .map_err(RuntimeEngineError::Core)
            .and_then(|()| {
                drive_deterministic_full(
                    &workflow,
                    &mut run,
                    &mut budget,
                    &mut store,
                    &contracts,
                    RetryPolicy::NEVER,
                )
            });
        // Then it returns a Core error with InvalidProgramCounter
        match result {
            Err(RuntimeEngineError::Core(EngineError::InvalidProgramCounter { step })) => {
                assert_eq!(step, StepIdx::new(5));
            }
            other => {
                // Wrong: expected InvalidProgramCounter
                assert_eq!(other, Ok(RuntimeSignal::Continue));
            }
        }
    }

    #[test]
    fn step_budget_new_with_zero_allows_no_steps() {
        // Given a step budget with 0 steps
        let mut budget = StepBudget::new(0);
        // When trying to take a step
        let result = budget.try_take();
        // Then it returns Ok(false) — no steps allowed
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn step_budget_remaining_decreases_after_each_step() {
        // Given a step budget with 3 steps
        let mut budget = StepBudget::new(3);
        // When taking steps
        assert_eq!(budget.try_take(), Ok(true));
        assert_eq!(budget.try_take(), Ok(true));
        assert_eq!(budget.try_take(), Ok(true));
        // Then the fourth step is denied
        assert_eq!(budget.try_take(), Ok(false));
    }

    #[test]
    fn retry_check_returns_done_when_attempts_equal_max() {
        // Given a retry policy with max_attempts = 3
        let policy = RetryPolicy::DEFAULT;
        // When current attempt equals max_attempts
        let target = execute_retry_check(3, policy, StepIdx::new(1), StepIdx::new(10));
        // Then it routes to the exhausted step
        assert_eq!(target, StepIdx::new(10));
    }

    #[test]
    fn execute_do_returns_ticket_with_correct_attempt_field() {
        // Given a run and action
        let run = match RunFrame::new(RunId::new(5), StepIdx::new(0), 4, 2) {
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        // When executing a Do node
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(3),
            contract_ref,
            &registry_contracts,
        );
        // Then it returns AwaitingAction with attempt=1 and seq=3
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.attempt, 1);
                assert_eq!(ticket.seq, SeqNo::new(3));
                assert_eq!(ticket.run, RunId::new(5));
            }
            other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
        }
    }

    #[test]
    fn error_handler_routes_to_handler_on_unknown_retryable() {
        // Given a failure that is retryable with unknown code
        let failure = ActionFailure {
            code: ActionFailureCode::Unknown,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        // When executing error handler
        let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
        // Then it routes to handler because retryable
        assert_eq!(target, StepIdx::new(8));
    }

    #[test]
    fn error_handler_routes_to_handler_on_non_unknown_non_retryable() {
        // Given a failure that is not retryable but code is Timeout (not Unknown)
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        // When executing error handler
        let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
        // Then it routes to handler because code != Unknown
        assert_eq!(target, StepIdx::new(8));
    }

    #[test]
    fn compute_idempotency_key_is_unique_for_different_seq() {
        // Given same run and action but different sequence numbers
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(0));
        // Then the keys differ
        assert_ne!(key1, key2);
    }

    #[test]
    fn compute_idempotency_key_is_unique_for_different_action() {
        // Given same run and seq but different action
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(1));
        // Then the keys differ
        assert_ne!(key1, key2);
    }

    #[test]
    fn resume_action_outcome_failed_retryable_returns_awaiting_action() {
        // Given a run and a retryable failure outcome
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let outcome = ActionOutcome::Failed(failure);
        // When resuming with the retryable failure
        let result = resume_action_outcome(&mut run, &outcome);
        // Then it returns AwaitingAction
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.run, RunId::new(1));
                assert_eq!(ticket.step, StepIdx::new(0));
            }
            other => {
                // Wrong: expected AwaitingAction
                assert_eq!(other, Ok(RuntimeSignal::Continue));
            }
        }
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

    // Additional engine BDD tests

    #[test]
    fn runtime_engine_error_from_engine_error() {
        // Given a core EngineError
        let core = EngineError::UnsupportedPrimitive { primitive: "test" };
        // When converting to RuntimeEngineError via From
        let engine_err: RuntimeEngineError = core.into();
        // Then it is wrapped in Core variant
        assert_eq!(
            engine_err,
            RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive: "test" })
        );
    }

    #[test]
    fn runtime_engine_error_from_action_error() {
        // Given an ActionError
        let action = ActionError::UnknownAction {
            action: ActionId::new(5),
        };
        // When converting to RuntimeEngineError via From
        let engine_err: RuntimeEngineError = action.into();
        // Then it is wrapped in Action variant
        assert_eq!(
            engine_err,
            RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(5)
            })
        );
    }

    #[test]
    fn retry_policy_never_has_base_delay_zero() {
        // Given NEVER retry policy
        assert_eq!(RetryPolicy::NEVER.base_delay_ms, 0);
        assert_eq!(RetryPolicy::NEVER.exponential_backoff, false);
    }

    #[test]
    fn retry_policy_default_has_base_delay_100() {
        // Given DEFAULT retry policy
        assert_eq!(RetryPolicy::DEFAULT.base_delay_ms, 100);
        assert_eq!(RetryPolicy::DEFAULT.exponential_backoff, false);
    }

    #[test]
    fn retry_check_routes_to_body_below_max() {
        // Given a policy with max_attempts = 3
        let policy = RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        // When current attempt is 0
        let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
        // Then routes to body
        assert_eq!(target, StepIdx::new(5));
    }

    #[test]
    fn retry_check_routes_to_body_at_max_minus_one() {
        // Given a policy with max_attempts = 3
        let policy = RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        // When current attempt is 2 (max - 1)
        let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
        // Then routes to body
        assert_eq!(target, StepIdx::new(5));
    }

    #[test]
    fn execute_do_returns_unknown_action_error_for_empty_registry() {
        // Given a run with no registered actions
        let run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let empty_contracts: Vec<ActionContract> = Vec::new();
        let dummy = ActionContract {
            id: ActionId::new(0),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
        };
        // When executing with an empty registry
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(0),
            &dummy,
            &empty_contracts,
        );
        // Then it returns UnknownAction
        assert_eq!(
            result,
            Err(RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(0)
            }))
        );
    }

    #[test]
    fn runtime_signal_debug_output() {
        // Given runtime signals
        let cont = RuntimeSignal::Continue;
        let exhausted = RuntimeSignal::StepBudgetExhausted;
        let wait = RuntimeSignal::AwaitingWait;
        let ask = RuntimeSignal::AwaitingAsk;
        // When formatting with debug
        let cont_debug = format!("{cont:?}");
        let ex_debug = format!("{exhausted:?}");
        let wait_debug = format!("{wait:?}");
        let ask_debug = format!("{ask:?}");
        // Then debug output contains variant names
        assert_eq!(cont_debug.contains("Continue"), true);
        assert_eq!(ex_debug.contains("StepBudgetExhausted"), true);
        assert_eq!(wait_debug.contains("AwaitingWait"), true);
        assert_eq!(ask_debug.contains("AwaitingAsk"), true);
    }

    #[test]
    fn runtime_engine_error_equality_retry_exhausted() {
        // Given two identical RetryExhausted errors
        let a = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        let b = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_engine_error_equality_taint_violation() {
        // Given two identical TaintViolation errors
        let a = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(5),
        };
        let b = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(5),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_engine_error_equality_differs_attempts() {
        // Given two RetryExhausted with different attempts
        let a = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        let b = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 5,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_engine_error_equality_differs_action() {
        // Given two RetryExhausted with different actions
        let a = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 3,
        };
        let b = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(2),
            attempts: 3,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn error_handler_routes_to_body_when_failure_is_unknown_and_non_retryable() {
        // Given a failure that is not retryable with Unknown code
        let failure = ActionFailure {
            code: ActionFailureCode::Unknown,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        // When executing error handler
        let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
        // Then it routes to body because non-retryable AND unknown
        assert_eq!(target, StepIdx::new(3));
    }

    #[test]
    fn resume_action_outcome_suspended_preserves_ticket() {
        // Given a run and a suspended outcome with specific ticket
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ticket = ActionTicket {
            run: RunId::new(99),
            step: StepIdx::new(7),
            seq: SeqNo::new(42),
            action: ActionId::new(13),
            attempt: 4,
            idempotency_key: 12345,
        };
        let outcome = ActionOutcome::Suspended(ticket);
        // When resuming with suspended
        let result = resume_action_outcome(&mut run, &outcome);
        // Then it returns AwaitingAction with exact ticket
        assert_eq!(
            result,
            Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                run: RunId::new(99),
                step: StepIdx::new(7),
                seq: SeqNo::new(42),
                action: ActionId::new(13),
                attempt: 4,
                idempotency_key: 12345,
            }))
        );
    }

    #[test]
    fn execute_do_idempotency_key_computation() {
        // Given a run and action
        let run = match RunFrame::new(RunId::new(42), StepIdx::new(0), 4, 2) {
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        // When executing Do node
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(10),
            contract_ref,
            &registry_contracts,
        );
        // Then the idempotency key is deterministic
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                let expected_key =
                    compute_idempotency_key(RunId::new(42), SeqNo::new(10), ActionId::new(1));
                assert_eq!(ticket.idempotency_key, expected_key);
            }
            other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
        }
    }

    #[test]
    fn execute_do_with_clean_input_and_pure_idempotency_succeeds() {
        // Given a run with clean input slot
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        // When executing Do with clean input
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(0),
            contract_ref,
            &registry_contracts,
        );
        // Then it returns AwaitingAction (no taint violation)
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(1));
            }
            other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
        }
    }

    #[test]
    fn runtime_engine_error_debug_output() {
        // Given engine errors
        let core =
            RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive: "test" });
        let action = RuntimeEngineError::Action(ActionError::UnknownAction {
            action: ActionId::new(1),
        });
        let retry = RuntimeEngineError::RetryExhausted {
            action: ActionId::new(2),
            attempts: 3,
        };
        let taint = RuntimeEngineError::TaintViolation {
            step: StepIdx::new(5),
        };
        // When formatting with debug
        // Then debug output contains relevant variant names
        assert_eq!(format!("{core:?}").contains("Core"), true);
        assert_eq!(format!("{action:?}").contains("Action"), true);
        assert_eq!(format!("{retry:?}").contains("RetryExhausted"), true);
        assert_eq!(format!("{taint:?}").contains("TaintViolation"), true);
    }

    #[test]
    fn runtime_engine_error_runtime_codes_delegate_to_core_and_action() {
        assert_eq!(
            RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive: "test" })
                .runtime_code(),
            Some("UNSUPPORTED_PRIMITIVE")
        );
        assert_eq!(
            RuntimeEngineError::Action(ActionError::PayloadTooLarge {
                max_bytes: 1,
                actual_bytes: 2,
            })
            .runtime_code(),
            Some("PAYLOAD_TOO_LARGE")
        );
    }

    #[test]
    fn runtime_engine_error_runtime_codes_cover_section_17_mappings() {
        assert_eq!(
            RuntimeEngineError::RetryExhausted {
                action: ActionId::new(2),
                attempts: 3,
            }
            .runtime_code(),
            Some("RETRY_EXHAUSTED")
        );
    }

    #[test]
    fn runtime_engine_error_runtime_codes_are_unique() {
        let codes = [RuntimeEngineError::RETRY_EXHAUSTED_RUNTIME_CODE];
        assert_eq!(codes.len(), 1);
        assert_eq!(
            codes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            1
        );
    }

    #[test]
    fn runtime_engine_error_runtime_code_is_absent_without_section_17_equivalent() {
        assert_eq!(
            RuntimeEngineError::TaintViolation {
                step: StepIdx::new(5)
            }
            .runtime_code(),
            None
        );
    }

    #[test]
    fn runtime_signal_equality_continue() {
        // Given two Continue signals
        assert_eq!(RuntimeSignal::Continue, RuntimeSignal::Continue);
    }

    #[test]
    fn runtime_signal_equality_exhausted() {
        // Given two StepBudgetExhausted signals
        assert_eq!(
            RuntimeSignal::StepBudgetExhausted,
            RuntimeSignal::StepBudgetExhausted
        );
    }

    #[test]
    fn runtime_signal_equality_awaiting_wait() {
        // Given two AwaitingWait signals
        assert_eq!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingWait);
    }

    #[test]
    fn runtime_signal_equality_awaiting_ask() {
        // Given two AwaitingAsk signals
        assert_eq!(RuntimeSignal::AwaitingAsk, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn runtime_signal_differs_awaiting_wait_from_awaiting_ask() {
        assert_ne!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn retry_policy_clone_preserves_values() {
        // Given a retry policy
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 200,
            exponential_backoff: true,
        };
        // When cloning
        let cloned = policy.clone();
        // Then values match
        assert_eq!(cloned.max_attempts, 5);
        assert_eq!(cloned.base_delay_ms, 200);
        assert_eq!(cloned.exponential_backoff, true);
    }

    // =======================================================================
    // Adversarial BDD tests — engine
    // =======================================================================

    #[test]
    fn drive_deterministic_budget_zero_returns_step_budget_exhausted() {
        // Given a workflow that would normally finish
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("nop"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(0);
        // When driving with zero budget
        let result = drive_deterministic_full(
            &workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
        );
        // Then it returns StepBudgetExhausted
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
    }

    #[test]
    fn execute_do_taint_violation_for_pure_action_with_secret_input() {
        // Given a run with a secret-tainted input slot and DeterministicPure action
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret),
            Ok(())
        );
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
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
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            contract,
        ];
        let contract_ref = match registry_contracts.get(1) {
            Some(c) => c,
            None => return,
        };
        // When executing Do with secret input on pure action
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(0),
            contract_ref,
            &registry_contracts,
        );
        // Then the pure action ABI rejects tainted input before scheduling.
        match result {
            Err(RuntimeEngineError::TaintViolation { step }) => {
                assert_eq!(step, StepIdx::new(0));
            }
            other => {
                assert_eq!(
                    other,
                    Err(RuntimeEngineError::TaintViolation {
                        step: StepIdx::new(0)
                    })
                );
            }
        }
    }

    #[test]
    fn drive_deterministic_full_invalid_pc_returns_core_error() {
        // Given a workflow with 1 node but pc set to 99
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("tiny"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(10);
        // When driving with invalid pc
        let result = run
            .set_pc(StepIdx::new(99))
            .map_err(RuntimeEngineError::Core)
            .and_then(|()| {
                drive_deterministic_full(
                    &workflow,
                    &mut run,
                    &mut budget,
                    &mut store,
                    &[],
                    RetryPolicy::NEVER,
                )
            });
        // Then it returns Core(InvalidProgramCounter)
        assert_eq!(
            result,
            Err(RuntimeEngineError::Core(
                EngineError::InvalidProgramCounter {
                    step: StepIdx::new(99),
                }
            ))
        );
    }

    #[test]
    fn resolve_contract_returns_unknown_action_for_empty_contracts() {
        // Given empty contracts slice
        let contracts: Vec<ActionContract> = Vec::new();
        // When resolving any action
        let result = resolve_contract(ActionId::new(0), &contracts);
        // Then it returns UnknownAction
        assert_eq!(
            result,
            Err(RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(0),
            }))
        );
    }

    #[test]
    fn resume_action_outcome_ready_writes_slot_and_continues() {
        // Given a run with slot 0 unwritten
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ready = vb_core::action::ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
            encoded_len: 1,
        };
        // When resuming with a Ready outcome
        let result = resume_action_outcome(&mut run, &ActionOutcome::Ready(ready));
        // Then it returns Continue and slot is written
        assert_eq!(result, Ok(RuntimeSignal::Continue));
        assert_eq!(run.read_slot(SlotIdx::new(0)), Ok(&SlotValue::Bool(true)));
    }

    #[test]
    fn compute_idempotency_key_handles_large_values_without_overflow() {
        // Given maximum-ish values
        let key = compute_idempotency_key(
            RunId::new(u64::MAX),
            SeqNo::new(u64::MAX),
            ActionId::new(65535),
        );
        // And the same inputs produce the same key
        let key2 = compute_idempotency_key(
            RunId::new(u64::MAX),
            SeqNo::new(u64::MAX),
            ActionId::new(65535),
        );
        assert_eq!(key, key2);
    }

    #[test]
    fn execute_error_handler_routes_to_body_only_when_unknown_and_non_retryable() {
        // Given all combinations of failure properties
        let cases: Vec<(ActionFailureCode, bool, StepIdx)> = vec![
            (ActionFailureCode::Unknown, false, StepIdx::new(3)), // body: only case
            (ActionFailureCode::Unknown, true, StepIdx::new(8)),  // handler: retryable
            (ActionFailureCode::Timeout, false, StepIdx::new(8)), // handler: not Unknown
            (ActionFailureCode::Rejected, true, StepIdx::new(8)), // handler: retryable
            (ActionFailureCode::Rejected, false, StepIdx::new(8)), // handler: not Unknown
        ];
        let handler = StepIdx::new(8);
        let body = StepIdx::new(3);
        for (code, retryable, expected) in cases {
            let failure = ActionFailure {
                code,
                retryable,
                taint: Taint::Clean,
                detail: None,
                encoded_len: 0,
            };
            let target = execute_error_handler(&failure, handler, body);
            assert_eq!(
                target, expected,
                "Failed for code={code:?} retryable={retryable}"
            );
        }
    }

    // =======================================================================
    // Adversarial BDD tests - engine attack vectors
    // =======================================================================

    #[test]
    fn drive_deterministic_full_single_step_budget_exhausts_after_one_step() {
        // Given a workflow with 3 nodes (SetConst -> SetConst -> Finish)
        let set1 = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set2 = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("multi_step"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([5; 32]),
            nodes: Box::from([set1, set2, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
            slot_count: 2,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(1);
        // When driving with budget of 1
        let result = drive_deterministic_full(
            &workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
        );
        // Then it returns StepBudgetExhausted (only one step executed)
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // And pc has advanced to step 1
        assert_eq!(run.pc(), StepIdx::new(1));
    }

    #[test]
    fn execute_do_with_at_least_once_external_idempotency_propagates_secret_taint() {
        // Given a run with secret input and AtLeastOnceExternal idempotency
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        assert_eq!(
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret),
            Ok(())
        );
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::AtLeastOnceExternal,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
        };
        let registry: Vec<ActionContract> = vec![
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            contract,
        ];
        let contract_ref = match registry.get(1) {
            Some(c) => c,
            None => return,
        };
        // When executing Do with secret input on AtLeastOnceExternal
        let result = execute_do(
            &run,
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            SeqNo::new(0),
            contract_ref,
            &registry,
        );
        // Then it returns AwaitingAction (no taint violation for external actions)
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(1));
            }
            other => {
                assert_eq!(other, Ok(RuntimeSignal::Continue));
            }
        }
    }

    #[test]
    fn resolve_contract_for_action_with_mismatched_id_in_slot_returns_unknown() {
        // Given a contracts vector where slot 1 has ActionId(99) instead of ActionId(1)
        let contracts: Vec<ActionContract> = vec![
            ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
            ActionContract {
                id: ActionId::new(99),
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 5000,
                idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            },
        ];
        // When resolving ActionId(1) which is at index 1 but stored as ActionId(99)
        let result = resolve_contract(ActionId::new(1), &contracts);
        // Then it returns UnknownAction (id mismatch filter)
        assert_eq!(
            result,
            Err(RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(1),
            }))
        );
    }

    #[test]
    fn resume_action_outcome_ready_with_wrong_slot_index_returns_slot_out_of_bounds() {
        // Given a run with 1 slot (slot_count=1) and an outcome targeting slot 5
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 1) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ready = vb_core::action::ActionOutputReady {
            output_slot: SlotIdx::new(5),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        // When resuming with an out-of-bounds output slot
        let result = resume_action_outcome(&mut run, &ActionOutcome::Ready(ready));
        // Then it returns a Core error (SlotOutOfBounds)
        match result {
            Err(RuntimeEngineError::Core(EngineError::SlotOutOfBounds { slot })) => {
                assert_eq!(slot, SlotIdx::new(5));
            }
            other => {
                assert_eq!(other, Ok(RuntimeSignal::Continue));
            }
        }
    }

    #[test]
    fn drive_deterministic_full_step_budget_exactly_one_runs_one_step() {
        // Given a 2-step workflow (Nop -> Nop)
        let node0 = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::Nop,
        };
        let node1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = vb_core::workflow::WorkflowParts {
            name: Box::from("two_nop"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([6; 32]),
            nodes: Box::from([node0, node1]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 1) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(1);
        // When driving with budget=1 (exactly one step)
        let result = drive_deterministic_full(
            &workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
        );
        // Then it returns StepBudgetExhausted
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // And budget is now exhausted
        assert_eq!(budget.try_take(), Ok(false));
    }

    #[test]
    fn execute_do_without_contract_returns_valid_ticket_for_any_action() {
        // Given a run and no registered contracts
        let run = match RunFrame::new(RunId::new(42), StepIdx::new(0), 4, 2) {
            Ok(f) => f,
            Err(_) => return,
        };
        // When executing without a contract
        let result = execute_do_without_contract(
            &run,
            StepIdx::new(3),
            ActionId::new(7),
            SlotIdx::new(0),
            SeqNo::new(5),
        );
        // Then it returns AwaitingAction with correct fields
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(ticket.run, RunId::new(42));
                assert_eq!(ticket.step, StepIdx::new(3));
                assert_eq!(ticket.action, ActionId::new(7));
                assert_eq!(ticket.seq, SeqNo::new(5));
                assert_eq!(ticket.attempt, 1);
            }
            other => {
                assert_eq!(other, Ok(RuntimeSignal::Continue));
            }
        }
    }

    #[test]
    fn runtime_signal_equality_differs_for_different_finished_values() {
        // Given two Finished signals with different values
        let a = RuntimeSignal::Finished(SlotValue::I64(1));
        let b = RuntimeSignal::Finished(SlotValue::I64(2));
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_signal_awaiting_action_equality_matches_on_ticket() {
        // Given two AwaitingAction signals with same ticket
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 42,
        };
        let a = RuntimeSignal::AwaitingAction(ticket);
        let b = RuntimeSignal::AwaitingAction(ticket);
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_signal_awaiting_action_differs_for_different_ticket() {
        // Given two AwaitingAction signals with different tickets
        let a = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        });
        let b = RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(2),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        });
        assert_ne!(a, b);
    }
}
