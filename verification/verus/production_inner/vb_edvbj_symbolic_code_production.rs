// SPDX-License-Identifier: MIT
//
// Drift-detection mirror for vb-edvbj symbolic codes.
// DRIFT POLICY: `crates/vb_runtime/src/error/diagnostics.rs:107-198`

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use vstd::prelude::*;

verus! {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorSymbolicCode {
    InternalInvariant,
    Other,
}

impl MirrorSymbolicCode {
    pub fn internal_invariant() -> Self { Self::InternalInvariant }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorRuntimeError {
    UnmappedRuntimeJournalEvent { event_kind: &'static str },
    Other,
}

impl MirrorRuntimeError {
    pub fn unmapped(event_kind: &'static str) -> Self {
        Self::UnmappedRuntimeJournalEvent { event_kind }
    }
}

#[verifier::external]
pub fn mirror_symbolic_code(_err: &MirrorRuntimeError) -> MirrorSymbolicCode {
    MirrorSymbolicCode::InternalInvariant
}

#[verifier::external]
pub fn mirror_runtime_code(_err: &MirrorRuntimeError) -> Option<&'static str> {
    None
}

} // verus!
