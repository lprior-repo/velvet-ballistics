#![forbid(unsafe_code)]
//! Behavior tests for the expression fuzz harness pipeline.
//!
//! These tests simulate what `fuzz_expression` does:
//! `validate UTF-8 → lex → parse → compile → eval`
//!
//! They verify each pipeline stage produces the correct error variant
//! on invalid input, and that the evaluator produces exact results on
//! valid input. Tests call individual pipeline stages directly so error
//! assertions are precise and diagnostics are clear.

#[cfg(test)]
mod input_boundary;

#[cfg(test)]
mod lexer_error_reachability;

#[cfg(test)]
mod parser_error_reachability;

#[cfg(test)]
mod compiler_error_reachability;

#[cfg(test)]
mod evaluator_error_reachability;

#[cfg(test)]
mod pipeline_integration;

#[cfg(test)]
mod nan_bug;

#[cfg(test)]
mod bound_exhaustion;

#[cfg(test)]
mod unit_edge_variants;
