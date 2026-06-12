//! Test sub-module for the `enums` module.
//!
//! Re-exports the `retry_safety_tests` module under the path
//! `enums::tests::retry_safety_tests`. The test file is at
//! `enums/tests/retry_safety_tests.rs` per the proof-to-rust-map.md
//! canonical path; the parent `enums/mod.rs` declares
//! `mod tests;` to reach this subdirectory.

#[cfg(test)]
mod retry_safety_tests;
