#![allow(unused_imports)]
//! Verus specification and proof for vb_runtime action module — vb-rxru0 (revised).
//!
//! Replaces tautological proofs (PF-R001 rejected) with real mathematical claims.
//!
//! Obligations addressed: OBL-010, OBL-014, OBL-016, OBL-018
//! (binding to dispatch_generic field preservation, issue_action_ticket correctness,
//!  and cross-crate MockMarker derivation).
//!
/// GOD RULE 2: Each spec fn models actual production behavior in
/// `vb_runtime::action::dispatch_generic` and `vb_core::action::issue_action_ticket`.

use vstd::prelude::*;

verus! {

    use vstd::prelude::*;

    // ============================================================================
    // Model: Abstract ActionTicket and ActionOutcome for Verus reasoning
    //
    // These model types represent the same data as the production types
    // in vb_core::action, enabling Verus to reason about the behavior
    // of dispatch_generic and issue_action_ticket without depending on
    // the actual Rust types (which may carry trait impls Verus cannot model).
    // ============================================================================

    /// Abstract ActionTicket matching vb_core::action::ActionTicket fields.
    pub struct AbstractTicket {
        run: u64,
        step: u64,
        seq: u64,
        action: u64,
        attempt: u16,
        idempotency_key: u128,
        capacity: u16,
    }

    impl AbstractTicket {
        pub open spec fn run(&self) -> u64 { self.run }
        pub open spec fn step(&self) -> u64 { self.step }
        pub open spec fn seq(&self) -> u64 { self.seq }
        pub open spec fn action(&self) -> u64 { self.action }
        pub open spec fn attempt(&self) -> u16 { self.attempt }
        pub open spec fn idempotency_key(&self) -> u128 { self.idempotency_key }
        pub open spec fn capacity(&self) -> u16 { self.capacity }
    }
