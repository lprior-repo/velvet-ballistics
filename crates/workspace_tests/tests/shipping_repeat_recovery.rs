//! Repeat attempts/checks/finish across live drive, durable journal, reopen,
//! recovery classification, cold arena state and artifact-guided replay.
//!
//! Bead: vb-eirmk.8.2. Scenarios assert value, taint, pc, step state, packed
//! counter, recorded action outcomes and exact evidence refusal so a weakened
//! Repeat transition cannot pass on compilation alone.

#![forbid(unsafe_code)]

#[path = "shipping_repeat_recovery/action.rs"]
mod action;
#[path = "shipping_repeat_recovery/ask.rs"]
mod ask;
#[path = "shipping_repeat_recovery/cold.rs"]
mod cold;
#[path = "shipping_repeat_recovery/live.rs"]
mod live;
#[path = "shipping_repeat_recovery/replay.rs"]
mod replay;
#[path = "shipping_repeat_recovery/support.rs"]
mod support;
