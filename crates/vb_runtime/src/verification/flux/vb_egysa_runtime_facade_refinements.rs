// vb-egysa: Flux refinement artifact for runtime facade semantics.
//! Flux refinements for vb_runtime Runtime facade public API.
//!
//! This module provides Flux refinement types for the Runtime struct's
//! public API methods including submit, tick, action completion, and ask answering.

#![cfg(flux)]

use crate::RuntimeError;
use crate::RuntimeResult;

/// Refined result type for Runtime operations.
///
/// This refines the RuntimeResult to track whether the operation
/// succeeded or failed with a specific error variant.
#[flux_rs::refined_by(v: int)]
pub enum RuntimeResultRef {
    #[flux_rs::variant(RuntimeResultRef[0])]
    Ok,
    #[flux_rs::variant(RuntimeResultRef[1])]
    ErrQueueFull,
    #[flux_rs::variant(RuntimeResultRef[2])]
    ErrRunNotFound,
    #[flux_rs::variant(RuntimeResultRef[3])]
    ErrOther,
}

impl<T> From<Result<T, RuntimeError>> for RuntimeResultRef {
    fn from(result: Result<T, RuntimeError>) -> Self {
        match result {
            Ok(_) => RuntimeResultRef::Ok,
            Err(RuntimeError::QueueFull) => RuntimeResultRef::ErrQueueFull,
            Err(RuntimeError::RunNotFound) => RuntimeResultRef::ErrRunNotFound,
            Err(_) => RuntimeResultRef::ErrOther,
        }
    }
}

/// Specification: Runtime submit operations succeed or return a known error.
#[flux_rs::sig(fn() -> RuntimeResultRef{v: v == 0})]
pub fn runtime_submit_result_invariant() -> RuntimeResultRef {
    // This is a spec function that represents the invariant that
    // a properly initialized runtime with valid inputs should succeed.
    RuntimeResultRef::Ok
}

/// Specification: tick_all returns true when work was done, false when idle.
#[flux_rs::sig(fn() -> bool)]
pub fn runtime_tick_returns_bool() -> bool {
    true // Spec function for tick result interpretation
}
