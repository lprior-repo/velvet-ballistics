#![forbid(unsafe_code)]

//! State 8 Kani setup marker for capability enforcement harness routing.
//!
//! The executable harnesses remain in `kani_capability_harnesses.rs`; this
//! module exists because the approved State 8 setup obligation checks for
//! `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs` before
//! State 11 runs `cargo kani --harness capability_name_grants_harness`.
