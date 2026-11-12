#![forbid(unsafe_code)]
//! Typed AST type definitions for the workflow definition language.
//!
//! All AST types are pure data structures with no parsing logic.

mod fields;
mod step;
mod workflow;

pub use fields::{ExampleAst, InputField, ResultMapping, SecretField, VarField};
pub use step::{
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, RetryPolicy, ScalarValue, StepAst,
    StepPrimitive, TogetherBranch, TriggerAst,
};
pub use workflow::WorkflowSource;

pub use workflow::WorkflowSourceParts;
