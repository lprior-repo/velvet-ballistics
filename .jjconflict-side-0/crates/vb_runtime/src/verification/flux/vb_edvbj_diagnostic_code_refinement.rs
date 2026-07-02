// SPDX-License-Identifier: MIT
//
// Flux refinement: RuntimeError::UnmappedRuntimeJournalEvent has bounded payload.

#![cfg(flux_enabled)]

use flux_rs::attrs::*;

#[flux_rs::sig(fn () -> i32[0])]
pub const fn unmapped_event_kind_max_len() -> i32 { 0 }

#[flux_rs::sig(fn (RuntimeError::UnmappedRuntimeJournalEvent[@e]) -> i32)]
pub fn unmapped_event_kind_code(_e: RuntimeError) -> i32 { 0 }
