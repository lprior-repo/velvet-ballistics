#![forbid(unsafe_code)]

//! Reduce replay accumulator/termination parity and cold-reopen recovery.

#[path = "shipping_reduce_recovery/support.rs"]
mod support;

#[path = "shipping_reduce_recovery/activations.rs"]
mod activations;
#[path = "shipping_reduce_recovery/cold_reopen.rs"]
mod cold_reopen;
#[path = "shipping_reduce_recovery/journal_recovery.rs"]
mod journal_recovery;
#[path = "shipping_reduce_recovery/live_parity.rs"]
mod live_parity;
#[path = "shipping_reduce_recovery/typed_refusals.rs"]
mod typed_refusals;
