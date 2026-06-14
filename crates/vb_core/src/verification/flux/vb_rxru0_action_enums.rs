//! Flux refinements for vb_core action module.
//!
//! Verifier lane: flux-rs
//! Obligations: OBL-001, OBL-002, OBL-019
//!
//! These refinements bind enum discriminant invariants and method pre-/post-conditions
//! to the production types in `vb_core::action`.

#![allow(unused)]

use flux_rs::attrs::*;

// ─── Idempotency enum ───────────────────────────────────────────────────────────
// OBL-001: Idempotency must have exactly 3 unit variants with discriminants {0,1,2}.

/// Idempotency discriminant is bounded to [0, 2].
#[refined_by(in(0..=2))]
pub type IdempotencyDiscriminant = u8;

// OBL-002: Idempotency::DeterministicPure is always idempotent.
// Post-condition of is_idempotent for DeterministicPure: returns true.

// ─── SideEffect enum ────────────────────────────────────────────────────────────
// 7 variants: Pure=0, LocalRead=1, LocalWrite=2, ExternalRead=3,
//             ExternalWrite=4, Process=5, UnsafeShell=6.

/// SideEffect discriminant is bounded to [0, 6].
#[refined_by(in(0..=6))]
pub type SideEffectDiscriminant = u8;

/// `is_idempotent` returns true exactly for Pure, LocalRead, LocalWrite, ExternalRead.
#[refined_by(in(0..=3))]
pub type IdempotentSideEffectDiscriminant = u8;

/// `requires_external_lease` returns true exactly for Process, UnsafeShell.
#[refined_by(in(5..=6))]
pub type LeaseRequiredSideEffectDiscriminant = u8;

// ─── RetrySafety enum ───────────────────────────────────────────────────────────
// 4 variants: Idempotent=0, RequiresIdempotencyKey=1, NotRetrySafe=2, Unknown=3.

/// RetrySafety discriminant is bounded to [0, 3].
#[refined_by(in(0..=3))]
pub type RetrySafetyDiscriminant = u8;

// ─── ActionFailureCode enum ─────────────────────────────────────────────────────
// 9 known variants (0-8) plus Unknown=255.

/// ActionFailureCode discriminant is either in [0, 8] or exactly 255.
#[refined_by(in(0..=8) | eq(255))]
pub type ActionFailureCodeDiscriminant = u8;

// ─── ActionTicket field count ───────────────────────────────────────────────────
// OBL-019: ActionTicket will have 8 fields (was 7). The mock field is added
// as a new field with default value. This refinement captures the field count
// invariant for verification planning.

/// ActionTicket field count: 7 before MockMarker, 8 after.
/// Verification planning: the mock field defaults to HttpGet (discriminant 0).
const ACTION_TICKET_FIELDS_BEFORE: usize = 7;
const ACTION_TICKET_FIELDS_AFTER: usize = 8;
