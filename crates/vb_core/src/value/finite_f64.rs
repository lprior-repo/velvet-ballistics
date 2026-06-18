#![forbid(unsafe_code)]
//! Compact finite floating-point scalar for the runtime value model.
//!
//! # Why a custom newtype (not `ordered-float` / `noisy_float`)
//!
//! Both `ordered-float::NotNan<f64>` and `noisy_float::R64` were evaluated:
//!
//! - `NotNan` only rejects NaN and **allows** +/- infinity, so it cannot replace
//!   this type without an additional manual check -- adding a dependency for no
//!   net benefit.
//! - `R64` (`NoisyFloat<f64, FiniteChecker>`) does reject both NaN and infinity,
//!   but validates via `debug_assert!`, meaning invalid values silently pass in
//!   release builds.  This is incompatible with the project's zero-tolerance
//!   safety policy (`unwrap_used = "deny"`, no panics in production paths).
//! - Both crates pull in `num-traits` and other transitive dependencies the
//!   workspace otherwise avoids.
//!
//! The custom implementation is ~40 lines of straightforward code, validates in
//! both debug **and** release builds, has zero dependencies, and provides exactly
//! the invariant this crate needs: "reject NaN AND infinity at construction."

use crate::errors::{CoreError, CoreResult};
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Finite floating-point scalar accepted by the runtime value model.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FiniteF64(f64);

impl Eq for FiniteF64 {}

impl fmt::Display for FiniteF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FiniteF64 {
    /// Creates a finite floating-point value, rejecting NaN and infinities.
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(CoreError::NonFiniteNumber)
        }
    }

    /// Returns the raw finite floating-point value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    #[cfg(kani)]
    pub(crate) fn _kani_any() -> Self {
        let value: f64 = kani::any();
        kani::assume(value.is_finite());
        Self(value)
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer).and_then(|value| {
            Self::new(value).map_err(|err| serde::de::Error::custom(err.to_string()))
        })
    }
}
