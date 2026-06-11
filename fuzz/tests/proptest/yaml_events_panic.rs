#![forbid(unsafe_code)]
//! Proptest suites for YAML event parsing (vb-jpq7.34).
//!
//! Obligations covered:
//! - PO-PROP-001 through PO-PROP-005
//!
//! All strategies generate from the actual type space using proptest
//! combinators — no hardcoded dummy data (GOD RULE 1 compliance).

#[path = "yaml_events_panic/proptests.rs"]
mod proptests;
#[path = "yaml_events_panic/strategies.rs"]
mod strategies;
