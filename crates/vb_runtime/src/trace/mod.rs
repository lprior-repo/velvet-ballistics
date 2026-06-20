#![forbid(unsafe_code)]
//! Bounded trace ring using rtrb SPSC ring buffer.
//!
//! # Module layout
//!
//! - event: `TraceEvent` domain enum and its accessor methods.
//! - [`ring`]: `TraceRing` bounded ring buffer.
//! - kani: Kani verification models (gated behind `cfg(kani)`).
//! - tests: End-to-end behavioral tests.

mod event;
#[cfg(kani)]
mod kani;
pub mod ring;

#[cfg(test)]
mod tests;

pub use event::TraceEvent;
pub use ring::TraceRing;

// Re-export commonly-used IDs so callers don't need to reach into vb_core.
pub use vb_core::ids::{RunId, SlotIdx, StepIdx};

#[cfg(all(kani, feature = "kani-trace-ring"))]
pub(crate) use kani::KaniTraceEventKind;
