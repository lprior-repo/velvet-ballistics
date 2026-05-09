#![forbid(unsafe_code)]
//! Parsing logic for the workflow AST.
//!
//! This module is organized into sub-modules by concern:
//! - [`workflow`] – top-level document parsing
//! - [`trigger`]  – trigger declarations
//! - [`fields`]   – inputs, vars, secrets parsing
//! - [`steps`]    – step and primitive parsing
//! - [`metadata`] – retry, error handler, result, examples

pub mod fields;
pub mod metadata;
pub mod steps;
pub mod trigger;
pub mod workflow;

pub use workflow::parse_workflow_ast;
