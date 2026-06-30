#![forbid(unsafe_code)]
//! Taint validation tests for vb_compile.
//!
//! Tests the compile pipeline's handling of secret taint in Finish results
//! per Section 47 contract.

#[cfg(test)]
#[path = "tests/secret_finish_tests.rs"]
mod tests;
