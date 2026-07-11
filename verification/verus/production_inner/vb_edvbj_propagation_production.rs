// SPDX-License-Identifier: MIT
//
// Drift-detection mirror for vb-edvbj propagation chain.
// DRIFT POLICY: `crates/vb_runtime/src/journal/chunk_002.rs:408-412`

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use vstd::prelude::*;

verus! {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorRunId { pub value: u64 }
impl MirrorRunId { pub const fn new(value: u64) -> Self { Self { value } } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorEventSeq { pub value: u64 }
impl MirrorEventSeq { pub const fn new(value: u64) -> Self { Self { value } } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorJournalEvent { Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorRuntimeError { Other }

pub type MirrorRuntimeResult<T> = Result<T, MirrorRuntimeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorDurabilityProfile { Strict, Async }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictProfileGuardResult { Ok(()), ErrStrict }

#[verifier::external]
pub fn mirror_append_sequenced_body(
    storage_event_result: MirrorRuntimeResult<MirrorJournalEvent>,
) -> MirrorRuntimeResult<()> {
    match storage_event_result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

#[verifier::external]
pub fn mirror_queued_strict_append_sequenced(
    profile: MirrorDurabilityProfile,
    _storage_event_result: MirrorRuntimeResult<MirrorJournalEvent>,
) -> StrictProfileGuardResult {
    match profile {
        MirrorDurabilityProfile::Strict => StrictProfileGuardResult::ErrStrict,
        MirrorDurabilityProfile::Async => StrictProfileGuardResult::Ok(()),
    }
}



} // verus!

pub fn is_err_strict(r: StrictProfileGuardResult) -> bool {
    matches!(r, StrictProfileGuardResult::ErrStrict)
}
