use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("unsupported generated Rust IR feature: {feature}")]
    UnsupportedIr {
        feature: &'static str,
    },
    #[error("codegen output exceeds buffer capacity")]
    FormatBufferOverflow,
    #[error("rustfmt failed: {detail}")]
    RustfmtFailed {
        detail: String,
    },
    #[error("compile check failed: {detail}")]
    CompileCheckFailed {
        detail: String,
    },
    #[error("semantic equivalence violation: {detail}")]
    SemanticMismatch {
        detail: String,
    },
    #[error("codegen IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("trybuild fixture error: {detail}")]
    TrybuildFixture {
        detail: String,
    },
}

pub type CodegenResult<T> = Result<T, CodegenError>;
