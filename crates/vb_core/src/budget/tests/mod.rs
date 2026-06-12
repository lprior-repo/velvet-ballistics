//! Chunked test modules for the `tests.rs` tests.
//!
//! This directory holds the test functions that were split out of
//! `tests.rs` (budget unit tests) to satisfy the 300-line source-length
//! cap. Each `chunk_NNN.rs` is a self-contained test module; together
//! they re-create the original file's test set exactly (semantic
//! content preserved; only file structure changed).
//!
//! The `prelude` module provides shared helper functions
//! (`test_contract`, `test_budget`, `test_policy`, `ensure_equal`,
//! `single_node_workflow`, etc.) that were originally defined inline
//! in the 7339-line `tests.rs` file.
#![forbid(unsafe_code)]
mod prelude;
mod chunk_001;
mod chunk_002;
mod chunk_003;
mod chunk_004;
mod chunk_005;
mod chunk_006;
mod chunk_007;
mod chunk_008;
mod chunk_009;
mod chunk_010;
mod chunk_011;
mod chunk_012;
mod chunk_013;
mod chunk_014;
mod chunk_015;
mod chunk_016;
mod chunk_017;
mod chunk_018;
mod chunk_019;
mod chunk_020;
mod chunk_021;
mod chunk_022;
mod chunk_023;
mod chunk_024;
mod chunk_025;
mod chunk_026;
mod chunk_027;
mod chunk_028;
mod chunk_029;
