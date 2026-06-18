#![forbid(unsafe_code)]
//! Schema scope — controls which fields are legal at each nesting depth.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaScope {
    /// Top-level input schemas (e.g. `inputs.foo: { from: ... }`).
    Input,
    /// Nested object field schemas (`inputs.foo.fields.bar: { ... }`).
    ObjectField,
}

impl SchemaScope {
    /// Returns `true` when `from` is a permitted field for this scope.
    pub(crate) const fn allows_from(self) -> bool {
        matches!(self, Self::Input)
    }
}
