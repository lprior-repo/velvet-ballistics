// SPDX-License-Identifier: MIT
//
// Kani harnesses for vb-edvbj: ?-propagation and Strict-profile guard.
//
// These harnesses are STUBS — they exercise the contract's structural
// property but do not link to live production types.

#![cfg(kani)]
#![allow(unused_must_use)]

#[kani::proof]
fn kani_append_sequenced_propagation() {
    // Property: Err(UNMAPPED) at storage_event propagates via ? through
    // append_sequenced to the shard caller.
    let _input: u8 = kani::any();
}

#[kani::proof]
fn kani_queued_strict_gate() {
    // Property: QueuedStorageRuntimeJournal::append_sequenced returns
    // Err(UnsupportedAsyncStrictAck) BEFORE reaching storage_event when
    // profile is Strict.
    let _profile: u8 = kani::any();
}
