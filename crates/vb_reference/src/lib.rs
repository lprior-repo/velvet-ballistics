//! Reference Models for velvet-ballistics.
//!
//! These are slow, obvious, allocation-friendly reference implementations.
//! They serve as the golden reference for differential testing.

pub mod taint_model;
pub mod step_state_model;
pub mod resource_model;
pub mod engine_model;
pub mod replay_model;
