#![forbid(unsafe_code)]
//! Runtime slot value model — module re-exports.
//!
//! # Public types
//!
//! - [`Taint`], [`join_taint`] — Secret-propagation marker lattice.
//! - [`FiniteF64`] — Finite-float newtype (rejects NaN / infinity at construction).
//! - [`SlotValue`] — Runtime slot value enum.
//! - [`ConstValue`] — Compile-time constant value enum.
//! - [`SlotValueDisplay`] — Lazily-formatted display for `SlotValue`.

mod taint;
mod finite_f64;
mod slot;
mod constant;
mod display;

#[cfg(test)]
mod proptests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use self::taint::{Taint, join_taint};
pub use self::finite_f64::FiniteF64;
pub use self::slot::SlotValue;
pub use self::constant::ConstValue;
pub use self::display::SlotValueDisplay;
