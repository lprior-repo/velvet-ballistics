//! Verification artifacts for vb_runtime.
//!
//! Kani harnesses (cfg(kani)) and proptest properties (cfg(test)).

#[cfg(kani)]
pub(crate) mod kani;

#[cfg(test)]
pub(crate) mod proptest;
