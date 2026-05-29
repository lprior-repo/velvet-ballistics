// Obligation: PO-004
// Claim: runtime_open_no_side_effects
#![cfg(feature = "proof-vb-aoah-migration")]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AoahPhase {
    OldStore,
    MigrationRequired,
    Migrating,
    Verified,
    CurrentStore,
    CleanupSucceeded,
    NoopVerified,
    BoundedError,
}

// Flux refinement intent for PO-004: #[flux_rs::refined_by(old_empty: bool, verified_state: bool, manifest_is_current: bool)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AoahState {
    pub phase: AoahPhase,
    // Flux field intent: #[flux_rs::field(u8)]
    pub old_records: u8,
    // Flux field intent: #[flux_rs::field(bool[verified_state])]
    pub verified: bool,
    // Flux field intent: #[flux_rs::field(bool[manifest_is_current])]
    pub manifest_current: bool,
}

pub const MAX_RECORDS: u8 = 4;
pub const MAX_BYTES: u8 = 16;

// Flux signature intent: #[flux_rs::sig(fn(AoahState) -> bool)]
pub fn cleanup_success_refined(state: AoahState) -> bool {
    state.phase != AoahPhase::CleanupSucceeded || state.old_records == 0
}

// Flux signature intent: #[flux_rs::sig(fn(AoahState) -> bool)]
pub fn manifest_advance_refined(state: AoahState) -> bool {
    !state.manifest_current || state.verified
}

// Flux signature intent: #[flux_rs::sig(fn(AoahState) -> bool)]
pub fn empty_noop_refined(state: AoahState) -> bool {
    state.phase != AoahPhase::NoopVerified || (state.old_records == 0 && state.verified)
}

// Flux signature intent: #[flux_rs::sig(fn(u8, u8) -> Option<u8>)]
pub fn checked_byte_total(left: u8, right: u8) -> Option<u8> {
    left.checked_add(right).filter(|total| *total <= MAX_BYTES)
}
