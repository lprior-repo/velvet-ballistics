//! Typed AST for the workflow definition language.
//!
//! This module provides [`WorkflowSource`] and its supporting types, representing
//! a fully-parsed workflow YAML document. The [`parse_workflow_ast`] function
//! (in [`parse`]) converts raw YAML text into this typed structure after
//! profile validation.
//!
//! # Module Structure
//!
//! - [`types`] - All AST type definitions
//! - [`parse`] - Parsing entry points and helpers
//! - [`parse_steps`] - Step parsing logic
//! - [`tests`] - Comprehensive test suite

pub mod parse;
pub mod parse_steps;
pub mod parse_trigger;
pub mod parse_fields;
pub mod types;
#[cfg(test)] mod tests;

// Re-export all types from the types submodule
pub use types::*;

// Re-export the main parsing function
pub use parse::parse_workflow_ast;
