#![forbid(unsafe_code)]
//! Emergency-deny marker storage seam.
//!
//! Single fixed key `active` with exact one-byte value `[1]` for version 1.
//! Missing key = Inactive. Valid exact `[1]` = Active. Zero-length or extra
//! bytes = Malformed. Wrong version byte = UnsupportedVersion.
//!
//! Activation always performs a strict-batch commit with SyncAll before
//! returning success, even if the marker is already Active, because a prior
//! failed persistence can leave visible but not durably confirmed data.
//! Repeated activation writes the same constant marker, giving full equality
//! with no overflow or race counter.

use crate::error::JournalError;

/// Emergency-deny overlay state persisted in the Fjall journal store.
///
/// `Inactive` means no emergency-deny marker is present. `Active` means a
/// valid versioned marker is present. No sequence counter; repeated
/// activation writes the same constant marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyDenyState {
    Inactive,
    Active,
}

/// Fixed marker key for the emergency-deny overlay.
const EMERGENCY_DENY_KEY: &[u8] = b"active";

/// Current emergency-deny marker version byte.
const EMERGENCY_DENY_VERSION: u8 = 1;

/// The complete valid active marker value: one version byte.
const EMERGENCY_DENY_ACTIVE_MARKER: [u8; 1] = [EMERGENCY_DENY_VERSION];

impl crate::journal::FjallJournal {
    /// Returns the current emergency-deny state persisted in this store.
    ///
    /// `Inactive` when no marker key exists. `Active` when a valid
    /// versioned marker is present. Storage read errors propagate as
    /// `JournalError::Fjall`, never converted to `Inactive`.
    /// Malformed markers or unsupported versions return their specific
    /// typed errors, never `Inactive`.
    pub fn emergency_deny_state(&self) -> Result<EmergencyDenyState, JournalError> {
        let result = self
            .emergency_deny
            .get(EMERGENCY_DENY_KEY)
            .map_err(JournalError::Fjall)?;
        let value = match result {
            Some(v) => v,
            None => return Ok(EmergencyDenyState::Inactive),
        };
        let bytes = value.to_vec();
        // Exact one-byte check
        if bytes.len() != 1 {
            return Err(JournalError::EmergencyDenyMarkerMalformed);
        }
        // Validate version byte
        let version = *bytes
            .first()
            .ok_or(JournalError::EmergencyDenyMarkerMalformed)?;
        if version != EMERGENCY_DENY_VERSION {
            return Err(JournalError::EmergencyDenyMarkerUnsupportedVersion { version });
        }
        Ok(EmergencyDenyState::Active)
    }

    /// Activates the emergency-deny overlay with strict durability.
    ///
    /// Reads and validates any existing marker first, rejecting corrupt or
    /// unsupported state without overwriting it. Then writes the constant
    /// active marker and persists with `PersistMode::SyncAll` before
    /// returning success. Always performs a strict-batch commit+SyncAll
    /// even if already Active, because a prior failed persistence can
    /// leave visible but not durably confirmed data.
    ///
    /// Idempotent: repeated calls write the same constant marker,
    /// producing full equality with no overflow or race counter.
    /// Returns the new state.
    pub fn activate_emergency_deny(&self) -> Result<EmergencyDenyState, JournalError> {
        // First read and validate existing marker, rejecting corrupt/unsupported
        // state without overwriting it.
        let current = self.emergency_deny_state()?;
        let _ = current;
        // Write constant active marker via strict batch commit
        let mut batch = self.batch();
        batch.put_emergency_deny(EMERGENCY_DENY_ACTIVE_MARKER)?;
        batch.strict().commit()?;
        Ok(EmergencyDenyState::Active)
    }
}
