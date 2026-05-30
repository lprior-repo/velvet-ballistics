#![forbid(unsafe_code)]
//! Restriction validation for compile-time variable scope rules.
//!
//! This module enforces that certain variables (like `$attempt.number`)
//! are only accessible in specific syntactic contexts.

#[cfg(test)]
mod tests {
    mod attempt_number_tests;
}
