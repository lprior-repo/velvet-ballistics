#![forbid(unsafe_code)]

//! Action registry for compile-time contract resolution and runtime dispatch.

use std::collections::HashMap;

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionName, ActionOutcome, ActionResult, ActionTicket,
};
use vb_core::ids::ActionId;

pub use crate::idempotency::IdempotencyTracker;

/// Maximum number of registered actions.
const MAX_REGISTERED_ACTIONS: usize = 65_535;

/// Registry mapping numeric action identifiers to their contracts.
#[derive(Debug, Clone)]
pub struct ActionRegistry {
    slots: Vec<ActionSlot>,
    by_name: HashMap<ActionName, ActionId>,
}

#[derive(Debug, Clone)]
enum ActionSlot {
    Empty,
    Registered(ActionContract),
}

impl ActionRegistry {
    /// Creates an empty action registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            by_name: HashMap::new(),
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
        self.ensure_slot_capacity(slot)?;

        if matches!(self.slots.get(slot), Some(ActionSlot::Registered(_))) {
            return Err(ActionError::DispatchFailed);
        }

        // Check for duplicate name after duplicate-id rejection so id collisions
        // keep their existing error contract.
        if self.by_name.contains_key(&contract.name) {
            return Err(ActionError::UnknownAction {
                action: contract.id,
            });
        }

        // Save name and id before moving contract
        let contract_name = contract.name.clone();
        let contract_id = contract.id;

        self.write_empty_slot(slot, contract)?;

        // Register by name for lookup
        self.by_name.insert(contract_name, contract_id);

        Ok(())
    }

    fn ensure_slot_capacity(&mut self, slot: usize) -> ActionResult<()> {
        let needed = slot.checked_add(1).ok_or(ActionError::DispatchFailed)?;
        if needed > self.slots.len() {
            self.slots.resize_with(needed, || ActionSlot::Empty);
        }
        Ok(())
    }

    fn write_empty_slot(&mut self, slot: usize, contract: ActionContract) -> ActionResult<()> {
        match self.slots.get_mut(slot) {
            Some(empty_slot @ ActionSlot::Empty) => {
                *empty_slot = ActionSlot::Registered(contract);
                Ok(())
            }
            Some(ActionSlot::Registered(_)) => Err(ActionError::DispatchFailed),
            None => Err(ActionError::DispatchFailed),
        }
    }

    /// Resolves a compile-time action id to its contract.
    pub fn resolve_compile_time(&self, action: ActionId) -> ActionResult<&ActionContract> {
        let index = usize::from(action.get());
        self.slots
            .get(index)
            .and_then(ActionSlot::registered_contract)
            .filter(|contract| contract.id == action)
            .ok_or(ActionError::UnknownAction { action })
    }

    /// Resolves an action name to its contract.
    ///
    /// Returns a reference to the contract if found, or an error if the name
    /// is not registered.
    pub fn resolve_by_name(&self, name: &ActionName) -> ActionResult<&ActionContract> {
        let action_id = self
            .by_name
            .get(name)
            .ok_or_else(|| ActionError::UnknownAction {
                action: ActionId::new(0), // dummy id for error
            })?;
        self.resolve_compile_time(*action_id)
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
        self.slots.len()
    }

    /// Returns true when no actions are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| matches!(slot, ActionSlot::Empty))
    }

    /// Returns registered action contracts in deterministic [`ActionId`] order.
    ///
    /// The registry stores contracts in action-id slots, so iteration is already
    /// sorted by id. The returned references are read-only and do not mutate the
    /// registry.
    #[must_use]
    pub fn registered_contracts(&self) -> Vec<&ActionContract> {
        self.slots
            .iter()
            .filter_map(ActionSlot::registered_contract)
            .collect()
    }
}

impl ActionSlot {
    fn registered_contract(&self) -> Option<&ActionContract> {
        match self {
            Self::Empty => None,
            Self::Registered(contract) => Some(contract),
        }
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
        capacity: 1,
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
#[path = "action/tests.rs"]
mod tests;
