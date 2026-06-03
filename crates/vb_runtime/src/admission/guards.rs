#![forbid(unsafe_code)]
//! Capability guard helpers for admission control.

use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::ActionId;

use super::errors::AdmissionError;

/// Checks whether a capability is granted for an action.
///
/// Returns `Ok(())` if the action's capability is covered by the granted set,
/// or `Err(AdmissionError::CapabilityDenied)` otherwise.
pub fn check_capability(
    action: ActionId,
    required: &Capability,
    granted: &CapabilitySet,
) -> Result<(), AdmissionError> {
    if granted.grants(required) {
        Ok(())
    } else {
        Err(AdmissionError::CapabilityDenied {
            action,
            required: required.clone(),
            granted: granted.clone(),
        })
    }
}

pub(crate) fn capability_count_mismatch_error(
    required: &[Capability],
    granted: &CapabilitySet,
) -> AdmissionError {
    let fallback = Capability::new("__capability_count_mismatch__".into(), ActionId::new(0));
    let required_capability = required.first().cloned().unwrap_or(fallback);
    AdmissionError::CapabilityDenied {
        action: required_capability.action_id(),
        required: required_capability,
        granted: granted.clone(),
    }
}
