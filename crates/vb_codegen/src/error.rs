#![forbid(unsafe_code)]

/// Code generation failures for the bounded generated-workflow subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CodegenError {
    /// Compatibility marker for deferred optional backends.
    #[error("codegen backend deferred")]
    Deferred,
    /// The compiled IR uses a construct outside the generated subset.
    #[error("unsupported generated IR: {feature}")]
    UnsupportedIr { feature: &'static str },
}
