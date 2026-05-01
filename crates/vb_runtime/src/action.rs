#![forbid(unsafe_code)]

//! Action registry for compile-time contract resolution and runtime dispatch.

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionOutcome, ActionResult, ActionTicket,
    Idempotency,
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
        assert_eq!(result, Err(ActionError::UnknownAction { action: ActionId::new(99) }));
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
            other => assert_eq!(other, Ok(ActionOutcome::Suspended(ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(0),
                action: ActionId::new(5),
                attempt: 1,
                idempotency_key: 0,
            }))),
        }
    }

    #[test]
    fn register_duplicate_returns_error() {
        let mut registry = ActionRegistry::new();
        let contract = test_contract(3);
        assert_eq!(registry.register(contract), Ok(()));
        let duplicate = test_contract(3);
        assert_eq!(registry.register(duplicate), Err(ActionError::DispatchFailed));
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
        };
        assert_eq!(registry.register(contract), Ok(()));
        let input = test_input(1);
        let resolved = registry.resolve_compile_time(ActionId::new(1));
        assert_eq!(resolved.as_ref().map(|c| c.id), Ok(ActionId::new(1)));
        let contract = resolved.ok().cloned();
        let Some(ref contract) = contract else { return };
        let result = registry.dispatch(&input, contract);
        assert_eq!(result, Err(ActionError::PayloadTooLarge { max_bytes: 0, actual_bytes: 0 }));
    }
}
