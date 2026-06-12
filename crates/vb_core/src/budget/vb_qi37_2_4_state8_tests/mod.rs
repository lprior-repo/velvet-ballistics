//! Chunked test modules for the `vb_qi37_2_4_state8_tests.rs` tests.
//!
//! This directory holds the test functions that were split out of
//! `vb_qi37_2_4_state8_tests.rs` (Kani state-8 budget tests) to satisfy the 300-line source-length
//! cap. Each `chunk_NNN.rs` is a self-contained test module; together
//! they re-create the original file's test set exactly (semantic
//! content preserved; only file structure changed).
#![forbid(unsafe_code)]
mod chunk_001;
mod chunk_002;
mod chunk_003;
mod chunk_004;
mod chunk_005;
