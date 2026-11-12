//! Shard partition math module.
//!
//! **TRUST BOUNDARY**: The types in this module are verification models.
//! They will be promoted to production types in State 6/7 (implementation).
//! The Kani harnesses (`#[cfg(kani)]`) and proptest strategies (`#[cfg(test)]`)
//! are verification-only and will be relocated when types are promoted.

pub mod partition;
