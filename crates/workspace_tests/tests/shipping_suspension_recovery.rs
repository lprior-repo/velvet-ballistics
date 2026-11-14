#![forbid(unsafe_code)]
//! Given/When/Then scenarios for journal-driven suspension, retry and error
//! routing recovery (vb-eirmk.9).
//!
//! Every scenario starts from a real compiled source workflow driven by the
//! production `Runtime` against a real Fjall store. The durable prefix is the
//! journal the run actually wrote, never a synthesized sequence.

#[path = "shipping_suspension_recovery/support.rs"]
mod support;

#[path = "shipping_suspension_recovery/routing.rs"]
mod routing;

#[path = "shipping_suspension_recovery/handlers.rs"]
mod handlers;

#[path = "shipping_suspension_recovery/handler_prefix.rs"]
mod handler_prefix;

#[path = "shipping_suspension_recovery/ask.rs"]
mod ask;

#[path = "shipping_suspension_recovery/refusals.rs"]
mod refusals;
