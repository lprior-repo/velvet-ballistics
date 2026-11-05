#![forbid(unsafe_code)]
//! Shard construction, queue operations, and core tick processing.

include!("impl_parts/chunk_004.rs");
include!("impl_parts/chunk_001.rs");
include!("impl_parts/chunk_002.rs");
include!("impl_parts/chunk_003.rs");
include!("impl_parts/policy_decision.rs");

#[cfg(test)]
mod tests {
    include!("impl_tests/chunk_001.rs");
    include!("impl_tests/chunk_002.rs");
}

#[cfg(all(test, not(kani)))]
#[path = "impl_tests/single_event_sequence_preflight.rs"]
mod single_event_sequence_preflight;
