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
//!
//! Active parser tests live in the crate-level test module.

pub(crate) mod parse;
pub(crate) mod parse_fields;
pub(crate) mod parse_steps;
pub(crate) mod parse_trigger;
pub mod types;

// Re-export types from the types submodule (explicit list — no glob,
// so that pub(crate)-restricted items like WorkflowSourceParts are not
// accidentally made public in production builds).
pub use types::{
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, ExampleAst, InputField, ResultMapping,
    RetryPolicy, ScalarValue, SecretField, StepAst, StepPrimitive, TogetherBranch, TriggerAst,
    VarField, WorkflowSource,
};

// WorkflowSourceParts is pub(crate) in production, pub when the test-util
// feature is active (enabled by dependent-crate dev-dependencies).
#[cfg(any(test, feature = "test-util"))]
pub use types::WorkflowSourceParts;

// Re-export the main parsing function
pub(crate) use parse::parse_workflow_ast;
