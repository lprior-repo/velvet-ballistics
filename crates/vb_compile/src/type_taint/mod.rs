#![forbid(unsafe_code)]
//! Compile-time type taint analysis for workflow ASTs.
//!
//! This module walks the workflow AST forward over the step sequence, tracking
//! `ValueType` + `Taint` facts in per-slot buckets.  It rejects type-mismatched
//! conditions and slot-aliasing errors at compile time.
//!
//! # Module layout
//!
//! - `types` — `ValueType` enum and `ValueFact` struct with taint merge logic.
//! - `engine` — `Facts` analysis state machine + schema-to-facts builders.
//! - `eval` — Expression-level type-and-taint evaluation (AST & parsed trees).
//! - `step` — Step-level validation: forward data-flow over the step sequence.

mod engine;
mod eval;
mod step;
mod types;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub(crate) use step::validate_workflow_ast;

// Items reachable by tests via `super::` paths through this module.
// Tests import `validate_workflow_ast` from the public entry point.
