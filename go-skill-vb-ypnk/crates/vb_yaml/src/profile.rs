#![forbid(unsafe_code)]

//! Strict YAML profile enforcement.
//!
//! This module implements the "strict profile" that rejects YAML features
//! incompatible with the velvet-ballastics workflow definition language.

#[path = "profile_dupkeys.rs"]
mod profile_dupkeys;
#[path = "profile_validation.rs"]
mod profile_validation;

pub use profile_dupkeys::*;
pub use profile_validation::*;

#[cfg(test)]
#[path = "profile_error_variants_tests.rs"]
mod profile_error_variants_tests;
#[cfg(test)]
#[path = "profile_tests.rs"]
mod profile_tests;
#[cfg(test)]
#[path = "profile_tests_adversarial.rs"]
mod profile_tests_adversarial;
