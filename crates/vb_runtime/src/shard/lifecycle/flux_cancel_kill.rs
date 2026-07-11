#![forbid(unsafe_code)]
//! Retired Flux-rs sketch for cancel/kill lifecycle invariants.
//!
//! This file is intentionally not included by `shard/lifecycle.rs` and carries
//! no executable tests. It is retained only to prevent stale proof notes from
//! asserting the old idempotent cancel/kill contract.
//!
//! Current production contract for vb-4969v:
//! - `handle_cancel` and `handle_kill` return `RunNotFound` for runs that were
//!   never admitted and are not retained as terminal identities.
//! - For an active run with an in-flight action, `ActionAbandoned` and the
//!   terminal marker (`RunCancelled` or `RunKilled`) are appended as one same-run
//!   journal batch before pending-action, timer, frame, runtime-state, terminal,
//!   counter, or trace mutation.
//! - If that durable append fails, pending action ownership, runtime state, run
//!   state, checked-out ownership, counters, trace, timers, and journal sequence
//!   remain unchanged so the terminal command can be retried.
//! - Successful terminalization may remove pending action/timer boundaries only
//!   after the durable terminal batch has been accepted.
