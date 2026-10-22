#![forbid(unsafe_code)]
//! Registered local execution. Serialized reports are audit data, never checked authority.
mod authority;
mod catalog;
mod conformance;
mod error;
mod packed;
mod receipt;
mod register;

pub use authority::RegisterGrant;
pub use catalog::ExecutorCatalog;
pub use conformance::{ConformanceReport, ExecutedConformance};
pub use error::ExecutorError;
pub use receipt::RegisterReceipt;
pub use register::FjallRegister;

#[cfg(test)]
mod issuance_test_support;

#[cfg(test)]
mod issuance_tests;

#[cfg(test)]
mod issuance_failure_tests;
