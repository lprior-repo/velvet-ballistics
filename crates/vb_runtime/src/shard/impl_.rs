#![forbid(unsafe_code)]
//! Shard construction, queue operations, and core tick processing.

include!("impl_parts/chunk_004.rs");
include!("impl_parts/dispatch.rs");
include!("impl_parts/chunk_001.rs");
include!("impl_parts/timer_methods.rs");
include!("impl_parts/journal_helpers.rs");
include!("impl_parts/evidence_flush.rs");
include!("impl_parts/chunk_002.rs");
include!("impl_parts/chunk_003.rs");

#[cfg(test)]
mod tests {
    include!("impl_tests/chunk_001.rs");
    include!("impl_tests/chunk_002.rs");
}
