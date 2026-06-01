#![forbid(unsafe_code)]
//! Type taint validation module.
//!
//! This module validates workflow ASTs for type correctness and prevents
//! secret taint from leaking into public results.

mod expressions;
mod facts;
mod steps;
mod types;
mod validation;

pub(crate) use validation::validate_workflow_ast;
