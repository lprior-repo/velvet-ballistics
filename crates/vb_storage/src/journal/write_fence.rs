#![forbid(unsafe_code)]
//! Test-only owned-child custody and bounded-output support.

#[cfg(test)]
#[path = "write_fence/test_support.rs"]
pub(crate) mod test_support;
