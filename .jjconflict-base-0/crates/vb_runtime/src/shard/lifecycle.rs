#![forbid(unsafe_code)]
//! Run lifecycle management: submit, resume, cancel, action completion, timers.

include!("lifecycle/chunk_003.rs");
include!("lifecycle/chunk_001.rs");
include!("lifecycle/chunk_002.rs");

#[cfg(test)]
mod tests {
    include!("lifecycle_tests/chunk_001.rs");
    include!("lifecycle_tests/chunk_002.rs");
    include!("lifecycle_tests/chunk_003.rs");
    include!("lifecycle_tests/chunk_004.rs");
    include!("lifecycle_tests/chunk_005.rs");
    include!("lifecycle_tests/chunk_006.rs");
    include!("lifecycle_tests/chunk_007.rs");
}
