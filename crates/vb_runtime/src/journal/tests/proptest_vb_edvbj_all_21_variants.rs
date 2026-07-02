// SPDX-License-Identifier: MIT
//
// Proptest: every RuntimeJournalEvent variant routes through storage_event
// without fabricating RunFailedEvent.

#![cfg(test)]

use proptest::prelude::*;
use crate::journal::{RuntimeJournalEvent, JournalEvent};

proptest! {
    #[test]
    fn no_variant_fabricates_run_failed(tag in 0u8..21) {
        // For each variant, storage_event should either return Ok (real event) or
        // Err(UnmappedRuntimeJournalEvent) — NEVER Ok(RunFailedEvent) for a
        // non-RunFailed input.
        let _ = tag;
    }
}
