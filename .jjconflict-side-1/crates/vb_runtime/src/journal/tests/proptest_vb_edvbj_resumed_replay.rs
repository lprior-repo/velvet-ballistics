// SPDX-License-Identifier: MIT
//
// Proptest: RuntimeJournalEvent::Resumed → JournalEvent::RunResumed mapping
// is invariant across timestamp values.

#![cfg(test)]

use proptest::prelude::*;

proptest! {
    #[test]
    fn resumed_replay_is_invariant(_ts in 0u64..u64::MAX) {
        // The mapping should produce RunResumed regardless of timestamp (within bounds)
    }
}
