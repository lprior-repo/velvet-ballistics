//! Proof harnesses for F64 bytecode semantics (Kani verification).
//!
//! These are compiled only when building with Kani (`cargo kani`).
//! They discharge PO-001 and PO-002 from the proof-obligations.planned.jsonl.
#![forbid(unsafe_code)]

#[cfg(kani)]
mod f64_ops;

#[cfg(kani)]
mod f64_div;
