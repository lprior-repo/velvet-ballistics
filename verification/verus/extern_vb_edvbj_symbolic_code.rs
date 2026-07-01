// SPDX-License-Identifier: MIT
//
// Extern companion for vb_edvbj_symbolic_code.rs
// Production binding for crates/vb_runtime/src/error/diagnostics.rs:107-198

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

#[path = "production_inner/vb_edvbj_symbolic_code_production.rs"]
pub mod production;

pub use production::{
    MirrorRuntimeError, MirrorSymbolicCode,
    mirror_symbolic_code, mirror_runtime_code,
};

pub fn prod_methods_drift_check_symbolic_code() {}
