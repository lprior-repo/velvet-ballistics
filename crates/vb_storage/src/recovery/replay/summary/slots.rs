#![forbid(unsafe_code)]
//! Slot recovery, taint extraction, pending actions, and replay error mapping.

mod errors;
mod pending;
mod recovery;
mod taint;

// Re-export public API
pub use pending::pending_actions_from_events;

// Re-export items used by tests and other internal modules
pub(crate) use recovery::{RecoveredSlots, recover_slots};
pub(crate) use taint::{RecoveredSlotTaint, recovered_slot_taint};
pub(crate) use errors::replay_error_to_recovery;
