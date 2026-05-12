#![forbid(unsafe_code)]
//! Multi-shard runtime routing commands to correct shards.

include!("runtime/chunk_003.rs");
include!("runtime/chunk_001.rs");
include!("runtime/chunk_002.rs");

#[cfg(test)]
mod tests {
    include!("runtime/tests/chunk_001.rs");
    include!("runtime/tests/chunk_002.rs");
    include!("runtime/tests/chunk_003.rs");
    include!("runtime/tests/chunk_004.rs");
    include!("runtime/tests/chunk_005.rs");
    include!("runtime/tests/chunk_006.rs");
    include!("runtime/tests/chunk_007.rs");
}
