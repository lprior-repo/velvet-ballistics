#![forbid(unsafe_code)]

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    InvalidToken(String),
    NavItemNotFound(String),
    InvalidScreenTransition(String),
    TokenParseError(String),
    InvalidFlowDocument(String),
    LayoutNotComputed,
    NodeNotFound(usize),
    InvalidViewport,
    AnimationOverflow,
    ViewHidden,
    MissingDesignToken(String),
}
