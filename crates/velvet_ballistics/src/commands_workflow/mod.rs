#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.

mod dot;
mod helpers;
mod simulate;

pub(crate) use dot::{collect_kind_edges, generate_dot};
pub(crate) use helpers::node_kind_label;
pub(crate) use simulate::simulate_workflow;
