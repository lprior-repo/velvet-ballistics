//! Chunked test modules for the `tests_and_verification.rs` tests.
//!
//! This directory holds the test functions that were split out of
//! `tests_and_verification.rs` (tests-and-verification) to satisfy the 300-line source-length
//! cap. Each `chunk_NNN.rs` is a self-contained test module; together
//! they re-create the original file's test set exactly (semantic
//! content preserved; only file structure changed).
#![forbid(unsafe_code)]
mod chunk_001;
