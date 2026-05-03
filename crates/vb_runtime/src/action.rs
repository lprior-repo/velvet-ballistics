#![forbid(unsafe_code)]

//! Action registry for compile-time contract resolution and runtime dispatch.

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionOutcome, ActionResult, ActionTicket,
    Idempotency, RetrySafety, SideEffect,
};
use vb_core::ids::ActionId;

/// Maximum number of registered actions.
const MAX_REGISTERED_ACTIONS: usize = 65_535;

/// Registry mapping numeric action identifiers to their contracts.
#[derive(Debug, Clone)]
pub struct ActionRegistry {
    contracts: Vec<ActionContract>,
}

impl ActionRegistry {
    /// Creates an empty action registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            contracts: Vec::new(),
        }
    }

    /// Registers an action contract. Returns an error if the action id is
    /// already registered or the registry is full.
    pub fn register(&mut self, contract: ActionContract) -> ActionResult<()> {
        let index = contract.id.get();
        let slot = usize::from(index);
        if slot >= MAX_REGISTERED_ACTIONS {
            return Err(ActionError::UnknownAction {
                action: contract.id,
            });
        }
        if slot < self.contracts.len() {
            if self.contracts.get(slot).is_some()
                && self
                    .contracts
                    .get(slot)
                    .is_some_and(|existing| existing.id == contract.id)
            {
                return Err(ActionError::DispatchFailed);
            }
            *self
                .contracts
                .get_mut(slot)
                .ok_or(ActionError::DispatchFailed)? = contract;
            return Ok(());
        }
        // Extend to fill up to the target slot.
        let needed = slot.checked_add(1).ok_or(ActionError::DispatchFailed)?;
        self.contracts.resize_with(needed, || ActionContract {
            id: ActionId::new(0),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        });
        *self
            .contracts
            .get_mut(slot)
            .ok_or(ActionError::DispatchFailed)? = contract;
        Ok(())
    }

    /// Resolves a compile-time action id to its contract.
    pub fn resolve_compile_time(&self, action: ActionId) -> ActionResult<&ActionContract> {
        let index = usize::from(action.get());
        self.contracts
            .get(index)
            .filter(|contract| contract.id == action)
            .ok_or(ActionError::UnknownAction { action })
    }

    /// Dispatches an action input through the registry and produces an outcome.
    /// In generated mode, this becomes a match on ActionId. Here we provide the
    /// generic table-driven dispatch.
    pub fn dispatch(
        &self,
        input: &ActionInput,
        contract: &ActionContract,
    ) -> ActionResult<ActionOutcome> {
        let resolved = self.resolve_compile_time(input.action)?;
        if resolved.id != contract.id {
            return Err(ActionError::UnknownAction {
                action: input.action,
            });
        }
        // Generated dispatch shape: match on ActionId.
        // This table-driven path handles the general case.
        dispatch_generic(input, resolved)
    }

    /// Returns the number of registered actions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    /// Returns true when no actions are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic table-driven dispatch that produces a suspended ticket outcome.
fn dispatch_generic(input: &ActionInput, contract: &ActionContract) -> ActionResult<ActionOutcome> {
    validate_input_bytes(input, contract)?;
    let ticket = ActionTicket {
        run: input.run,
        step: input.step,
        seq: input.ticket.seq,
        action: input.action,
        attempt: input.ticket.attempt,
        idempotency_key: input.ticket.idempotency_key,
    };
    Ok(ActionOutcome::Suspended(ticket))
}

/// Validates that the input payload does not exceed the contract's byte limit.
fn validate_input_bytes(_input: &ActionInput, contract: &ActionContract) -> ActionResult<()> {
    // Byte-level validation requires encoded_len from the caller.
    // This is a structural check placeholder; actual byte counting
    // happens at the IPC boundary.
    if contract.max_input_bytes == 0 && contract.input_slot_count > 0 {
        return Err(ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{RunId, SeqNo, SlotIdx, StepIdx};

    fn test_contract(id: u16) -> ActionContract {
        ActionContract {
            id: ActionId::new(id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }
    }

    fn test_input(action: u16) -> ActionInput {
        ActionInput {
            run: RunId::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(action),
            input: SlotIdx::new(0),
            ticket: ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(0),
                action: ActionId::new(action),
                attempt: 1,
                idempotency_key: 0,
            },
        }
    }

    #[test]
    fn register_and_resolve_action() {
        let mut registry = ActionRegistry::new();
        let contract = test_contract(10);
        assert_eq!(registry.register(contract), Ok(()));
        let resolved = registry.resolve_compile_time(ActionId::new(10));
        assert_eq!(resolved.map(|c| c.id), Ok(ActionId::new(10)));
    }

    #[test]
    fn resolve_unknown_action_returns_error() {
        let registry = ActionRegistry::new();
        let result = registry.resolve_compile_time(ActionId::new(99));
        assert_eq!(
            result,
            Err(ActionError::UnknownAction {
                action: ActionId::new(99)
            })
        );
    }

    #[test]
    fn dispatch_produces_suspended_outcome() {
        let mut registry = ActionRegistry::new();
        let contract = test_contract(5);
        assert_eq!(registry.register(contract), Ok(()));
        let input = test_input(5);
        let resolved = registry.resolve_compile_time(ActionId::new(5));
        assert_eq!(resolved.as_ref().map(|c| c.id), Ok(ActionId::new(5)));
        let contract = resolved.ok().cloned();
        assert_eq!(contract.as_ref().map(|c| c.id), Some(ActionId::new(5)));
        let Some(ref contract) = contract else { return };
        let result = registry.dispatch(&input, contract);
        match result {
            Ok(ActionOutcome::Suspended(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(5));
            }
            other => assert_eq!(
                other,
                Ok(ActionOutcome::Suspended(ActionTicket {
                    run: RunId::new(1),
                    step: StepIdx::new(0),
                    seq: SeqNo::new(0),
                    action: ActionId::new(5),
                    attempt: 1,
                    idempotency_key: 0,
                }))
            ),
        }
    }

    #[test]
    fn register_duplicate_returns_error() {
        let mut registry = ActionRegistry::new();
        let contract = test_contract(3);
        assert_eq!(registry.register(contract), Ok(()));
        let duplicate = test_contract(3);
        assert_eq!(
            registry.register(duplicate),
            Err(ActionError::DispatchFailed)
        );
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = ActionRegistry::default();
        assert_eq!(registry.is_empty(), true);
    }

    #[test]
    fn len_returns_zero_for_new_registry() {
        let registry = ActionRegistry::new();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn len_increases_after_register() {
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.register(test_contract(1)), Ok(()));
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        assert_eq!(registry.len(), 6);
    }

    #[test]
    fn validate_input_bytes_rejects_when_max_input_bytes_is_zero() {
        let mut registry = ActionRegistry::new();
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        assert_eq!(registry.register(contract), Ok(()));
        let input = test_input(1);
        let resolved = registry.resolve_compile_time(ActionId::new(1));
        assert_eq!(resolved.as_ref().map(|c| c.id), Ok(ActionId::new(1)));
        let contract = resolved.ok().cloned();
        let Some(ref contract) = contract else { return };
        let result = registry.dispatch(&input, contract);
        assert_eq!(
            result,
            Err(ActionError::PayloadTooLarge {
                max_bytes: 0,
                actual_bytes: 0
            })
        );
    }

    #[test]
    fn action_registry_resolve_returns_correct_contract() {
        // Given a registry with one contract
        let mut registry = ActionRegistry::new();
        let contract = test_contract(5);
        assert_eq!(registry.register(contract), Ok(()));
        // When resolving the action
        let result = registry.resolve_compile_time(ActionId::new(5));
        // Then it returns the correct contract with matching id
        match result {
            Ok(c) => {
                assert_eq!(c.id, ActionId::new(5));
                assert_eq!(c.input_slot_count, 1);
                assert_eq!(c.output_slot_count, 1);
                assert_eq!(c.max_input_bytes, 1024);
                assert_eq!(c.max_output_bytes, 1024);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn action_registry_register_fills_gaps() {
        // Given a registry where action 10 is registered first
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(10)), Ok(()));
        // Then len is 11 (slots 0..10)
        assert_eq!(registry.len(), 11);
        // And action 10 resolves correctly
        let resolved = registry.resolve_compile_time(ActionId::new(10));
        assert_eq!(resolved.map(|c| c.id), Ok(ActionId::new(10)));
    }

    #[test]
    fn action_registry_dispatch_rejects_mismatched_contract() {
        // Given a registry with action 5
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        // When dispatching with input for action 5 but a different contract
        let input = test_input(5);
        let wrong_contract = ActionContract {
            id: ActionId::new(3),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let result = registry.dispatch(&input, &wrong_contract);
        // Then it returns an UnknownAction error
        assert_eq!(
            result,
            Err(ActionError::UnknownAction {
                action: ActionId::new(5)
            })
        );
    }

    #[test]
    fn action_registry_is_not_empty_after_register() {
        // Given a registry with one action
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(1)), Ok(()));
        // When checking is_empty
        // Then it is not empty
        assert_eq!(registry.is_empty(), false);
    }

    #[test]
    fn action_registry_new_default_matches_new() {
        // Given a default registry
        let default = ActionRegistry::default();
        let new = ActionRegistry::new();
        // When comparing
        // Then both are empty with same len
        assert_eq!(default.is_empty(), true);
        assert_eq!(new.is_empty(), true);
        assert_eq!(default.len(), 0);
        assert_eq!(new.len(), 0);
    }

    #[test]
    fn action_registry_register_multiple_actions() {
        // Given a registry
        let mut registry = ActionRegistry::new();
        // When registering actions 0, 1, 2
        assert_eq!(registry.register(test_contract(0)), Ok(()));
        assert_eq!(registry.register(test_contract(1)), Ok(()));
        assert_eq!(registry.register(test_contract(2)), Ok(()));
        // Then all resolve correctly
        assert_eq!(
            registry
                .resolve_compile_time(ActionId::new(0))
                .map(|c| c.id),
            Ok(ActionId::new(0))
        );
        assert_eq!(
            registry
                .resolve_compile_time(ActionId::new(1))
                .map(|c| c.id),
            Ok(ActionId::new(1))
        );
        assert_eq!(
            registry
                .resolve_compile_time(ActionId::new(2))
                .map(|c| c.id),
            Ok(ActionId::new(2))
        );
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn action_registry_resolve_unregistered_action_fails() {
        // Given a registry with action 0
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(0)), Ok(()));
        // When resolving unregistered action 5
        let result = registry.resolve_compile_time(ActionId::new(5));
        // Then it returns UnknownAction
        assert_eq!(
            result,
            Err(ActionError::UnknownAction {
                action: ActionId::new(5)
            })
        );
    }

    #[test]
    fn action_registry_dispatch_with_correct_contract_succeeds() {
        // Given a registry with action 0
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(0)), Ok(()));
        let input = test_input(0);
        let contract = test_contract(0);
        // When dispatching with matching contract
        let result = registry.dispatch(&input, &contract);
        // Then it succeeds with Suspended outcome
        match result {
            Ok(ActionOutcome::Suspended(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(0));
            }
            other => {
                assert_eq!(
                    other,
                    Ok(ActionOutcome::Suspended(ActionTicket {
                        run: RunId::new(1),
                        step: StepIdx::new(0),
                        seq: SeqNo::new(0),
                        action: ActionId::new(0),
                        attempt: 1,
                        idempotency_key: 0,
                    }))
                );
            }
        }
    }

    #[test]
    fn action_contract_fields_are_preserved() {
        // Given a contract with specific fields
        let contract = ActionContract {
            id: ActionId::new(42),
            input_slot_count: 3,
            output_slot_count: 2,
            max_input_bytes: 2048,
            max_output_bytes: 4096,
            timeout_ms: 10000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([]),
        };
        // When registering and resolving
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(contract), Ok(()));
        let resolved = registry.resolve_compile_time(ActionId::new(42));
        // Then all fields are preserved
        match resolved {
            Ok(c) => {
                assert_eq!(c.id, ActionId::new(42));
                assert_eq!(c.input_slot_count, 3);
                assert_eq!(c.output_slot_count, 2);
                assert_eq!(c.max_input_bytes, 2048);
                assert_eq!(c.max_output_bytes, 4096);
                assert_eq!(c.timeout_ms, 10000);
                assert_eq!(c.idempotency, Idempotency::IdempotentExternal);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn action_registry_len_increases_with_gap() {
        // Given a registry with action 5
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        // Then len is 6 (slots 0..5)
        assert_eq!(registry.len(), 6);
    }

    #[test]
    fn action_registry_gap_slot_resolves_for_default_id() {
        // Given a registry with action 5
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        // When resolving action 0 (gap slot filled with default ActionId(0))
        let result = registry.resolve_compile_time(ActionId::new(0));
        // Then it resolves because gap slots have ActionId(0) which matches
        match result {
            Ok(c) => {
                assert_eq!(c.id, ActionId::new(0));
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn action_registry_gap_slot_nondefault_id_fails() {
        // Given a registry with action 5
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        // When resolving action 3 (gap slot with default id, not matching 3)
        let result = registry.resolve_compile_time(ActionId::new(3));
        // Then it returns UnknownAction
        assert_eq!(
            result,
            Err(ActionError::UnknownAction {
                action: ActionId::new(3)
            })
        );
    }

    // =======================================================================
    // Adversarial BDD tests - action registry attack vectors
    // =======================================================================

    #[test]
    fn action_registry_dispatch_unknown_action_returns_exact_error_variant() {
        // Given an empty registry
        let registry = ActionRegistry::new();
        let input = test_input(99);
        let contract = test_contract(99);
        // When dispatching an unknown action
        let result = registry.dispatch(&input, &contract);
        // Then it returns UnknownAction with the exact action id
        assert_eq!(
            result,
            Err(ActionError::UnknownAction {
                action: ActionId::new(99)
            })
        );
    }

    #[test]
    fn action_registry_register_then_reregister_same_id_returns_dispatch_failed() {
        // Given a registry with action 1
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(1)), Ok(()));
        // When registering the same action id again
        let result = registry.register(test_contract(1));
        // Then it returns DispatchFailed (duplicate rejection)
        assert_eq!(result, Err(ActionError::DispatchFailed));
    }

    #[test]
    fn action_registry_register_max_action_id_does_not_overflow() {
        // Given an empty registry
        let mut registry = ActionRegistry::new();
        // When registering action at max valid index (65534)
        let contract = ActionContract {
            id: ActionId::new(65534),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let result = registry.register(contract);
        // Then it succeeds (65534 < 65535 = MAX_REGISTERED_ACTIONS)
        assert_eq!(result, Ok(()));
        assert_eq!(registry.len(), 65535);
    }

    #[test]
    fn action_registry_validate_input_bytes_rejects_zero_with_slots() {
        // Given a contract with max_input_bytes=0 and input_slot_count=1
        let mut registry = ActionRegistry::new();
        let contract = ActionContract {
            id: ActionId::new(1),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        assert_eq!(registry.register(contract), Ok(()));
        let input = test_input(1);
        let resolved = registry.resolve_compile_time(ActionId::new(1));
        let contract = match resolved {
            Ok(c) => c.clone(),
            Err(_) => return,
        };
        // When dispatching
        let result = registry.dispatch(&input, &contract);
        // Then it returns PayloadTooLarge (zero bytes with slots)
        assert_eq!(
            result,
            Err(ActionError::PayloadTooLarge {
                max_bytes: 0,
                actual_bytes: 0,
            })
        );
    }

    #[test]
    fn action_registry_dispatch_with_contract_zero_bytes_and_zero_slots_succeeds() {
        // Given a contract with max_input_bytes=0 and input_slot_count=0
        let mut registry = ActionRegistry::new();
        let contract = ActionContract {
            id: ActionId::new(2),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        assert_eq!(registry.register(contract), Ok(()));
        let input = ActionInput {
            run: RunId::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(2),
            input: SlotIdx::new(0),
            ticket: ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(0),
                action: ActionId::new(2),
                attempt: 1,
                idempotency_key: 0,
            },
        };
        let contract = match registry.resolve_compile_time(ActionId::new(2)) {
            Ok(c) => c.clone(),
            Err(_) => return,
        };
        // When dispatching with zero bytes and zero slots
        let result = registry.dispatch(&input, &contract);
        // Then it succeeds (no payload to validate)
        match result {
            Ok(ActionOutcome::Suspended(_)) => {}
            other => {
                assert_eq!(
                    other,
                    Ok(ActionOutcome::Suspended(ActionTicket {
                        run: RunId::new(1),
                        step: StepIdx::new(0),
                        seq: SeqNo::new(0),
                        action: ActionId::new(2),
                        attempt: 1,
                        idempotency_key: 0,
                    }))
                );
            }
        }
    }

    #[test]
    fn action_registry_resolve_after_many_registrations_finds_correct_action() {
        // Given a registry with actions 0, 5, 10, 20
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(0)), Ok(()));
        assert_eq!(registry.register(test_contract(5)), Ok(()));
        assert_eq!(registry.register(test_contract(10)), Ok(()));
        assert_eq!(registry.register(test_contract(20)), Ok(()));
        // When resolving action 10
        let result = registry.resolve_compile_time(ActionId::new(10));
        // Then it returns the correct contract
        match result {
            Ok(c) => {
                assert_eq!(c.id, ActionId::new(10));
                assert_eq!(c.input_slot_count, 1);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn action_registry_dispatch_returns_ticket_with_correct_action_from_input() {
        // Given a registry with action 3
        let mut registry = ActionRegistry::new();
        assert_eq!(registry.register(test_contract(3)), Ok(()));
        let input = ActionInput {
            run: RunId::new(77),
            step: StepIdx::new(5),
            action: ActionId::new(3),
            input: SlotIdx::new(0),
            ticket: ActionTicket {
                run: RunId::new(77),
                step: StepIdx::new(5),
                seq: SeqNo::new(10),
                action: ActionId::new(3),
                attempt: 2,
                idempotency_key: 99,
            },
        };
        let contract = match registry.resolve_compile_time(ActionId::new(3)) {
            Ok(c) => c.clone(),
            Err(_) => return,
        };
        // When dispatching
        let result = registry.dispatch(&input, &contract);
        // Then the returned ticket carries the input ticket's fields
        match result {
            Ok(ActionOutcome::Suspended(ticket)) => {
                assert_eq!(ticket.action, ActionId::new(3));
                assert_eq!(ticket.run, RunId::new(77));
                assert_eq!(ticket.step, StepIdx::new(5));
                assert_eq!(ticket.seq, SeqNo::new(10));
                assert_eq!(ticket.attempt, 2);
                assert_eq!(ticket.idempotency_key, 99);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(ActionOutcome::Suspended(ActionTicket {
                        run: RunId::new(0),
                        step: StepIdx::new(0),
                        seq: SeqNo::new(0),
                        action: ActionId::new(0),
                        attempt: 0,
                        idempotency_key: 0,
                    }))
                );
            }
        }
    }

    // ========================================================================
    // IdempotencyTracker tests
    // ========================================================================

    #[test]
    fn idempotency_tracker_new_is_empty() {
        use crate::idempotency::IdempotencyTracker;
        let tracker = IdempotencyTracker::new();
        assert_eq!(tracker.is_empty(), true);
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn idempotency_tracker_record_completion_succeeds() {
        use crate::idempotency::IdempotencyTracker;
        let mut tracker = IdempotencyTracker::new();
        let ticket = ActionTicket {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 42,
        };
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert_eq!(tracker.is_completed(&ticket), true);
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn idempotency_tracker_duplicate_completion_returns_error() {
        use crate::idempotency::IdempotencyTracker;
        let mut tracker = IdempotencyTracker::new();
        let ticket = ActionTicket {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 99,
        };
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert_eq!(
            tracker.mark_completed(&ticket),
            Err(ActionError::CompletionAlreadyRecorded)
        );
    }

    #[test]
    fn idempotency_tracker_different_keys_are_independent() {
        use crate::idempotency::IdempotencyTracker;
        let mut tracker = IdempotencyTracker::new();
        let ticket_a = ActionTicket {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 1,
        };
        let ticket_b = ActionTicket {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 2,
        };
        let ticket_c = ActionTicket {
            run: RunId::new(0),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 0,
            idempotency_key: 3,
        };
        assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
        assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
        assert_eq!(tracker.is_completed(&ticket_a), true);
        assert_eq!(tracker.is_completed(&ticket_b), true);
        assert_eq!(tracker.is_completed(&ticket_c), false);
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn idempotency_tracker_default_matches_new() {
        use crate::idempotency::IdempotencyTracker;
        let default = IdempotencyTracker::default();
        let new = IdempotencyTracker::new();
        assert_eq!(default.len(), new.len());
        assert_eq!(default.is_empty(), new.is_empty());
    }
}
