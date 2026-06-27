#![forbid(unsafe_code)]
//! Workflow field AST data.

use super::{AuthorEntry, AuthorValue};

/// An input field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputField {
    /// Field name.
    pub key: String,
    pub value: AuthorValue,
}

/// A variable field declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarField {
    /// Variable name.
    pub key: String,
    pub value: AuthorValue,
}

/// A secret reference declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretField {
    /// Secret name.
    pub key: String,
    pub value: String,
}

/// Result mapping at the end of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultMapping {
    /// Result expression.
    pub fields: Vec<AuthorEntry<AuthorValue>>,
}

/// An inline example / test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleAst {
    /// Example description.
    pub description: Option<String>,
    /// Input bindings for the example.
    pub input: Option<AuthorValue>,
    /// Expected result expression.
    pub expected: Option<AuthorValue>,
}
