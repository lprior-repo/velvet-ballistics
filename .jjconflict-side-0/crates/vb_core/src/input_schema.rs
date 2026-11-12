#![forbid(unsafe_code)]
//! Owned, cold input schema payloads independent of YAML and value arenas.

use crate::workflow::{CompiledInputConstraint, InputSlotKind};
use serde::{Deserialize, Serialize};

/// Cold schema/value traversal bound, matching owned input graph depth.
pub const MAX_INPUT_SCHEMA_DEPTH: usize = 64;

/// One named object field and its complete schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledInputField {
    /// Source field name, resolved against owned object key payloads.
    pub name: Box<str>,
    /// Field value kind, independent of any list element kind.
    pub kind: InputSlotKind,
    /// Retained flags, bounds, defaults, and nested fields.
    pub constraint: CompiledInputConstraint,
}

/// Portable default tree; arena handles are never retained as cold payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum InputDefaultValue {
    /// Explicit null.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed integer literal.
    I64(i64),
    /// UTF-8 text.
    Text(Box<str>),
    /// Ordered list items.
    List(Box<[InputDefaultValue]>),
    /// Object fields in source order.
    Object(Vec<(Box<str>, InputDefaultValue)>),
}

mod decode;
mod materialize;
pub use materialize::materialize_input_default;
mod defaults;
pub(crate) use defaults::apply_root_schema_defaults;
pub use defaults::{apply_input_schema_defaults, default_value};
