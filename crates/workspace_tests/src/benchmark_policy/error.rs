use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub(crate) enum PolicyError {
    Message(String),
    Usage(String),
}

pub(crate) type Result<T> = std::result::Result<T, PolicyError>;

macro_rules! bail {
    ($($arg:tt)*) => {{
        return Err($crate::benchmark_policy::error::PolicyError::Message(format!($($arg)*)));
    }};
}
pub(crate) use bail;

pub(crate) trait Context<T> {
    fn context(self, msg: &str) -> Result<T>;
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E: Display> Context<T> for std::result::Result<T, E> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| PolicyError::Message(format!("{msg}: {e}")))
    }
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| PolicyError::Message(format!("{}: {e}", f())))
    }
}

impl<T> Context<T> for Option<T> {
    fn context(self, msg: &str) -> Result<T> {
        self.ok_or_else(|| PolicyError::Message(msg.to_string()))
    }
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| PolicyError::Message(f()))
    }
}

impl From<std::io::Error> for PolicyError {
    fn from(e: std::io::Error) -> Self {
        PolicyError::Message(e.to_string())
    }
}

impl From<serde_json::Error> for PolicyError {
    fn from(e: serde_json::Error) -> Self {
        PolicyError::Message(e.to_string())
    }
}

impl From<serde_yaml::Error> for PolicyError {
    fn from(e: serde_yaml::Error) -> Self {
        PolicyError::Message(e.to_string())
    }
}
