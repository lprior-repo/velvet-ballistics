#![forbid(unsafe_code)]
//! Runtime-local journal append port.

include!("journal/chunk_001.rs");
include!("journal/chunk_002.rs");
include!("journal/chunk_003.rs");

#[cfg(test)]
mod tests {
    include!("journal/tests/chunk_001.rs");
    include!("journal/tests/chunk_002.rs");
    include!("journal/tests/chunk_003.rs");
}
