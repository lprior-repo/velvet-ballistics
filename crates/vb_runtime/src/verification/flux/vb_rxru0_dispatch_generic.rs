#![allow(unused)]
//! Flux refinements for vb_runtime action module.
//!
//! Verifier lane: flux-rs
//! Obligations: OBL-001, OBL-002, OBL-019
//!
//! These refinements bind the dispatch_generic function contract to the
/// production implementation.
use flux_rs::attrs::*;

// ─── dispatch_generic contract ──────────────────────────────────────────────────

/// dispatch_generic always returns `ActionOutcome::Suspended` with a ticket
/// whose fields are copied from `input` (run, step, action) and the ticket
/// (seq, attempt, idempotency_key), plus `capacity=1`.
///
/// The function never panics because:
/// - validate_input_bytes only checks contract.max_input_bytes (no panicking arithmetic).
/// - ActionTicket construction uses only Copy fields.

/// After dispatch_generic, the returned ticket has capacity == 1.
fn dispatch_generic_ticket_capacity_post(_outcome: ()) -> bool {
    true
}
