#![forbid(unsafe_code)]
//! F1 bounded slice: canonical scalar Set + Do action round-trip.
//! Source-to-runtime regression oracles for vb-8eocj.2.
//!
//! Tests are organized into modules:
//! - `positive`: Successful scalar Set -> Finish and Do action round-trips
//! - `negative`: Compile-time and type error cases
//! - `do_action`: Do action suspension, completion, and value verification

mod vb_f1_scalar_do_support;

mod do_action;
mod negative;
mod positive;
