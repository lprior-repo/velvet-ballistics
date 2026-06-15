//! Integration test module tree for vb_proof_kernels.
//!
//! This file defines the module hierarchy for integration tests.
//! Cargo compiles individual .rs files as test binaries, but mod.rs files
//! define modules that those test binaries can import.

mod proptest;
