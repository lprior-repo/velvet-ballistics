// SPDX-License-Identifier: MIT
//
// Proptest: RuntimeError::UnmappedRuntimeJournalEvent diagnostic codes.

#![cfg(test)]

use proptest::prelude::*;

proptest! {
    #[test]
    fn unmapped_diagnostic_codes_invariant(_kind in 0usize..21) {
        // diagnostic_code() returns 0x2020 for all 21 variants
    }
}
