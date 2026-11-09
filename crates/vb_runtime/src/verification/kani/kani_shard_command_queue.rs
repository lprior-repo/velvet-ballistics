#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for the shard command queue boundary.

use crate::shard::types::{MAX_COMMAND_QUEUE_CAPACITY, is_valid_command_queue_capacity};

#[kani::proof]
fn command_queue_bounds() {
    let capacity: usize = kani::any();
    let valid = is_valid_command_queue_capacity(capacity);

    if valid {
        kani::assert(capacity > 0, "valid capacity is non-zero");
        kani::assert(
            capacity <= MAX_COMMAND_QUEUE_CAPACITY,
            "valid capacity stays within max bound",
        );
        return;
    }

    kani::assert(
        capacity == 0 || capacity > MAX_COMMAND_QUEUE_CAPACITY,
        "invalid capacity is exactly outside the queue domain",
    );
}
