#![forbid(unsafe_code)]
//! Typed AST for the workflow definition language.
//!
//! This module provides [`WorkflowSource`] and its supporting types, representing
//! a fully-parsed workflow YAML document. The internal parser converts raw YAML
//! text into this typed structure after profile validation.
//!
//! # Module Structure
//!
//! - [`types`] - All AST type definitions
//! - `parse` - Parsing entry points and helpers
//! - `parse_steps` - Step parsing logic
//! - `tests` - Comprehensive test suite

pub(crate) mod parse;
pub(crate) mod parse_fields;
pub(crate) mod parse_steps;
pub(crate) mod parse_trigger;
// Legacy parser tests targeted the removed pre-canonical AST shape.
pub mod types;

// Re-export all types from the types submodule
pub use types::*;

// Re-export the main parsing function
pub(crate) use parse::parse_workflow_ast;
