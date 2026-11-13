//! Owned shipping qualification for nested outer Repeat/Together/Choose.
//!
//! Bead: vb-eirmk.4.7.4. Every legal outer/inner cell compiles real YAML,
//! validates full IR, admits through the owned submission API on a strict
//! Fjall journal, cuts live after nested progress, then proves drop/reopen/
//! recover/resume/replay parity with exact values, taints and order.

#![forbid(unsafe_code)]

#[path = "shipping_nested_control/cells.rs"]
mod cells;
#[path = "shipping_nested_control/matrix.rs"]
mod matrix;
#[path = "vb_f2_nested_matrix/source.rs"]
mod source;
#[path = "shipping_nested_control/support.rs"]
mod support;
