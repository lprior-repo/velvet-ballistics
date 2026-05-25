//! Internal test modules for vb_compile.
//!
//! Declared with `#[cfg(test)] mod tests;` in lib.rs.
//! Each sub-module contains focused test suites.

pub(crate) mod error_variant_tests;

#[cfg(test)]
mod foreach_digest_tests;

pub(crate) mod wait_digest_unit_tests;
