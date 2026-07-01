// SPDX-License-Identifier: MIT
//
// Extern companion for vb_edvbj_propagation.rs
// Production binding for chunk_002.rs:342-346 and chunk_003.rs:8-16.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

#[path = "production_inner/vb_edvbj_propagation_production.rs"]
pub mod production;

pub use production::{
    MirrorDurabilityProfile, MirrorRuntimeError, MirrorRuntimeResult, MirrorEventSeq,
    MirrorJournalEvent, MirrorRunId, StrictProfileGuardResult,
    is_err_strict,
    mirror_append_sequenced_body, mirror_queued_strict_append_sequenced,
};

pub fn prod_methods_drift_check_propagation() {}
