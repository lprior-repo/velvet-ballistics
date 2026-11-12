#![forbid(unsafe_code)]
//! Manual gate evaluation and enforcement (MASTER §77.15, V1-MIN).
//!
//! Pure functions over the value types in [`vb_core::manual_gate`]:
//! decision evaluation, timeout/escalation enforcement, receipt digest
//! binding, and replay reconstruction. Nothing here performs an external
//! effect, mints a capability, or touches the journal directly; callers
//! journal the returned [`vb_core::manual_gate::GateOutcomeRecord`] as a
//! `JournalEvent::GateDecisionEvent`.
//!
//! Enforcement summary:
//! - The submitted evidence digest must equal the gate's evidence digest.
//! - High-risk, non-compensatable approvals require evidence display.
//! - Separation of duties (requester != approver) always applies to
//!   high-risk approvals and, when the gate rule requires it, to every
//!   approval. It also applies to break-glass approvals at high risk.
//! - Approval requires the gate's approver role and, when the gate
//!   demands it, a non-empty reason.
//! - Break-glass waives the role check (that is the point of break-glass)
//!   but requires the gate to allow it, a non-empty justification, and
//!   leaves a full audit trail in the record.
//! - Decisions after the deadline are rejected; [`evaluate_timeout`]
//!   produces the escalation record instead.
//! - Replay reconstructs history from records only. It never synthesizes
//!   a receipt and never authorizes a live effect.

mod errors;
mod evaluate;
mod receipt;
mod replay;

#[cfg(test)]
mod tests;

pub use errors::GateError;
pub use evaluate::{evaluate_decision, evaluate_timeout};
pub use receipt::{compute_receipt_digest, verify_receipt};
pub(crate) use replay::reconstruct_open_gates;
pub use replay::{
    GateReplay, gate_decision_event, gate_opened_event, replay_gate_history, replay_gate_outcome,
};
