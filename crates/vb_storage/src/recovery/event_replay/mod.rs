#![forbid(unsafe_code)]
//! Event replay and tail application for RunFrame hydration.
//!
//! Provides:
//! - `apply_tail_events`: applies journal events to a mutable RunFrame
//! - `compute_parallel_in_flight`: computes peak parallel in-flight from events
//! - `SlotTaintReadObservation` / `SlotTaintResolution` / `resolve_slot_taint_read`
//!
//! These are the core replay primitives: applying deterministic state
//! transitions from journal events onto a live RunFrame.

mod parallel;
mod tail;
mod taint;

pub(crate) use parallel::compute_parallel_in_flight;
pub(crate) use tail::apply_tail_events;
