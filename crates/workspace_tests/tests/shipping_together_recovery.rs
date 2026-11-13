#![forbid(unsafe_code)]
//! Together live, cold-reopen and replay shipping qualification.
//!
//! Scenarios are split into registered submodules so each stays reviewable:
//! branch order and taint, activation/budget behaviour, deterministic replay,
//! journal reopen, and evidence rejection. Nested Together rejection remains
//! enforced by the compiler; nothing here relaxes it.

#[path = "shipping_together_recovery/activations.rs"]
mod activations;
#[path = "shipping_together_recovery/branches.rs"]
mod branches;
#[path = "shipping_together_recovery/evidence.rs"]
mod evidence;
#[path = "shipping_together_recovery/failure.rs"]
mod failure;
#[path = "shipping_together_recovery/recovery.rs"]
mod recovery;
#[path = "shipping_together_recovery/replay.rs"]
mod replay;
#[path = "shipping_together_recovery/support.rs"]
mod support;
