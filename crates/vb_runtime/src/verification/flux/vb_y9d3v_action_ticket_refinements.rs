//! Flux disposition for ActionTicket generation fence — vb-y9d3v.
//!
//! ## Production Truth
//! `vb_core::action::ActionTicket` has public `u16` fields. Zero-valued
//! `attempt` and `capacity` are VALID and CONSTRUCTIBLE in production.
//! Runtime helper functions (`validate_ticket_attempt`, `normalize_scheduled_ticket`)
//! reject or normalize zero/invalid values at VALIDATION BOUNDARIES, not at
//! type-construction time.
//!
//! `RetryPolicy` in `vb_runtime::engine::RetryPolicy` also has public fields
//! and can be constructed with `max_attempts == 0`. Validation happens at
//! `record_retry_attempt` call sites.
//!
//! ## Why Previous Annotations Were False
//! - `#[invariant(self.attempt > 0)]` on ActionTicket — PRODUCTION allows attempt==0
//! - `#[invariant(self.capacity > 0)]` on ActionTicket — PRODUCTION allows capacity==0
//! - `#[invariant(self.max_attempts > 0)]` on RetryPolicy — PRODUCTION allows max==0
//! - `record_retry_attempt_refined` returned `Result<u16, RuntimeError>` — production
//!   returns `Result<bool, RuntimeError>` and mutates `&mut RunState`
//!
//! ## Flux Lane Disposition: WAIVED
//! All Flux obligations (PO-0003, PO-0007, PO-0011, PO-0015, PO-0019,
//! PO-0023, PO-0027, PO-0031, PO-0035, PO-0039) are WAIVED for this bead.
//!
//! Compensating evidence comes from:
//! - Kani: `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs`
//! - proptest: `crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs`
//! - behavior tests: `crates/vb_runtime/src/shard/helpers/tests.rs`
//!
//! ## Future Flux Path (when production APIs support it)
//! If `ActionTicket` is refactored to have a private constructor with invariants,
//! or if small pure validation helpers are extracted (e.g., `classify_ticket_attempt`
//! with scalar inputs), Flux could refine those boundaries.
//!
//! Until then, Flux annotations here would be false and must not be claimed as proof.

#![forbid(unsafe_code)]
