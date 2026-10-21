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

#[cfg(any(test, feature = "test-util"))]
pub use workflow::WorkflowSourceParts;

#[cfg(not(any(test, feature = "test-util")))]
pub(crate) use workflow::WorkflowSourceParts;
