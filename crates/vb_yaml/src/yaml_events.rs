#![forbid(unsafe_code)]

//! Typed YAML event stream wrapping saphyr-parser events.
//!
//! This module provides a [`YamlEvent`] enum that mirrors the saphyr-parser
//! event types but owns all data and carries source location information.

#[path = "yaml_events_conv.rs"]
mod events_conv;
#[path = "yaml_events_types.rs"]
mod events_types;

pub use events_conv::*;
pub use events_types::*;

#[cfg(test)]
#[path = "yaml_events_tests.rs"]
mod events_tests;
