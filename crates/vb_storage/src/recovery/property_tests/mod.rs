#![forbid(unsafe_code)]
//! Property-test module for the `vb_storage::recovery` package.
//!
//! Hosts proptest harnesses that exercise the recovery pipeline against
//! fuzz-malformed journal record envelopes. Each sub-module is wired under
//! `#[cfg(test)]` from the parent `recovery::mod.rs` so the harness compiles
//! only on `cargo test` invocations and never leaks into the production build.

#[cfg(test)]
mod error_recovery;
