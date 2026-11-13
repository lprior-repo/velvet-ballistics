//! Pure JSON/row validation for vb-jpq7 closure evidence.
//! Separated from filesystem and CLI actions.

mod checks;
pub(crate) mod cli;
mod issues;
mod manifest;
mod markers;
mod raw_log;
mod resolution;
mod row_validation;
mod self_test;
mod validation;

pub(crate) use issues::{Issue, parse_issues};
pub(crate) use manifest::parse_manifest;
pub(crate) use self_test::run_self_test;
pub(crate) use validation::validate_closed_children;

#[cfg(test)]
#[path = "validation_parity_tests.rs"]
mod validation_parity_tests;
