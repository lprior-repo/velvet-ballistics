//! Cold-path validation for the dense registry used by numeric action dispatch.
use super::{ActionContract, ActionName, ActionNameError};
use crate::ActionId;

/// A contract registry cannot be used by the production numeric dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionRegistryError {
    /// An entry cannot be represented by the action identifier domain.
    #[error("action registry index {index} exceeds the identifier range")]
    IndexOverflow { index: usize },
    /// Sparse, reordered, or duplicate identifiers violate direct dispatch.
    #[error("action registry index {index} has ID {actual:?}, expected {expected:?}")]
    IndexMismatch {
        index: usize,
        expected: ActionId,
        actual: ActionId,
    },
    /// Deserialized names must satisfy the same rules as constructor input.
    #[error("action {action:?} has an invalid name: {source}")]
    InvalidName {
        action: ActionId,
        #[source]
        source: ActionNameError,
    },
}

/// Validate the exact dense registry consumed by indexed runtime dispatch.
/// Requiring index == identifier also excludes duplicate identifiers.
pub fn validate_action_registry(contracts: &[ActionContract]) -> Result<(), ActionRegistryError> {
    for (index, contract) in contracts.iter().enumerate() {
        let expected = ActionId::new(
            u16::try_from(index).map_err(|_| ActionRegistryError::IndexOverflow { index })?,
        );
        if contract.id != expected {
            return Err(ActionRegistryError::IndexMismatch {
                index,
                expected,
                actual: contract.id,
            });
        }
        ActionName::new(contract.name.as_str()).map_err(|source| {
            ActionRegistryError::InvalidName {
                action: contract.id,
                source,
            }
        })?;
    }
    Ok(())
}
