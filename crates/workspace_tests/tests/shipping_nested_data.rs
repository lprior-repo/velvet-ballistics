#![forbid(unsafe_code)]
//! Owned shipping qualification for nested primitive data flows.
//!
//! Bead: vb-eirmk.4.7.2. Outer `ForEach`, `Collect`, `Reduce` crossed with the six
//! existing inner families. Every legal cell compiles the frozen shared source
//! fixture, admits a real owned submission onto a Strict Fjall journal, drives
//! the public runtime to a nonterminal cold cut after inner progress, drops and
//! reopens the journal, recovers through the public runtime, replays the owned
//! arena, and asserts exact terminal values, taints, ordered materialization,
//! and journal evidence. No comparison between two self-produced handles and no
//! success inferred from an unselected replay path.

#[allow(dead_code)]
#[path = "vb_f2_nested_matrix/source.rs"]
mod source;

#[path = "shipping_nested_data/cells.rs"]
mod cells;
#[path = "shipping_nested_data/flow.rs"]
mod flow;
#[path = "shipping_nested_data/support.rs"]
mod support;
